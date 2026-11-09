[CmdletBinding()]
param([string]$Target)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $arguments = @('build', '--release', '--workspace', '--locked')
    if ($Target) { $arguments += @('--target', $Target) }
    & cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw 'The workspace build failed.' }
    $directory = if ($Target) { "target\$Target\release" } else { 'target\release' }
    Write-Output "Run $directory\serein-studio.exe. Keep serein-player.exe in the same folder."
}
finally { Pop-Location }
