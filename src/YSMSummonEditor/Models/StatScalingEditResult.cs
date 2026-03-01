using YSMLib.ChangeTracking.Models;
using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Models;

public sealed record StatScalingEditResult(
    StatResourceContext? StatContext,
    IReadOnlyList<CreatureLevelBonusUpsert> BonusUpdates,
    IReadOnlyList<PendingFieldChange> FieldChanges);
