param(
    [ValidateSet("framework-dependent", "win-x64")]
    [string]$Flavor = "framework-dependent"
)

$ErrorActionPreference = "Stop"

$publishDir = Join-Path $env:USERPROFILE "Documents\YSMReleasedTools\YSMSummonEditor\$Flavor\"
$profile = if ($Flavor -eq "win-x64") { "Folder-win-x64" } else { "Folder-framework-dependent" }

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
    -p:GenerateDocumentationFile=false
