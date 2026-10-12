<#
.SYNOPSIS
  Benchmarks reproduzíveis do NEXUS: tempo, CPU e pico de memória de cada etapa, medidos no
  processo real da CLI (release) sobre um repositório sintético determinístico.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/bench.ps1 -Files 10000 -Runs 5

  Saída: tabela Markdown no console e bench-results.json na pasta temporária indicada.
#>
param(
  [int]$Files = 10000,
  [int]$Runs = 5,
  [string]$WorkDir = (Join-Path $env:TEMP "nexus-bench")
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $root
try {
  # O cargo escreve progresso em stderr; no PowerShell 5.1 isso não pode virar erro.
  $ErrorActionPreference = "Continue"
  cargo build --release -p nexus-cli 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "falha ao compilar a CLI" }
  cargo build --release -p nexus-core --example synthetic_repo 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "falha ao compilar o gerador" }
  $ErrorActionPreference = "Stop"
} finally { Pop-Location }

$cli = Join-Path $root "target\release\nexus.exe"
$gen = Join-Path $root "target\release\examples\synthetic_repo.exe"
if (Test-Path $WorkDir) { Remove-Item -Recurse -Force $WorkDir }
$repo = Join-Path $WorkDir "repo"
$data = Join-Path $WorkDir "data"
& $gen $repo $Files | Out-Null

# Executa a CLI medindo tempo de parede, CPU (usuário + sistema) e pico do working set
# (amostrado a cada 10 ms enquanto o processo roda).
function Measure-Nexus([string[]]$Arguments, [string]$Stdin = $null) {
  $psi = New-Object System.Diagnostics.ProcessStartInfo $cli
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.RedirectStandardInput = [bool]$Stdin
  $psi.Arguments = ((@("--data-dir", "`"$data`"") + $Arguments) -join " ")
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $p = [System.Diagnostics.Process]::Start($psi)
  if ($Stdin) { $p.StandardInput.Write($Stdin); $p.StandardInput.Close() }
  $out = $p.StandardOutput.ReadToEndAsync()
  $err = $p.StandardError.ReadToEndAsync()
  $peak = 0
  while (-not $p.HasExited) {
    try { $p.Refresh(); if ($p.WorkingSet64 -gt $peak) { $peak = $p.WorkingSet64 } } catch {}
    Start-Sleep -Milliseconds 10
  }
  $p.WaitForExit()
  $sw.Stop()
  if ($p.ExitCode -ne 0) { throw "nexus $($Arguments -join ' ') falhou: $($err.Result)" }
  [pscustomobject]@{
    WallMs = $sw.Elapsed.TotalMilliseconds
    CpuMs  = $p.TotalProcessorTime.TotalMilliseconds
    PeakMiB = [math]::Round($peak / 1MB, 1)
    Out = $out.Result
  }
}

function Median([double[]]$values) {
  $s = $values | Sort-Object
  $s[[int][math]::Floor(($s.Count - 1) / 2)]
}

$results = [ordered]@{}
function Add-Result([string]$name, [object[]]$samples) {
  $results[$name] = [pscustomobject]@{
    Etapa = $name
    Amostras = $samples.Count
    "Parede (ms, mediana)" = [math]::Round((Median ($samples | ForEach-Object WallMs)), 0)
    "CPU (ms, mediana)" = [math]::Round((Median ($samples | ForEach-Object CpuMs)), 0)
    "Pico de memória (MiB, máx)" = ($samples | Measure-Object PeakMiB -Maximum).Maximum
  }
}

Measure-Nexus @("init") | Out-Null
Measure-Nexus @("projects", "add", "`"$repo`"") | Out-Null
Add-Result "Indexação inicial ($Files arquivos)" @(Measure-Nexus @("index", "repo"))
Add-Result "Incremental sem mudanças" @(1..$Runs | ForEach-Object { Measure-Nexus @("index", "repo") })
$one = @()
foreach ($i in 1..$Runs) {
  Add-Content -Path (Join-Path $repo "pkg000\m1.ts") -Value "// alteração $i"
  $one += Measure-Nexus @("index", "repo")
}
Add-Result "Incremental com 1 arquivo alterado" $one
Add-Result "Reindexação completa (--full, conteúdo reaproveitado)" @(Measure-Nexus @("index", "repo", "--full"))
$queries = @("computeTotal42", "soma dos totais", "Store123 add", "helper do módulo")
Add-Result "Busca (processo completo da CLI)" @(foreach ($r in 1..$Runs) { foreach ($q in $queries) { Measure-Nexus @("search", "`"$q`"", "--json") } })
Add-Result "Context Pack (compilar + exportar)" @(1..$Runs | ForEach-Object { Measure-Nexus @("context", "`"alterar computeTotal42 para aceitar descontos`"", "--json") })
Add-Result "Símbolo (nexus symbol)" @(1..$Runs | ForEach-Object { Measure-Nexus @("symbol", "Store77", "--json") })
Add-Result "Dependências de um arquivo (nexus deps)" @(1..$Runs | ForEach-Object { Measure-Nexus @("deps", "pkg001/m150.ts", "--json") })
$mcp = '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"},"name":"search_project","arguments":{"query":"computeTotal42"}}}' + "`n"
$mcpRuns = @(1..$Runs | ForEach-Object { Measure-Nexus @("mcp", "serve") $mcp })
foreach ($m in $mcpRuns) {
  if ($m.Out -notmatch '"isError":false' -or $m.Out -notmatch 'computeTotal42') { throw "sessão MCP sem resultado válido: $($m.Out)" }
}
Add-Result "Sessão MCP com 1 busca" $mcpRuns

$graph = (Measure-Nexus @("deps", "--json")).Out | ConvertFrom-Json
$db = Get-Item (Join-Path $data "nexus.db")
$wal = Get-Item (Join-Path $data "nexus.db-wal") -ErrorAction SilentlyContinue

"`n## Benchmarks — $Files arquivos, $Runs execuções por etapa"
"Máquina: $((Get-CimInstance Win32_Processor | Select-Object -First 1).Name), $([math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)) GiB; $(Get-Date -Format 'yyyy-MM-dd HH:mm')`n"
"| Etapa | Amostras | Parede (ms, mediana) | CPU (ms, mediana) | Pico de memória (MiB) |"
"|---|---|---|---|---|"
foreach ($r in $results.Values) {
  "| $($r.Etapa) | $($r.Amostras) | $($r.'Parede (ms, mediana)') | $($r.'CPU (ms, mediana)') | $($r.'Pico de memória (MiB, máx)') |"
}
"`nGrafo: $($graph.graph.edges) arestas ($($graph.graph.resolved) internas, $($graph.graph.unresolved) não resolvidas), montado em $($graph.graph.durationMs) ms."
"Banco: $([math]::Round($db.Length / 1MB, 1)) MiB; WAL: $([math]::Round(($wal.Length) / 1MB, 1)) MiB."
$results.Values | ConvertTo-Json | Out-File -Encoding utf8 (Join-Path $WorkDir "bench-results.json")
