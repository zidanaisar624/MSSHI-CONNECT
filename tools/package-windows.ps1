[CmdletBinding()]
param([string]$Binary = "target/release/msshi-connect.exe", [string]$OutputDir = "dist")
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$dir = [IO.Directory]::CreateDirectory($OutputDir).FullName
$exe = Join-Path $dir 'MSSHI-Connect-Portable-win-x64.exe'
Copy-Item -LiteralPath $Binary -Destination $exe -Force
$version = & $exe --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch 'v1\.0\.2\b') { throw 'Unexpected Connect executable version' }
$health = & $exe --self-check
if ($LASTEXITCODE -ne 0 -or $health -ne 'OK') { throw 'Embedded interface failed its self-check' }
Add-Type -AssemblyName System.Drawing
$assocIcon = [System.Drawing.Icon]::ExtractAssociatedIcon($exe)
if ($null -eq $assocIcon -or $assocIcon.Width -lt 16) { throw 'Executable is missing its embedded Win32 icon resource' }
$assocIcon.Dispose()
$stage = Join-Path $dir ('package-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($stage)
try {
    Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'MSSHI-Connect.exe')
    foreach ($file in 'README.md','LICENSE','MSSHI_Connect.bat','msshi-connect.cmd') {
        Copy-Item -LiteralPath (Join-Path $repo $file) -Destination $stage
    }
    $assets = [IO.Directory]::CreateDirectory((Join-Path $stage 'assets')).FullName
    foreach ($file in 'msshi-connect-icon-64.png','msshi-connect-banner.jpg') {
        Copy-Item -LiteralPath (Join-Path $repo "assets/$file") -Destination $assets
    }
    $zipName = 'MSSHI-Connect-1.0.2-windows-x64.zip'
    $zip = Join-Path $dir $zipName
    Compress-Archive -Path "$stage/*" -DestinationPath $zip -Force
    $lines = @($exe,$zip) | ForEach-Object {
        (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($_)
    }
    [IO.File]::WriteAllLines((Join-Path $dir 'SHA256SUMS.txt'),[string[]]$lines,[Text.Encoding]::ASCII)
} finally {
    $resolved = (Resolve-Path -LiteralPath $stage).Path
    if ([IO.Path]::GetDirectoryName($resolved) -eq $dir -and [IO.Path]::GetFileName($resolved) -match '^package-[0-9a-f]{32}$') {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
