namespace YSMSummonEditor.Models;

public sealed class EvolutionViewState
{
    public required string Name { get; init; }
    public required int StageIndex { get; init; }
    public required List<TreeContextState> Trees { get; init; }
}
