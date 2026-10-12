<#
.SYNOPSIS
  Gera o instalador NSIS do NEXUS com a CLI `nexus` ao lado do app.

.DESCRIPTION
  1. Compila a CLI em release e a copia como sidecar (binaries/nexus-<alvo>.exe).
  2. Roda `tauri build` com tauri.release.conf.json, que inclui a CLI no instalador.
     Instalada, ela fica como `nexus.exe` na pasta do app, onde a tela Integrações a encontra.
  O instalador não é assinado (veja docs/release.md).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/release.ps1
#>
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$desktop = Join-Path $root "apps\desktop"
$triple = ((rustc -vV) | Select-String "^host: ").ToString().Substring(6).Trim()

Push-Location $root
try {
  # O cargo escreve progresso em stderr; no PowerShell 5.1 isso não pode virar erro.
  $ErrorActionPreference = "Continue"
  cargo build --release --locked -p nexus-cli 2>&1 | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "falha ao compilar a CLI" }
  $ErrorActionPreference = "Stop"
} finally { Pop-Location }

$bin = Join-Path $desktop "src-tauri\binaries"
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item (Join-Path $root "target\release\nexus.exe") (Join-Path $bin "nexus-$triple.exe") -Force

Push-Location $desktop
try {
  $ErrorActionPreference = "Continue"
  $runner = if (Get-Command pnpm -ErrorAction SilentlyContinue) { "pnpm" } else { "npx" }
  & $runner tauri build --config src-tauri/tauri.release.conf.json 2>&1 | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "falha no tauri build" }
  $ErrorActionPreference = "Stop"
} finally { Pop-Location }

$installer = Get-ChildItem (Join-Path $root "target\release\bundle\nsis\*.exe") -ErrorAction SilentlyContinue |
  Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $installer) { throw "instalador não encontrado em target\release\bundle\nsis" }
$hash = (Get-FileHash $installer.FullName -Algorithm SHA256).Hash.ToLower()
Write-Host ""
Write-Host "Instalador: $($installer.FullName)"
Write-Host ("Tamanho:    {0:N1} MB" -f ($installer.Length / 1MB))
Write-Host "SHA-256:    $hash"
