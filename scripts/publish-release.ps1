param()

$ErrorActionPreference = "Stop"

$publishDir = Join-Path $env:USERPROFILE "Documents\YSMReleasedTools\YSMSummonEditor\"
$profile = "Folder-framework-dependent"
$allowedExtensions = @('.exe', '.json', '.config')

if (Test-Path $publishDir) {
    Remove-Item -Path $publishDir -Recurse -Force
}

New-Item -ItemType Directory -Path $publishDir -Force | Out-Null

dotnet publish ".\src\YSMSummonEditor\YSMSummonEditor.csproj" `
    -c Release `
    -p:PublishProfile=$profile `
    -p:PublishDir="$publishDir" `
    -p:GeneratePackageOnBuild=false `
    -p:DebugSymbols=false `
    -p:DebugType=None `
    -p:GenerateDocumentationFile=false `
    -p:CopyOutputSymbolsToPublishDirectory=false `
    -p:IncludeSymbolsInSingleFile=false `
    -p:PublishSingleFile=true `
    -p:SelfContained=false

Get-ChildItem -Path $publishDir -Filter *.pdb -File -ErrorAction SilentlyContinue | Remove-Item -Force
Get-ChildItem -Path $publishDir -Filter *.xml -File -ErrorAction SilentlyContinue | Remove-Item -Force
Get-ChildItem -Path $publishDir -Directory -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force
Get-ChildItem -Path $publishDir -File -ErrorAction SilentlyContinue |
    Where-Object { $allowedExtensions -notcontains $_.Extension.ToLowerInvariant() } |
    Remove-Item -Force
