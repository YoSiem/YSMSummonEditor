namespace YSMSummonEditor.Models;

public sealed record RandomSkillGroupEditModel(
    int GroupId,
    int UseCode,
    IReadOnlyList<int> SkillIds)
{
    public static RandomSkillGroupEditModel Empty(int groupId) => new(groupId, 0, []);
}
