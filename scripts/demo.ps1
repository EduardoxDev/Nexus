<#
.SYNOPSIS
  Demonstração reproduzível do NEXUS pela CLI: autorizar, indexar, buscar, compilar um
  Context Pack, memória, atualidade, símbolos, dependências e uma chamada MCP.

.DESCRIPTION
  Usa uma pasta de dados própria (descartável) e não altera o repositório demonstrado.
  Sem -Repo, usa uma cópia do fixture tests/fixtures/saas-app. Nada sai da máquina.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/demo.ps1
  powershell -ExecutionPolicy Bypass -File scripts/demo.ps1 -Repo C:\code\meu-repo -Query "autenticação"
#>
param(
  [string]$Repo,
  [string]$Query = "validar sessão do usuário",
  [string]$Task = "corrigir a expiração da sessão",
  [string]$WorkDir = (Join-Path $env:TEMP "nexus-demo")
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $root
try {
  $ErrorActionPreference = "Continue"
  cargo build --release --locked -p nexus-cli 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "falha ao compilar a CLI" }
  $ErrorActionPreference = "Stop"
} finally { Pop-Location }
$cli = Join-Path $root "target\release\nexus.exe"

if (Test-Path $WorkDir) { Remove-Item -Recurse -Force $WorkDir }
$data = Join-Path $WorkDir "data"
if (-not $Repo) {
  $Repo = Join-Path $WorkDir "acme"
  Copy-Item -Recurse (Join-Path $root "tests\fixtures\saas-app") $Repo
}

# PowerShell 5.1 envia stdin em ASCII por padrão; a CLI fala UTF-8.
$OutputEncoding = New-Object System.Text.UTF8Encoding $false
[Console]::OutputEncoding = $OutputEncoding

function Step([string]$title, [string[]]$arguments, [string]$stdin = $null) {
  Write-Host ""
  Write-Host "=== $title" -ForegroundColor Cyan
  Write-Host "> nexus $($arguments -join ' ')" -ForegroundColor DarkGray
  # O progresso vai para stderr; não é erro.
  $ErrorActionPreference = "Continue"
  if ($stdin) { $stdin | & $cli --data-dir $data @arguments 2>&1 | Out-Host }
  else { & $cli --data-dir $data @arguments 2>&1 | ForEach-Object { "$_" } | Out-Host }
  if ($LASTEXITCODE -ne 0) { throw "falhou: nexus $($arguments -join ' ')" }
}

Step "1. Autorizar o projeto" @("projects", "add", $Repo)
Step "2. Indexar (incremental)" @("index", $Repo)
Step "3. Reindexar: só o que mudou é reprocessado" @("index", $Repo)
Step "4. Buscar com evidências" @("search", $Query, "--limit", "5")
Step "5. Onde o símbolo é declarado" @("symbol", "validateSession")
Step "6. Visão geral do grafo de dependências" @("deps")
Step "7. Registrar uma decisão confirmada" @(
  "memory", "add", "--kind", "decision", "--confirm",
  "--title", "Sessões expiram em 30 minutos",
  "--body", "A sessão do usuário expira após 30 minutos sem atividade.")
Step "8. Context Pack para uma tarefa" @("context", $Task, "--budget", "4000", "--max-files", "6")
Step "9. Atualidade: documentos, memória e arquivos" @("freshness", "scan")

$init = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"demo","version":"1"}}}'
$call = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_project","arguments":{"query":"' + $Query + '","limit":2}}}'
Step "10. A mesma busca via MCP (stdio)" @("mcp", "serve") ($init + "`n" + $call)

Write-Host ""
Write-Host "Demonstração concluída. Dados em $data (pode apagar)." -ForegroundColor Green
