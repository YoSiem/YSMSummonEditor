# YSMSummonEditor

Summon Editor for Rappelz resources (WPF, .NET 10).

## Stack

- WPF (`net10.0-windows`)
- `WPF-UI` + `ModernWpf`
- `CommunityToolkit.Mvvm`
- `Microsoft.Extensions.Hosting` DI
- MSSQL data source (`YSMLib.GameData.SqlServer`)

## Features in v1

- Summon browser with filters and 36x36 mini icons.
- Final-evo selection (`evolve_target = 0`) with automatic 3-stage chain loading.
- Skill tree editing (`SkillTreeResource`) with add/edit/delete.
- Skill editing dialog (`SkillResource`) with whitelist and advanced raw fields.
- Pending-change highlighting and review popup before commit.
- Shared tree and global skill usage warnings before save.
- Dump-folder recursive asset indexing for summon/skill images.

## Build

```powershell
dotnet restore YSMSummonEditor.slnx
dotnet build YSMSummonEditor.slnx -c Release
dotnet test YSMSummonEditor.slnx -c Release
```

## Runtime

Settings are stored in `%LocalAppData%\YSMSummonEditor\settings.json`.

## Publish

```powershell
dotnet publish .\src\YSMSummonEditor\YSMSummonEditor.csproj -c Release -p:PublishProfile=Folder-win-x64
dotnet publish .\src\YSMSummonEditor\YSMSummonEditor.csproj -c Release -p:PublishProfile=Folder-framework-dependent
```
