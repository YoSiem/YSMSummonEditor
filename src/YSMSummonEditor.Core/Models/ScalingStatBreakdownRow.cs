namespace YSMSummonEditor.Core.Models;

public sealed record ScalingStatBreakdownRow(
    string Stat,
    int Base,
    int Stage1,
    int Stage2,
    int Stage3,
    int Bonus,
    int Final,
    int Delta);
