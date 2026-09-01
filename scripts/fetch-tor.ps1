$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root 'apps/desktop/src-tauri/resources/tor'

if (Test-Path (Join-Path $dest 'tor.exe')) {
    Write-Host "tor.exe deja present dans $dest"
    exit 0
}

New-Item -ItemType Directory -Force -Path $dest | Out-Null

$base = 'https://dist.torproject.org/torbrowser/'
Write-Host "recherche de la derniere version stable sur $base"
$listing = (Invoke-WebRequest -Uri $base -UseBasicParsing).Content
$hrefs = [regex]::Matches($listing, 'href="([^"]+)/"') | ForEach-Object { $_.Groups[1].Value }
$stable = $hrefs | Where-Object { $_ -match '^\d+\.\d+\.\d+$' }
if (-not $stable) { throw "aucune version stable trouvee sur $base" }
$latest = @($stable | Sort-Object -Property { [version]$_ })[-1]
Write-Host "version retenue: $latest"

$bundle = "tor-expert-bundle-windows-x86_64-$latest.tar.gz"
$bundleUrl = "$base$latest/$bundle"

$tmpTgz = Join-Path $env:TEMP $bundle
$extract = Join-Path $env:TEMP 'void-tor-extract'
if (Test-Path $extract) { Remove-Item -Recurse -Force $extract }
New-Item -ItemType Directory -Force -Path $extract | Out-Null

Write-Host "telechargement de $bundle"
Invoke-WebRequest -Uri $bundleUrl -OutFile $tmpTgz -UseBasicParsing
if (-not (Test-Path $tmpTgz)) { throw "echec du telechargement de $bundleUrl" }

Write-Host "extraction"
tar -xzf $tmpTgz -C $extract
if ($LASTEXITCODE -ne 0) { throw "echec de l extraction (tar)" }

$torBin = Get-ChildItem -Path $extract -Recurse -Filter 'tor.exe' | Select-Object -First 1
if (-not $torBin) { throw 'tor.exe introuvable dans l archive' }
Copy-Item $torBin.FullName (Join-Path $dest 'tor.exe') -Force

foreach ($geo in @('geoip', 'geoip6')) {
    $geoFile = Get-ChildItem -Path $extract -Recurse -Filter $geo | Select-Object -First 1
    if ($geoFile) { Copy-Item $geoFile.FullName (Join-Path $dest $geo) -Force }
}

Remove-Item $tmpTgz -Force -ErrorAction SilentlyContinue
Remove-Item $extract -Recurse -Force -ErrorAction SilentlyContinue

& (Join-Path $dest 'tor.exe') --version | Select-Object -First 1
Write-Host "tor installe dans $dest"
