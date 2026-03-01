using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Core.Models;

namespace YSMSummonEditor.Core.Services;

public interface IStatScalingCalculator
{
    ScalingResult Calculate(ScalingInput input);
}

public sealed class StatScalingCalculator : IStatScalingCalculator
{
    private static readonly IReadOnlyList<ScalingStageConfig> StageConfigs =
    [
        new ScalingStageConfig(1, 0, 50, 60),
        new ScalingStageConfig(2, 50, 50, 65),
        new ScalingStageConfig(3, 100, 200, 200)
    ];

    private static readonly IReadOnlyList<StatDescriptor> StatDescriptors =
    [
        new StatDescriptor("STR", x => x.Str, x => x.Str),
        new StatDescriptor("VIT", x => x.Vit, x => x.Vit),
        new StatDescriptor("DEX", x => x.Dex, x => x.Dex),
        new StatDescriptor("AGI", x => x.Agi, x => x.Agi),
        new StatDescriptor("INT", x => x.Int, x => x.Int),
        new StatDescriptor("MEN", x => x.Men, x => x.Men),
        new StatDescriptor("LUK", x => x.Luk, x => x.Luk)
    ];

    public ScalingResult Calculate(ScalingInput input)
    {
        var snapshot = input.Snapshot;
        var level = Math.Clamp(input.Level, 1, 350);
        var previousLevel = Math.Max(level - 1, 0);
        var requestedEnhanceStage = input.EnhanceStage;
        var enhanceStage = Math.Clamp(requestedEnhanceStage, 0, 5);
        var statAmplify = input.EnhanceAmplifyByLevel.TryGetValue(enhanceStage, out var amplify)
            ? amplify
            : 0m;
        var enhanceMultiplier = 1m + statAmplify;
        var growthDepth = Math.Clamp(snapshot.GrowthDepth, 1, 3);
        var stageLookup = snapshot.StageBonuses.ToDictionary(x => x.StageIndex);

        var rows = new List<ScalingStatBreakdownRow>(StatDescriptors.Count);
        foreach (var descriptor in StatDescriptors)
        {
            var stageBonuses = new int[3];
            var stageBonusesPrevious = new int[3];
            for (var stageIndex = 1; stageIndex <= growthDepth; stageIndex++)
            {
                var config = StageConfigs[stageIndex - 1];
                var coefficient = stageLookup.TryGetValue(stageIndex, out var stageData)
                    ? descriptor.CoefficientSelector(stageData)
                    : 0m;

                stageBonuses[stageIndex - 1] = CalculateStageBonus(coefficient, config, stageIndex, level);
                stageBonusesPrevious[stageIndex - 1] = CalculateStageBonus(coefficient, config, stageIndex, previousLevel);
            }

            var baseValue = descriptor.BaseSelector(snapshot.BaseStats);
            var totalBonus = stageBonuses.Sum();
            var totalBonusPrevious = stageBonusesPrevious.Sum();
            var finalRaw = baseValue + totalBonus;
            var finalRawPrevious = baseValue + totalBonusPrevious;
            var finalValue = (int)decimal.Truncate(finalRaw * enhanceMultiplier);
            var finalPrevious = (int)decimal.Truncate(finalRawPrevious * enhanceMultiplier);
            var delta = finalValue - finalPrevious;

            rows.Add(new ScalingStatBreakdownRow(
                descriptor.Name,
                baseValue,
                stageBonuses[0],
                stageBonuses[1],
                stageBonuses[2],
                totalBonus,
                finalRaw,
                finalValue,
                delta));
        }

        return new ScalingResult(input.Level, level, requestedEnhanceStage, enhanceStage, statAmplify, rows);
    }

    private static int CalculateStageBonus(decimal coefficient, ScalingStageConfig config, int stageIndex, int level)
    {
        var stageLevel = Math.Clamp(level - config.Offset, 0, config.MaxLevel);
        if (stageLevel <= 0 || coefficient == 0m)
        {
            return 0;
        }

        var normalLevels = Math.Min(stageLevel, config.Threshold);
        var overLevels = Math.Max(stageLevel - config.Threshold, 0);
        var effectiveStageLevel = normalLevels + (overLevels * 3m);
        var stageMultiplier = 1m + ((stageIndex - 1) * 0.25m);
        var raw = coefficient * effectiveStageLevel * stageMultiplier;
        return (int)decimal.Truncate(raw);
    }

    private sealed record StatDescriptor(
        string Name,
        Func<StatBlockSnapshot, int> BaseSelector,
        Func<CreatureLevelBonusStageSnapshot, decimal> CoefficientSelector);
}
