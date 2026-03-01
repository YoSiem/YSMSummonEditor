using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Core.Models;

public sealed record ScalingInput(
    int Level,
    int EnhanceStage,
    IReadOnlyDictionary<int, decimal> EnhanceAmplifyByLevel,
    SummonStatScalingSnapshot Snapshot);
