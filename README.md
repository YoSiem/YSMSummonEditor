# YSMSummonEditor

`YSMSummonEditor` is a Windows desktop editor for Rappelz summon data, built with WPF and backed by MSSQL. It is intended for structured content editing with staged changes, review before save, and a workflow tailored for summon, skill tree, random skill group, and stat-scaling work.

## Overview

The application combines summon browsing, multi-evolution editing, skill and skill tree management, stat scaling tools, image lookup, and controlled save/export flow in a single dark-themed editor. It is built on top of `YSMLib` and currently targets MSSQL only.

## Core Capabilities

### Summon browsing and evolution chain

- Search by summon ID, summon name, or related text using `Go` or `Enter`
- Browse summons with 36x36 icons
- Load the full three-step evolution chain automatically for the selected final summon
- Display summon header icon, feature text, card metadata, and a visual `Evolution 1 -> Evolution 2 -> Evolution 3` chain

### Skill tree editing

- Inline editing of `SkillTreeResource` rows
- Right-click actions for:
  - editing a skill
  - deleting a row
  - creating a random skill group
  - editing a random skill group
  - detaching a random skill group
- Dirty row highlighting for staged changes
- Effective skills view synchronized with the currently active evolution stage

### Skill editing

- Dedicated skill editor for `SkillResource`
- Business-field editing and advanced raw-field editing
- Staged updates integrated into the global save and review flow

### Random skill groups

- Full support for `SummonRandomSkillResource`
- Recognition of `skill_group_id` in tree rows with `RND[GroupId]` badge display
- Random group editor with:
  - skill search by exact ID and name contains
  - 36x36 skill icons
  - tooltip support
  - maximum 15 entries
  - duplicate prevention
  - automatic sorting by `SkillId`
  - automatic `use_code = selected.Count`
- Automatic group ID allocation based on both:
  - `SummonRandomSkillResource.sid`
  - `SkillTreeResource.skill_group_id`
- Manual positive `Grp` entry can create an empty placeholder group automatically
- Shared group usage warnings with clone-or-edit-in-place decision during save

### Stat scaling editor

- Modal stats window for the selected final summon
- Editable data sources:
  - `StatResource`
  - `CreatureLevelBonus`
- Live calculator using:
  - level range `1..350`
  - enhance stage range `0..5`
  - `CreatureEnhance.stat_amplify`
- Displays raw and enhanced final values

### Save, review, and export

- All edits are staged first
- Save always goes through review
- Save target toggle:
  - `DB` for direct MSSQL write
  - `.sql` for diff export without touching the database
- Global pre-save confirmation before database commit
- Shared usage warnings for skills, skill trees, stats, growth bonuses, and random groups
- Unsaved-change protection when switching to another summon:
  - `Save`
  - `Discard`
  - `Cancel`

### Assets and UI

- Default dump folder auto-falls back to `%USERPROFILE%\Desktop\Dump` when empty
- Recursive asset indexing for summon and skill images
- Embedded branding assets and default 36x36 fallback icon
- Dark grayscale UI with custom scrollbars and game-oriented tooltip rendering

## Requirements

- Windows
- .NET 10 Desktop Runtime
- MSSQL access to the target Rappelz database

## Installation

### Release output

Published builds are written to:

`C:\Users\<you>\Documents\YSMReleasedTools\YSMSummonEditor\`

The release output is intentionally kept minimal. The publish script removes non-runtime artifacts and keeps only the executable and config/json-style runtime files when they exist.

### Local database bootstrap

Create a local `.env.local` file in the repository root. This file is ignored by git.

Example:

```env
YSM_DB_PROVIDER=MSSQL
YSM_DB_CONNECTION_STRING=Server=127.0.0.1,1433;Database=YourDb;User Id=YourUser;Password=YourPass;TrustServerCertificate=True;
```

Tracked template:

`./.env.local.template`

## First Run

1. Start `YSMSummonEditor.exe`
2. Open `Settings`
3. Confirm the database connection
4. Confirm the dump folder path

If no dump path is stored yet, the application defaults to:

`%USERPROFILE%\Desktop\Dump`

That folder is created automatically if missing.

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

Publish target:

`C:\Users\<you>\Documents\YSMReleasedTools\YSMSummonEditor\`

## Runtime Files

- User settings: `%LocalAppData%\YSMSummonEditor\settings.json`
- Local DB bootstrap: `./.env.local`

## Main Data Sources

- `SummonResource`
- `SkillTreeResource`
- `SkillResource`
- `SummonRandomSkillResource`
- `StatResource`
- `CreatureLevelBonus`
- `CreatureEnhance`
- `StringResource`
- `ItemResource`

## Tech Stack

- `WPF`
- `.NET 10`
- `WPF-UI`
- `ModernWpf`
- `CommunityToolkit.Mvvm`
- `Microsoft.Extensions.Hosting`
- `YSMLib`
