$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$executable = Join-Path $repositoryRoot 'target/release/gitzi.exe'
$distributionDirectory = Join-Path $repositoryRoot 'dist'
$packagePath = Join-Path $distributionDirectory 'gitzi-windows-x86_64.zip'
$temporaryPackagePath = Join-Path $distributionDirectory 'gitzi-windows-x86_64.tmp.zip'

$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCommand) {
    throw 'Cargo is required to build Gitzi. Install Rust and ensure cargo is on PATH.'
}

Push-Location $repositoryRoot
try {
    & $cargoCommand.Source build --release --locked
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Release executable not found: $executable"
}

New-Item -ItemType Directory -Path $distributionDirectory -Force | Out-Null
if (Test-Path -LiteralPath $temporaryPackagePath) {
    Remove-Item -LiteralPath $temporaryPackagePath -Force
}

$instructions = @'
Gitzi for Windows x86_64

Extract this archive and run gitzi.exe from a Windows desktop session.
Git 2.23 or later must be installed and available as `git` on PATH.
Gitzi uses your existing Git configuration, credential helpers and SSH agent.
This package does not include Git and does not provide GitHub pull-request/issues APIs.
'@

Add-Type -AssemblyName System.IO.Compression
$archive = $null
try {
    $archive = [System.IO.Compression.ZipFile]::Open(
        $temporaryPackagePath,
        [System.IO.Compression.ZipArchiveMode]::Create
    )
    $fixedTimestamp = [DateTimeOffset]::new(2000, 1, 1, 0, 0, 0, [TimeSpan]::Zero)

    $executableEntry = $archive.CreateEntry('gitzi.exe', [System.IO.Compression.CompressionLevel]::Optimal)
    $executableEntry.LastWriteTime = $fixedTimestamp
    $sourceStream = [System.IO.File]::OpenRead($executable)
    $entryStream = $executableEntry.Open()
    try {
        $sourceStream.CopyTo($entryStream)
    }
    finally {
        $entryStream.Dispose()
        $sourceStream.Dispose()
    }

    $instructionsEntry = $archive.CreateEntry('README.txt', [System.IO.Compression.CompressionLevel]::Optimal)
    $instructionsEntry.LastWriteTime = $fixedTimestamp
    $entryStream = $instructionsEntry.Open()
    $writer = [System.IO.StreamWriter]::new($entryStream, [System.Text.UTF8Encoding]::new($false))
    try {
        $writer.Write($instructions)
    }
    finally {
        $writer.Dispose()
    }
}
finally {
    if ($archive) {
        $archive.Dispose()
    }
}

Move-Item -LiteralPath $temporaryPackagePath -Destination $packagePath -Force
Write-Output "Created $packagePath"
