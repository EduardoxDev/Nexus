<#
.SYNOPSIS
  Gera os pacotes do NEXUS com a CLI `nexus` ao lado do app (Windows, Linux ou macOS).

.DESCRIPTION
  1. Compila a CLI em release e a copia como sidecar (binaries/nexus-<alvo>[.exe]).
  2. Roda `tauri build` com tauri.release.conf.json, que inclui a CLI no pacote.
     Instalada, ela fica ao lado do app (nexus.exe no Windows; Contents/MacOS/nexus no macOS;
     /usr/bin/nexus no .deb), onde a tela Integrações a encontra.
  3. Confere a CLI com scripts/smoke-cli.mjs (--version e handshake MCP), quando o alvo roda
     nesta máquina.
  Nada é assinado (veja docs/release.md).

  Funciona no Windows PowerShell 5.1 e no PowerShell 7 (pwsh) do Linux e do macOS.

.PARAMETER Target
  Triple do Rust (ex.: x86_64-apple-darwin). Padrão: o da máquina.

.PARAMETER Bundles
  Formatos do tauri, separados por vírgula. Padrão por sistema: nsis | deb,appimage | app,dmg.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/release.ps1
.EXAMPLE
  pwsh scripts/release.ps1 -Target x86_64-apple-darwin
#>
param(
  [string]$Target = "",
  [string]$Bundles = ""
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$desktop = Join-Path $root "apps/desktop"

$hostTriple = ((rustc -vV) | Select-String "^host: ").ToString().Substring(6).Trim()
$triple = if ($Target) { $Target } else { $hostTriple }
$isWin = $triple -like "*windows*"
$isMac = $triple -like "*apple-darwin*"
$exe = if ($isWin) { ".exe" } else { "" }
if (-not $Bundles) {
  $Bundles = if ($isWin) { "nsis" } elseif ($isMac) { "app,dmg" } else { "deb,appimage" }
}
# Com --target, o cargo e o tauri escrevem em target/<triple>/release.
$targetArgs = if ($Target) { @("--target", $Target) } else { @() }
$releaseDir = if ($Target) { Join-Path $root "target/$Target/release" } else { Join-Path $root "target/release" }

function Invoke-Step([string]$what, [scriptblock]$block) {
  # Ferramentas escrevem progresso em stderr; no PowerShell 5.1 isso não pode virar erro.
  $ErrorActionPreference = "Continue"
  & $block 2>&1 | Out-Host
  $code = $LASTEXITCODE
  $ErrorActionPreference = "Stop"
  if ($code -ne 0) { throw "falha: $what (código $code)" }
}

Push-Location $root
try {
  Invoke-Step "compilar a CLI" { cargo build --release --locked -p nexus-cli @targetArgs }
} finally { Pop-Location }

$cli = Join-Path $releaseDir "nexus$exe"
$bin = Join-Path $desktop "src-tauri/binaries"
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item $cli (Join-Path $bin "nexus-$triple$exe") -Force

if ($triple -eq $hostTriple) {
  $version = (Get-Content (Join-Path $desktop "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json).version
  Invoke-Step "fumaça da CLI" { node (Join-Path $root "scripts/smoke-cli.mjs") $cli $version }
} else {
  Write-Host "CLI de outro alvo ($triple): fumaça pulada nesta máquina."
}

Push-Location $desktop
try {
  $runner = if (Get-Command pnpm -ErrorAction SilentlyContinue) { "pnpm" } else { "npx" }
  Invoke-Step "tauri build" {
    & $runner tauri build --config src-tauri/tauri.release.conf.json --bundles $Bundles @targetArgs
  }
} finally { Pop-Location }

$bundleDir = Join-Path $releaseDir "bundle"
$artifacts = Get-ChildItem $bundleDir -Recurse -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Extension -in ".exe", ".deb", ".AppImage", ".dmg", ".gz", ".sig" }
if (-not $artifacts) { throw "nenhum pacote encontrado em $bundleDir" }
Write-Host ""
foreach ($a in $artifacts) {
  $hash = (Get-FileHash $a.FullName -Algorithm SHA256).Hash.ToLower()
  Write-Host ("{0}  {1:N1} MB  {2}" -f $hash, ($a.Length / 1MB), $a.FullName)
}
