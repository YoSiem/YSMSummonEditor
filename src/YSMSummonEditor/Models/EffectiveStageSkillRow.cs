using CommunityToolkit.Mvvm.ComponentModel;
using System.Collections.ObjectModel;
using System.Windows.Media.Imaging;

namespace YSMSummonEditor.Models;

public sealed partial class EffectiveStageSkillRow : ObservableObject
{
    public EffectiveStageSkillRow(int skillId, int level)
    {
        SkillId = skillId;
        Level = level;
    }

    public int SkillId { get; }
    public int Level { get; }

    [ObservableProperty]
    private string _skillName = string.Empty;

    [ObservableProperty]
    private BitmapImage? _skillIcon;

    [ObservableProperty]
    private string _skillTooltipMarkup = string.Empty;

    [ObservableProperty]
    private string _skillTooltipPlain = string.Empty;

    [ObservableProperty]
    private int _minSkillLv;

    [ObservableProperty]
    private int _maxSkillLv;

    [ObservableProperty]
    private bool _isDirty;

    [ObservableProperty]
    private bool _isRandomGroup;

    [ObservableProperty]
    private string _randomGroupBadge = string.Empty;

    [ObservableProperty]
    private string _randomGroupTooltip = string.Empty;

    public ObservableCollection<SkillTreeEntryRow> LinkedEntries { get; } = [];

    public string ToolTipText =>
        BuildToolTipText();

    private string BuildToolTipText()
    {
        var baseText = string.IsNullOrWhiteSpace(SkillTooltipPlain)
            ? $"Skill {SkillId} | {SkillName} | Lv {Level} | Min {MinSkillLv} | Max {MaxSkillLv}"
            : SkillTooltipPlain;

        if (!IsRandomGroup || string.IsNullOrWhiteSpace(RandomGroupTooltip))
        {
            return baseText;
        }

        return $"{baseText}\n\n{RandomGroupTooltip}";
    }
}
