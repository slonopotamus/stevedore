$ErrorActionPreference = 'Stop'

$version = $args[0] -replace '.*/', "$1"
winget install wingetcreate
wingetcreate update --urls "https://github.com/slonopotamus/stevedore/releases/download/${version}/stevedore-${version}-x86_64.msi" --version "${version}" --submit --token "${args[1]}" "Slonopotamus.Stevedore"
