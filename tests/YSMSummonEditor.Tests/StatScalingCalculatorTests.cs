using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Core.Models;
using YSMSummonEditor.Core.Services;

namespace YSMSummonEditor.Tests;

public sealed class StatScalingCalculatorTests
{
    private static readonly IReadOnlyDictionary<int, decimal> NoAmplify = new Dictionary<int, decimal>
    {
        [0] = 0m
    };

    [Fact]
    public void Calculate_MatchesGuideExample_ForStr()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = new SummonStatScalingSnapshot(
            SummonId: 102,
            StatId: 9001,
            EvolveType: 2,
            GrowthDepth: 2,
            BaseStats: new StatBlockSnapshot(9001, 20, 0, 0, 0, 0, 0, 0),
            StageBonuses:
            [
                new CreatureLevelBonusStageSnapshot(1, 101, 0.30m, 0, 0, 0, 0, 0, 0, true),
                new CreatureLevelBonusStageSnapshot(2, 102, 0.50m, 0, 0, 0, 0, 0, 0, true)
            ]);

        var result = calculator.Calculate(new ScalingInput(80, 0, NoAmplify, snapshot));
        var str = result.Rows.Single(x => x.Stat == "STR");

        Assert.Equal(24, str.Stage1);
        Assert.Equal(18, str.Stage2);
        Assert.Equal(42, str.Bonus);
        Assert.Equal(62, str.FinalRaw);
        Assert.Equal(62, str.Final);
    }

    [Fact]
    public void Calculate_UsesTruncate_PerStage()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = new SummonStatScalingSnapshot(
            SummonId: 100,
            StatId: 1,
            EvolveType: 1,
            GrowthDepth: 1,
            BaseStats: new StatBlockSnapshot(1, 0, 0, 0, 0, 0, 0, 0),
            StageBonuses:
            [
                new CreatureLevelBonusStageSnapshot(1, 100, 0.3333m, 0, 0, 0, 0, 0, 0, true)
            ]);

        var result = calculator.Calculate(new ScalingInput(10, 0, NoAmplify, snapshot));
        var str = result.Rows.Single(x => x.Stat == "STR");

        Assert.Equal(3, str.Bonus);
    }

    [Fact]
    public void Calculate_MissingStageBonus_UsesZero()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = new SummonStatScalingSnapshot(
            SummonId: 200,
            StatId: 1,
            EvolveType: 3,
            GrowthDepth: 3,
            BaseStats: new StatBlockSnapshot(1, 10, 0, 0, 0, 0, 0, 0),
            StageBonuses:
            [
                new CreatureLevelBonusStageSnapshot(1, 198, 0.2m, 0, 0, 0, 0, 0, 0, true)
            ]);

        var result = calculator.Calculate(new ScalingInput(120, 0, NoAmplify, snapshot));
        var str = result.Rows.Single(x => x.Stat == "STR");

        Assert.True(str.Stage1 > 0);
        Assert.Equal(0, str.Stage2);
        Assert.Equal(0, str.Stage3);
    }

    [Fact]
    public void Calculate_ClampsLevelTo350()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = CreateSimpleSnapshot();

        var result = calculator.Calculate(new ScalingInput(999, 0, NoAmplify, snapshot));

        Assert.Equal(350, result.EffectiveLevel);
    }

    [Fact]
    public void Calculate_DeltaIsBonusDifference()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = CreateSimpleSnapshot();

        var level1 = calculator.Calculate(new ScalingInput(1, 0, NoAmplify, snapshot)).Rows.Single(x => x.Stat == "STR");
        var level2 = calculator.Calculate(new ScalingInput(2, 0, NoAmplify, snapshot)).Rows.Single(x => x.Stat == "STR");

        Assert.Equal(1, level1.Delta);
        Assert.Equal(level2.Final - level1.Final, level2.Delta);
    }

    [Fact]
    public void Calculate_AppliesEnhanceAmplifyToFinalStat()
    {
        var calculator = new StatScalingCalculator();
        var snapshot = new SummonStatScalingSnapshot(
            SummonId: 100,
            StatId: 1,
            EvolveType: 1,
            GrowthDepth: 1,
            BaseStats: new StatBlockSnapshot(1, 100, 0, 0, 0, 0, 0, 0),
            StageBonuses:
            [
                new CreatureLevelBonusStageSnapshot(1, 100, 0m, 0, 0, 0, 0, 0, 0, true)
            ]);
        var enhanceMap = new Dictionary<int, decimal>
        {
            [0] = 0m,
            [5] = 0.1234m
        };

        var result = calculator.Calculate(new ScalingInput(1, 5, enhanceMap, snapshot));
        var str = result.Rows.Single(x => x.Stat == "STR");

        Assert.Equal(100, str.FinalRaw);
        Assert.Equal(112, str.Final);
        Assert.Equal(5, result.EffectiveEnhanceStage);
        Assert.Equal(0.1234m, result.StatAmplify);
    }

    private static SummonStatScalingSnapshot CreateSimpleSnapshot()
    {
        return new SummonStatScalingSnapshot(
            SummonId: 100,
            StatId: 1,
            EvolveType: 1,
            GrowthDepth: 1,
            BaseStats: new StatBlockSnapshot(1, 0, 0, 0, 0, 0, 0, 0),
            StageBonuses:
            [
                new CreatureLevelBonusStageSnapshot(1, 100, 1m, 0, 0, 0, 0, 0, 0, true)
            ]);
    }
}
