# Embedded by the Windows executable. Installs the Core launcher only.
$ErrorActionPreference = 'Stop'
$tag = 'v3.2.2'
$expected = '747b3edce5ea3b94ac7660f7c9a20735f977da7e'
$repository = 'https://github.com/zidanaisar624/MSSHI-Mega-Semar-Sang-Hyang-Ismaya.git'
$stage = $null
$previousTemp = $env:TEMP
$previousTmp = $env:TMP
try {
    Add-Type -AssemblyName System.Windows.Forms
    $homeDir = [IO.Path]::GetFullPath($env:MSSHI_HOME)
    if ($homeDir.StartsWith('\\')) { throw 'Pilih folder tujuan pada drive lokal.' }
    $ask = "Pasang CLI MSSHI Core $tag ke $homeDir dari GitHub? Pemasangan memerlukan Git dan Python 3.10+. Preset serta pengaturan profil diatur terpisah."
    $choice = [System.Windows.Forms.MessageBox]::Show($ask, 'MSSHI Connect', 'YesNo', 'Question', 'Button2')
    if ($choice -ne [System.Windows.Forms.DialogResult]::Yes) { exit 2 }
    $git = (Get-Command git.exe -CommandType Application -ErrorAction Stop).Source
    $driveRoot = [IO.Path]::GetPathRoot($homeDir)
    $drive = [IO.DriveInfo]::new($driveRoot)
    if (-not $drive.IsReady -or $drive.AvailableFreeSpace -lt 512MB) { throw 'Ruang kosong drive tujuan kurang dari 512 MB.' }
    $parent = [IO.Directory]::CreateDirectory((Join-Path $homeDir 'connect-cache')).FullName.TrimEnd('\')
    $stage = Join-Path $parent ('msshi-connect-core-' + [Guid]::NewGuid().ToString('N'))
    $env:TEMP = $parent
    $env:TMP = $parent
    $env:GIT_TERMINAL_PROMPT = '0'
    $env:GIT_CONFIG_NOSYSTEM = '1'
    $env:GIT_CONFIG_GLOBAL = 'NUL'
    & $git -c core.longpaths=true -c protocol.allow=never -c protocol.https.allow=always -c credential.helper= clone '--template=' --depth 1 --single-branch --branch $tag -- $repository $stage
    if ($LASTEXITCODE -ne 0) { throw 'Pengambilan Core resmi dari GitHub gagal.' }
    $actual = & $git -C $stage rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $actual -cne $expected) { throw 'Commit Core berbeda dari rilis yang ditinjau. Pemasangan dibatalkan.' }
    $options = @{ SourcePath=$stage; NoSetup=$true; NoPath=$true; NoPdf=$true; Yes=$true }
    if ($env:MSSHI_HOME) { $options.InstallDir = $env:MSSHI_HOME }
    & (Join-Path $stage 'install.ps1') @options
    if (-not (Test-Path -LiteralPath (Join-Path $homeDir 'bin\msshi.cmd') -PathType Leaf)) { throw 'Launcher belum ditemukan setelah instalasi.' }
    $state = Get-Content -LiteralPath (Join-Path $homeDir 'state.json') -Raw | ConvertFrom-Json
    if ($state.current -cnotmatch '^(?:[0-9a-f]{24}|[0-9a-f]{64})$') { throw 'ID rilis lokal tidak valid setelah pemasangan.' }
    $release = Join-Path (Join-Path $homeDir 'releases') $state.current
    $version = Get-Content -LiteralPath (Join-Path $release 'VERSION.json') -Raw | ConvertFrom-Json
    if ('v' + $version.version -cne $tag) { throw 'Versi lokal tidak sesuai dengan rilis yang dipilih.' }
    if (-not (Test-Path -LiteralPath (Join-Path $release 'scripts\msshi_cli.py') -PathType Leaf)) { throw 'CLI rilis tidak lengkap.' }
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
} finally {
    $env:TEMP = $previousTemp
    $env:TMP = $previousTmp
    if ($stage -and (Test-Path -LiteralPath $stage -PathType Container)) {
        $resolved = (Resolve-Path -LiteralPath $stage).Path
        if ([IO.Path]::GetDirectoryName($resolved) -eq $parent -and [IO.Path]::GetFileName($resolved) -match '^msshi-connect-core-[0-9a-f]{32}$') {
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
}
