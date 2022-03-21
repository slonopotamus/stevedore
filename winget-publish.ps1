$ErrorActionPreference = 'Stop'

$version = $args[0] -replace '.*/', "$1"

$ProgressPreference = 'SilentlyContinue'
iwr https://aka.ms/wingetcreate/latest -OutFile wingetcreate.exe
.\wingetcreate.exe update --urls "https://github.com/slonopotamus/stevedore/releases/download/${version}/stevedore-${version}-x86_64.msi" --version "${version}" --submit --token $args[1] "Slonopotamus.Stevedore"
