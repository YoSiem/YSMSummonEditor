using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Core.Models;

public sealed record ScalingInput(
    int Level,
    SummonStatScalingSnapshot Snapshot);
