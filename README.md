# YSMSummonEditor

`YSMSummonEditor` is a desktop editor for Rappelz summon resources (WPF, .NET 10, MSSQL).
It is focused on safe staged editing, readable UI, and fast summon/skill workflow.

## What You Can Do

### Summons and evolution chain

- Search summons by text or exact ID (`Go` button or `Enter`).
- Browse summon list with 36x36 icons.
- Load full 3-evolution chain automatically for selected final summon.
- See summon header icon, description (`text_feature_id`), and chain as icon flow (`Evo1 -> Evo2 -> Evo3`).

### Skill trees and skills

- Edit `SkillTreeResource` rows directly in grid (inline values).
- Open skill editor by right-click (`Edit Skill`) in `SkillTree` or `Effective Skills`.
- Delete rows by right-click with confirmation.
- Staged dirty tracking with visual row highlight.

### Random skill groups (`SummonRandomSkillResource`)

- Full support for `skill_group_id` in skill tree rows.
- Right-click actions in `SkillTree`:
  - `Create Random Group`
  - `Edit Random Group`
  - `Detach Random Group`
- Random group editor window:
  - Search skills (`contains + exact ID`, limit 300)
  - Add/remove/clear selected skills
  - Max 15 entries, no duplicates
  - Auto-sort by `SkillId`
  - `use_code` = selected count (auto)
- New group IDs are generated automatically without collisions.
- Manual `Grp` entry with unknown positive ID auto-creates placeholder group (`use_code=0`).
- Random groups are shown in UI with `RND[GroupId]` badge and detailed tooltip.

### Stat scaling editor

- Open modal `Stats` window for final summon.
- 3 sections:
  - Base stats (`StatResource`)
  - Growth bonuses (`CreatureLevelBonus`)
  - Calculator (level + breakdown)
- Calculator supports:
  - Level range `1..350`
  - `Enhance Stage` `0..5`
  - `CreatureEnhance.stat_amplify`
  - `FinalRaw` and enhanced `Final`

### Save workflow

- All edits are staged first.
- Save opens review window with change list.
- Save mode toggle:
  - `DB` -> write to MSSQL
  - `.sql` -> generate SQL script only
- In `DB` mode, global pre-save confirm shows destination DB and payload summary.
- Shared usage warnings are supported (skill/tree/stat/random group impacts, clone-or-in-place decisions where applicable).
- Unsaved-change guard when switching summon: `Save / Discard / Cancel`.

### Assets and visuals

- Default dump folder is auto-set to `%USERPROFILE%\Desktop\Dump` if empty (folder auto-created).
- Recursive asset index for summon/skill images.
- Built-in 36x36 fallback icon for missing assets: `common_mark_icon_unknown_middle.jpg`.
- Embedded branding assets (`Logo.ico`, `Heaven_logo1.png`).
- Dark grayscale UI with unified scrollbars and readable contrast.

## Requirements

- Windows
- .NET 10 Desktop Runtime
- MSSQL access to your game database

## Install / First Setup (Release Build)

1. Publish or copy released files to:
   - `C:\Users\<you>\Documents\YSMReleasedTools\YSMSummonEditor\`
2. In repository root, create local env file:
   - `.env.local` (not committed)
3. Put DB connection there, for example:

```env
YSM_DB_PROVIDER=MSSQL
YSM_DB_CONNECTION_STRING=Server=127.0.0.1,1433;Database=YourDb;User Id=YourUser;Password=YourPass;TrustServerCertificate=True;
```

4. Run `YSMSummonEditor.exe`.
5. Open `Settings` and confirm:
   - DB settings
   - Dump folder path (defaults to Desktop\Dump only when empty)

## Build From Source

```powershell
dotnet restore YSMSummonEditor.slnx
dotnet build YSMSummonEditor.slnx -c Release
dotnet test YSMSummonEditor.slnx -c Release
```

## Publish

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\publish-release.ps1
```

Output:

- `C:\Users\<you>\Documents\YSMReleasedTools\YSMSummonEditor\`
- Framework-dependent, clean output (`.pdb` and `.xml` removed)

## Runtime Files

- User settings:
  - `%LocalAppData%\YSMSummonEditor\settings.json`
- Local DB bootstrap:
  - `.env.local` in repo root (ignored by git)
  - `.env.local.template` is tracked template

## Main Data Tables Used

- `SummonResource`
- `SkillTreeResource`
- `SkillResource`
- `SummonRandomSkillResource`
- `StatResource`
- `CreatureLevelBonus`
- `CreatureEnhance`
- `StringResource`
- `ItemResource`
