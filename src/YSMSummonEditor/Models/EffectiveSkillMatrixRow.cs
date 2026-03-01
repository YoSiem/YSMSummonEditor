using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;

namespace YSMSummonEditor.Models;

public sealed partial class EffectiveSkillMatrixRow : ObservableObject
{
    private SkillTreeEntryRow? _stage1Entry;
    private SkillTreeEntryRow? _stage2Entry;
    private SkillTreeEntryRow? _stage3Entry;

    public EffectiveSkillMatrixRow(int treeSlotIndex, int skillId, int level)
    {
        TreeSlotIndex = treeSlotIndex;
        SkillId = skillId;
        Level = level;
    }

    public int TreeSlotIndex { get; }
    public int SkillId { get; }
    public int Level { get; }

    [ObservableProperty]
    private string _skillName = string.Empty;

    [ObservableProperty]
    private bool _isDirty;

    public bool E1Available => _stage1Entry is not null;
    public bool E2Available => _stage2Entry is not null;
    public bool E3Available => _stage3Entry is not null;

    public int? E1Min
    {
        get => _stage1Entry?.MinSkillLv;
        set => SetStageMin(1, value);
    }

    public int? E1Max
    {
        get => _stage1Entry?.MaxSkillLv;
        set => SetStageMax(1, value);
    }

    public int? E2Min
    {
        get => _stage2Entry?.MinSkillLv;
        set => SetStageMin(2, value);
    }

    public int? E2Max
    {
        get => _stage2Entry?.MaxSkillLv;
        set => SetStageMax(2, value);
    }

    public int? E3Min
    {
        get => _stage3Entry?.MinSkillLv;
        set => SetStageMin(3, value);
    }

    public int? E3Max
    {
        get => _stage3Entry?.MaxSkillLv;
        set => SetStageMax(3, value);
    }

    public string ToolTipText => $"Tree {TreeSlotIndex}, Skill {SkillId}, Lv {Level}";

    public int FirstAvailableStage =>
        E1Available ? 1 :
        E2Available ? 2 :
        E3Available ? 3 : 0;

    public void AttachStageEntry(int stageIndex, SkillTreeEntryRow entry)
    {
        switch (stageIndex)
        {
            case 1:
                SwapStageEntry(ref _stage1Entry, entry);
                break;
            case 2:
                SwapStageEntry(ref _stage2Entry, entry);
                break;
            case 3:
                SwapStageEntry(ref _stage3Entry, entry);
                break;
            default:
                return;
        }

        UpdateFromEntries();
    }

    public bool TryGetEntry(int stageIndex, out SkillTreeEntryRow? entry)
    {
        entry = stageIndex switch
        {
            1 => _stage1Entry,
            2 => _stage2Entry,
            3 => _stage3Entry,
            _ => null
        };

        return entry is not null;
    }

    private void SetStageMin(int stageIndex, int? value)
    {
        if (value is null || !TryGetEntry(stageIndex, out var entry) || entry is null)
        {
            return;
        }

        if (entry.MinSkillLv == value.Value)
        {
            return;
        }

        entry.MinSkillLv = value.Value;
        UpdateFromEntries();
    }

    private void SetStageMax(int stageIndex, int? value)
    {
        if (value is null || !TryGetEntry(stageIndex, out var entry) || entry is null)
        {
            return;
        }

        if (entry.MaxSkillLv == value.Value)
        {
            return;
        }

        entry.MaxSkillLv = value.Value;
        UpdateFromEntries();
    }

    private void SwapStageEntry(ref SkillTreeEntryRow? current, SkillTreeEntryRow replacement)
    {
        if (ReferenceEquals(current, replacement))
        {
            return;
        }

        if (current is not null)
        {
            current.PropertyChanged -= OnEntryPropertyChanged;
        }

        current = replacement;
        current.PropertyChanged += OnEntryPropertyChanged;
    }

    private void OnEntryPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(SkillTreeEntryRow.MinSkillLv):
            case nameof(SkillTreeEntryRow.MaxSkillLv):
            case nameof(SkillTreeEntryRow.IsDirty):
            case nameof(SkillTreeEntryRow.SkillName):
                UpdateFromEntries();
                break;
        }
    }

    private void UpdateFromEntries()
    {
        OnPropertyChanged(nameof(E1Min));
        OnPropertyChanged(nameof(E1Max));
        OnPropertyChanged(nameof(E2Min));
        OnPropertyChanged(nameof(E2Max));
        OnPropertyChanged(nameof(E3Min));
        OnPropertyChanged(nameof(E3Max));
        OnPropertyChanged(nameof(E1Available));
        OnPropertyChanged(nameof(E2Available));
        OnPropertyChanged(nameof(E3Available));
        OnPropertyChanged(nameof(ToolTipText));

        var resolvedName = _stage1Entry?.SkillName
            ?? _stage2Entry?.SkillName
            ?? _stage3Entry?.SkillName
            ?? $"Skill {SkillId}";

        SkillName = resolvedName;
        IsDirty = (_stage1Entry?.IsDirty ?? false)
            || (_stage2Entry?.IsDirty ?? false)
            || (_stage3Entry?.IsDirty ?? false);
    }
}
