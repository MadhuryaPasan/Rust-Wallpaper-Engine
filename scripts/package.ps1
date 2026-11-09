[CmdletBinding()]
param(
    [string]$Tag,
    [string]$IsccPath
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/version.ps1"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $version = Get-SereinVersion -ExpectedTag $Tag
    if (!$IsccPath) {
        $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
        $candidates = @(
            "$env:LOCALAPPDATA/SereinBuildTools/InnoSetup-7.1.1/ISCC.exe",
            "${env:ProgramFiles}/Inno Setup 7/ISCC.exe",
            "${env:ProgramFiles(x86)}/Inno Setup 7/ISCC.exe",
            "${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe"
        )
        if ($command) { $IsccPath = $command.Source }
        else { $IsccPath = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1 }
    }
    if (!$IsccPath -or !(Test-Path -LiteralPath $IsccPath)) {
        throw 'Inno Setup is missing. Run .\scripts\setup-inno.ps1 first, or pass -IsccPath.'
    }
    # An explicit target prevents an ARM64 or GNU host from mislabelling the ZIP.
    & "$PSScriptRoot/build.ps1" -Target 'x86_64-pc-windows-msvc'
    $binaries = Join-Path $root 'target/x86_64-pc-windows-msvc/release'
    $output = Join-Path $root "artifacts/$version"
    $name = "Serein-$version-windows-x64"
    $stage = Join-Path $output $name
    if (Test-Path -LiteralPath $stage) {
        # This path is always a versioned staging directory inside this workspace.
        $resolved = [IO.Path]::GetFullPath($stage)
        $boundary = [IO.Path]::GetFullPath((Join-Path $root 'artifacts')) + [IO.Path]::DirectorySeparatorChar
        if (!$resolved.StartsWith($boundary, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe staging path.' }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    foreach ($exe in @('serein-studio.exe', 'serein-player.exe')) {
        Copy-Item -LiteralPath (Join-Path $binaries $exe) -Destination $stage
    }
    # Rust's MSVC target links the VC runtime dynamically. Ship Microsoft's
    # redistributable copies app-locally rather than relying on a developer PC's PATH.
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (!(Test-Path -LiteralPath $vswhere)) { throw 'Visual Studio vswhere.exe is required to find the redistributable VC runtime.' }
    $runtimeFiles = @(& $vswhere -latest -products '*' -find 'VC/Redist/MSVC/**/x64/Microsoft.VC*.CRT/vcruntime140*.dll' | Where-Object { $_ -notmatch '[\\/]onecore[\\/]' } | Sort-Object -Descending | Group-Object { Split-Path -Leaf $_ } | ForEach-Object { $_.Group[0] })
    if ($LASTEXITCODE -ne 0 -or !($runtimeFiles | Where-Object { (Split-Path -Leaf $_) -eq 'vcruntime140.dll' })) {
        throw 'The x64 Microsoft VC runtime redistributable is missing. Install the C++ build tools redistributable components.'
    }
    foreach ($runtime in $runtimeFiles) { Copy-Item -LiteralPath $runtime -Destination $stage -Force }
    Copy-Item -LiteralPath "$root/README.md" -Destination $stage
    Copy-Item -LiteralPath "$root/docs" -Destination "$stage/docs" -Recurse
    Set-Content -LiteralPath "$stage/VERSION.txt" -Value $version -Encoding utf8
    # Shader source, preview textures and the creator guide are embedded in the EXEs.
    $zip = Join-Path $output "$name-portable.zip"
    Compress-Archive -Path "$stage/*" -DestinationPath $zip -Force
    $numericVersion = $version.Split('-')[0] + '.0'
    & $IsccPath "/DAppVersion=$version" "/DNumericVersion=$numericVersion" "/DSourceDir=$stage" "/DOutputDir=$output" "$root/installer/serein.iss"
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    $setup = Join-Path $output "$name-setup.exe"
    if (!(Test-Path -LiteralPath $setup)) { throw 'Installer output was not created.' }
    $hashes = foreach ($file in @($setup, $zip)) {
        $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
        '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $file)
    }
    Set-Content -LiteralPath "$output/SHA256SUMS.txt" -Value $hashes -Encoding ascii
    Write-Output "Packages: $output"
}
finally { Pop-Location }
