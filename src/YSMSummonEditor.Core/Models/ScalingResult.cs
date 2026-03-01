namespace YSMSummonEditor.Core.Models;

public sealed record ScalingResult(
    int RequestedLevel,
    int EffectiveLevel,
    int RequestedEnhanceStage,
    int EffectiveEnhanceStage,
    decimal StatAmplify,
    IReadOnlyList<ScalingStatBreakdownRow> Rows);
