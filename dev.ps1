# Run the contributor Tauri app (from repo root).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Get-Process contributor -ErrorAction SilentlyContinue | Stop-Process -Force

if ($args -contains "-Clean") {
  if (Test-Path ".\target") {
    Write-Host "Removing target/ (fixes stale paths after moving the repo)..."
    Remove-Item -Recurse -Force ".\target"
  }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "Rust/cargo not found. Install from https://rustup.rs"
}

if (-not (cargo tauri --version 2>$null)) {
  Write-Error "Tauri CLI missing. Run: cargo install tauri-cli --version `"^2`""
}

cargo tauri dev
