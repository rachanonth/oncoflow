param(
  [string] $Source = 'legacy/AllTable.mdb',
  [string] $Output = 'migration/output/oncoflow.db',
  [string] $JsonReport = 'migration/reports/migration_report.json',
  [string] $MarkdownReport = 'migration/reports/migration_report.md',
  [switch] $Replace
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$projectRoot = Split-Path -Parent $PSScriptRoot
$sourcePath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $Source))
$outputPath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $Output))
$jsonReportPath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $JsonReport))
$markdownReportPath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $MarkdownReport))
$extractor = Join-Path $PSScriptRoot 'scripts/extract_access.ps1'
$manifest = Join-Path $projectRoot 'src-tauri/Cargo.toml'
$binary = Join-Path $projectRoot 'src-tauri/target/debug/mdb-import.exe'

$beforeHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash
$extracted = @(& $extractor -Source $sourcePath)
if ($extracted.Count -eq 0) {
  throw 'ACE extraction returned no tables.'
}

& cargo build --manifest-path $manifest --features migration-cli --bin mdb-import
if ($LASTEXITCODE -ne 0) {
  throw "Rust importer build failed with exit code $LASTEXITCODE."
}

$arguments = @(
  '--source', $sourcePath,
  '--output', $outputPath,
  '--json-report', $jsonReportPath,
  '--markdown-report', $markdownReportPath,
  '--extracted-stdin'
)
if ($Replace) {
  $arguments += '--replace'
}

$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $binary
$startInfo.UseShellExecute = $false
$startInfo.RedirectStandardInput = $true
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
$startInfo.StandardInputEncoding = [System.Text.UTF8Encoding]::new($false)
$startInfo.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
$startInfo.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
foreach ($argument in $arguments) {
  [void] $startInfo.ArgumentList.Add($argument)
}

$process = [System.Diagnostics.Process]::Start($startInfo)
$exitCode = 1
try {
  foreach ($line in $extracted) {
    $process.StandardInput.WriteLine($line)
  }
  $process.StandardInput.Close()
  $stdout = $process.StandardOutput.ReadToEnd()
  $stderr = $process.StandardError.ReadToEnd()
  $process.WaitForExit()
  $exitCode = $process.ExitCode
}
finally {
  $process.Dispose()
  $extracted = $null
}

if ($exitCode -ne 0) {
  throw $stderr.Trim()
}
Write-Output $stdout.Trim()

$afterHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash
if ($beforeHash -ne $afterHash) {
  throw 'The source MDB checksum changed during the read-only import.'
}
