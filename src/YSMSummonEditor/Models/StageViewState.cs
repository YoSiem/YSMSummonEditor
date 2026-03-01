namespace YSMSummonEditor.Models;

public sealed class StageViewState
{
    public required string Name { get; init; }
    public required int StageIndex { get; init; }
    public required List<TreeContextState> Trees { get; init; }
}
