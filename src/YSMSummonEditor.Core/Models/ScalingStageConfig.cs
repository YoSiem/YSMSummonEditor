namespace YSMSummonEditor.Core.Models;

public sealed record ScalingStageConfig(
    int StageIndex,
    int Offset,
    int Threshold,
    int MaxLevel);
