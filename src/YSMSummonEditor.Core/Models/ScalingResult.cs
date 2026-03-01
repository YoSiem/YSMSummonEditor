namespace YSMSummonEditor.Core.Models;

public sealed record ScalingResult(
    int RequestedLevel,
    int EffectiveLevel,
    IReadOnlyList<ScalingStatBreakdownRow> Rows);
