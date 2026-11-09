function Get-SereinVersion {
    param([string]$ExpectedTag)
    $root = Split-Path -Parent $PSScriptRoot
    $json = & cargo metadata --manifest-path "$root/Cargo.toml" --no-deps --format-version 1 --locked
    if ($LASTEXITCODE -ne 0) { throw 'Unable to read Cargo workspace versions.' }
    $metadata = $json | ConvertFrom-Json
    $packages = @($metadata.packages | Where-Object { $_.id -in $metadata.workspace_members })
    $versions = @($packages.version | Select-Object -Unique)
    if ($versions.Count -ne 1) { throw 'All workspace crates must inherit the same workspace version.' }
    $version = $versions[0]
    if ($version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?$') {
        throw "Unsupported release version '$version'. Use MAJOR.MINOR.PATCH with an optional SemVer prerelease suffix."
    }
    # Windows file versions use four 16-bit numbers. Build metadata is deliberately
    # excluded from release names so different tags cannot name the same package.
    foreach ($part in $version.Split('-')[0].Split('.')) {
        if ([long]$part -gt 65535) { throw 'Windows version components must be at most 65535.' }
    }
    if ($ExpectedTag -and $ExpectedTag -cne "v$version") {
        throw "Tag '$ExpectedTag' does not match workspace version 'v$version'. Update Cargo.toml and Cargo.lock before tagging."
    }
    return $version
}
