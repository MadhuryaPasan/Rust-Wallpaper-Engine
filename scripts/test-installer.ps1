# Exercise file installation and uninstall in an isolated folder, without launching
# a wallpaper or modifying the user's Serein settings.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Installer)
$ErrorActionPreference = 'Stop'
$registered = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{D4D6507F-3A66-4DC3-99D0-5942ECBB937A}_is1'
if (Test-Path -LiteralPath $registered) {
    throw 'Serein is already installed for this account. Run the installer smoke test in a clean account or CI runner.'
}
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('serein-installer-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
$destination = Join-Path $testRoot 'app'
$uninstaller = Join-Path $destination 'unins000.exe'
try {
    $process = Start-Process -FilePath $installerPath -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS', "/DIR=`"$destination`"", "/LOG=`"$testRoot/install.log`"") -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "Installer returned $($process.ExitCode). Log: $testRoot/install.log" }
    foreach ($file in @('serein-studio.exe', 'serein-player.exe', 'vcruntime140.dll', 'README.md', 'VERSION.txt', 'docs/07-creator-guide.md', 'unins000.exe')) {
        if (!(Test-Path -LiteralPath (Join-Path $destination $file))) { throw "Installer omitted $file" }
    }
    $version = (Get-Content -LiteralPath "$destination/VERSION.txt" -Raw).Trim()
    foreach ($file in @('serein-studio.exe', 'serein-player.exe')) {
        $info = (Get-Item -LiteralPath (Join-Path $destination $file)).VersionInfo
        if ($info.ProductVersion -ne $version) { throw "$file product version '$($info.ProductVersion)' does not match $version" }
    }
    if (!(Test-Path -LiteralPath $registered)) { throw 'Installer did not register its uninstaller.' }
    Write-Output "Installation and executable versions verified ($version)."
}
finally {
    if (Test-Path -LiteralPath $uninstaller) {
        $process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$testRoot/uninstall.log`"") -Wait -PassThru -WindowStyle Hidden
        if ($process.ExitCode -ne 0) { throw "Uninstaller returned $($process.ExitCode). Logs: $testRoot" }
    }
}
if (Test-Path -LiteralPath "$destination/serein-studio.exe") { throw 'Uninstall left the studio executable behind.' }
if (Test-Path -LiteralPath $registered) { throw 'Uninstall registration was not removed.' }
Write-Output "Uninstall verified. Logs retained at $testRoot"
