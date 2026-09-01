$ErrorActionPreference = 'Stop'

$keyPath = "$env:USERPROFILE\.tauri\void-updater.key"
if (-not (Test-Path $keyPath)) {
    throw "cle privee absente: $keyPath (npx tauri signer generate -w $keyPath)"
}

$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw).Trim()
$passFile = "$env:USERPROFILE\.tauri\void-updater.key.pass"
if (Test-Path $passFile) {
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content $passFile -Raw).Trim()
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}

Push-Location "$PSScriptRoot\..\apps\desktop"
try {
    npm run tauri build
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "=== Artefacts ==="
Get-ChildItem "$PSScriptRoot\..\target\release\bundle\nsis" | ForEach-Object {
    "{0}  {1:N1} Mo" -f $_.Name, ($_.Length / 1MB)
}
