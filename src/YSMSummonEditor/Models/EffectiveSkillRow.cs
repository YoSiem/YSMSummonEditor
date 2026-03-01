namespace YSMSummonEditor.Models;

public sealed class EffectiveSkillRow
{
    public int StageIndex { get; init; }
    public int TreeSlotIndex { get; init; }
    public int SkillId { get; init; }
    public string SkillName { get; init; } = string.Empty;
    public int MinSkillLevel { get; init; }
    public int MaxSkillLevel { get; init; }
}
