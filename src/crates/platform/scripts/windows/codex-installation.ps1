# Read only the current user's registered official package, never all users.
# OpenAI's deployment documentation identifies the package as OpenAI.Codex.
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
Get-AppxPackage -Name 'OpenAI.Codex' |
    Sort-Object { [Version]$_.Version } -Descending |
    ForEach-Object {
        $package = $_
        $manifest = Get-AppxPackageManifest -Package $package.PackageFullName
        foreach ($application in $manifest.Package.Applications.Application) {
            $entry = [string]$application.Executable
            if ($entry) {
                [IO.Path]::Combine($package.InstallLocation, $entry) | ConvertTo-Json -Compress
            }
        }
    }
