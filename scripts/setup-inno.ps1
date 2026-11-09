# Install the compiler in a versioned, per-user build-tools directory.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$toolVersion = '7.1.1'
$directory = Join-Path $env:LOCALAPPDATA "SereinBuildTools/InnoSetup-$toolVersion"
$compiler = Join-Path $directory 'ISCC.exe'
if (Test-Path -LiteralPath $compiler) { Write-Output $compiler; return }
$download = Join-Path ([IO.Path]::GetTempPath()) "serein-innosetup-$toolVersion-x64.exe"
$releaseTag = 'is-' + $toolVersion.Replace('.', '_')
$uri = "https://github.com/jrsoftware/issrc/releases/download/$releaseTag/innosetup-$toolVersion-x64.exe"
Invoke-WebRequest -Uri $uri -OutFile $download
$signature = Get-AuthenticodeSignature -FilePath $download
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Pyrsys B\.V\.') {
    throw 'The Inno Setup download did not have the expected valid Pyrsys signature.'
}
$process = Start-Process -FilePath $download -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', "/DIR=`"$directory`"") -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0 -or !(Test-Path -LiteralPath $compiler)) {
    throw "Inno Setup compiler installation failed (exit $($process.ExitCode))."
}
Write-Output $compiler
