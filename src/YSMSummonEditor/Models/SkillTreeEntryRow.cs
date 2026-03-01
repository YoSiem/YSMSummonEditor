using CommunityToolkit.Mvvm.ComponentModel;
using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Models;

public sealed partial class SkillTreeEntryRow : ObservableObject
{
    private readonly SkillTreeEntrySnapshot _original;

    public SkillTreeEntryRow(SkillTreeEntrySnapshot snapshot, bool isNew = false)
    {
        _original = snapshot;
        IsNew = isNew;

        _skillId = snapshot.SkillId;
        _skillGroupId = snapshot.SkillGroupId;
        _minSkillLv = snapshot.MinSkillLv;
        _maxSkillLv = snapshot.MaxSkillLv;
        _lv = snapshot.Lv;
        _jobLv = snapshot.JobLv;
        _needSkillId1 = snapshot.NeedSkillId1;
        _needSkillLv1 = snapshot.NeedSkillLv1;
        _needSkillId2 = snapshot.NeedSkillId2;
        _needSkillLv2 = snapshot.NeedSkillLv2;
        _needSkillId3 = snapshot.NeedSkillId3;
        _needSkillLv3 = snapshot.NeedSkillLv3;
        _cEnhanceMin = snapshot.CEnhanceMin;
        _cEnhanceMax = snapshot.CEnhanceMax;
    }

    [ObservableProperty]
    private int _skillId;

    [ObservableProperty]
    private int _skillGroupId;

    [ObservableProperty]
    private int _minSkillLv;

    [ObservableProperty]
    private int _maxSkillLv;

    [ObservableProperty]
    private int _lv;

    [ObservableProperty]
    private int _jobLv;

    [ObservableProperty]
    private int _needSkillId1;

    [ObservableProperty]
    private int _needSkillLv1;

    [ObservableProperty]
    private int _needSkillId2;

    [ObservableProperty]
    private int _needSkillLv2;

    [ObservableProperty]
    private int _needSkillId3;

    [ObservableProperty]
    private int _needSkillLv3;

    [ObservableProperty]
    private int _cEnhanceMin;

    [ObservableProperty]
    private int _cEnhanceMax;

    [ObservableProperty]
    private string _skillName = string.Empty;

    [ObservableProperty]
    private bool _isDirty;

    public bool IsNew { get; }

    public SkillTreeEntrySnapshot ToSnapshot(int skillTreeId)
    {
        return new SkillTreeEntrySnapshot(
            skillTreeId,
            SkillId,
            SkillGroupId,
            MinSkillLv,
            MaxSkillLv,
            Lv,
            JobLv,
            NeedSkillId1,
            NeedSkillLv1,
            NeedSkillId2,
            NeedSkillLv2,
            NeedSkillId3,
            NeedSkillLv3,
            CEnhanceMin,
            CEnhanceMax);
    }

    private void RecalculateDirty()
    {
        var current = ToSnapshot(_original.SkillTreeId);
        IsDirty = IsNew || current != _original;
    }

    partial void OnSkillIdChanged(int value) => RecalculateDirty();
    partial void OnSkillGroupIdChanged(int value) => RecalculateDirty();
    partial void OnMinSkillLvChanged(int value) => RecalculateDirty();
    partial void OnMaxSkillLvChanged(int value) => RecalculateDirty();
    partial void OnLvChanged(int value) => RecalculateDirty();
    partial void OnJobLvChanged(int value) => RecalculateDirty();
    partial void OnNeedSkillId1Changed(int value) => RecalculateDirty();
    partial void OnNeedSkillLv1Changed(int value) => RecalculateDirty();
    partial void OnNeedSkillId2Changed(int value) => RecalculateDirty();
    partial void OnNeedSkillLv2Changed(int value) => RecalculateDirty();
    partial void OnNeedSkillId3Changed(int value) => RecalculateDirty();
    partial void OnNeedSkillLv3Changed(int value) => RecalculateDirty();
    partial void OnCEnhanceMinChanged(int value) => RecalculateDirty();
    partial void OnCEnhanceMaxChanged(int value) => RecalculateDirty();
}
