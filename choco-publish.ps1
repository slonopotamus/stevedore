$ErrorActionPreference = 'Stop'

New-Item -ItemType Directory -Force -Path target/choco/tools | Out-Null

$version = $args[0] -replace '.*/', "$1"
$msi_hash = (Get-FileHash "target/wix/stevedore-$version-x86_64.msi").Hash

(Get-Content choco/stevedore.nuspec) -replace '{{ version }}', $version | Set-Content "target/choco/stevedore-$version.nuspec"
(Get-Content choco/tools/chocolateyinstall.ps1) -replace '{{ version }}', $version -replace '{{ sha256 }}', $msi_hash | Set-Content target/choco/tools/chocolateyinstall.ps1
choco pack -out=target/choco/ "target/choco/stevedore-$version.nuspec"
choco apikey --key $args[1] -source https://push.chocolatey.org/
choco push "target/choco/stevedore.$version.nupkg"
