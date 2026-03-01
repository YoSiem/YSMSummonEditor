using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows;
using YSMLib.ChangeTracking.Models;
using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Core.Models;
using YSMSummonEditor.Core.Services;
using YSMSummonEditor.Models;

namespace YSMSummonEditor.Windows;

public partial class StatScalingWindow : Window
{
    private readonly SummonStatScalingSnapshot _original;
    private readonly IStatScalingCalculator _calculator;
    private readonly IReadOnlyDictionary<int, decimal> _enhanceAmplifyByLevel;
    private readonly ObservableCollection<BaseStatEditRow> _baseRows = [];
    private readonly ObservableCollection<CreatureLevelBonusEditRow> _bonusRows = [];

    public StatScalingWindow(
        SummonStatScalingSnapshot snapshot,
        IStatScalingCalculator calculator,
        IReadOnlyList<CreatureEnhanceSnapshot> creatureEnhance)
    {
        InitializeComponent();

        _original = snapshot;
        _calculator = calculator;
        _enhanceAmplifyByLevel = creatureEnhance
            .GroupBy(x => x.EnhanceLevel)
            .Select(x => x.Last())
            .ToDictionary(x => x.EnhanceLevel, x => x.StatAmplify);

        TxtHeader.Text = $"Stat Scaling | Summon {_original.SummonId} | Stat ID {_original.StatId} | Evolution Depth {_original.GrowthDepth}";

        BuildRows();
        GridBase.ItemsSource = _baseRows;
        GridBonus.ItemsSource = _bonusRows;

        TxtLevel.Text = "1";
        TxtEnhanceStage.Text = "0";
        RefreshCalculator();
    }

    public StatScalingEditResult? Result { get; private set; }

    private void BuildRows()
    {
        AddBaseRow("STR", _original.BaseStats.Str);
        AddBaseRow("VIT", _original.BaseStats.Vit);
        AddBaseRow("DEX", _original.BaseStats.Dex);
        AddBaseRow("AGI", _original.BaseStats.Agi);
        AddBaseRow("INT", _original.BaseStats.Int);
        AddBaseRow("MEN", _original.BaseStats.Men);
        AddBaseRow("LUK", _original.BaseStats.Luk);

        foreach (var stage in _original.StageBonuses.OrderBy(x => x.StageIndex))
        {
            var row = new CreatureLevelBonusEditRow(
                stage.StageIndex,
                stage.LookupId,
                stage.ExistsInDb,
                stage.Str,
                stage.Vit,
                stage.Dex,
                stage.Agi,
                stage.Int,
                stage.Men,
                stage.Luk);

            row.PropertyChanged += (_, _) => RefreshCalculator();
            _bonusRows.Add(row);
        }
    }

    private void AddBaseRow(string name, int value)
    {
        var row = new BaseStatEditRow(name, value);
        row.PropertyChanged += (_, _) => RefreshCalculator();
        _baseRows.Add(row);
    }

    private SummonStatScalingSnapshot BuildSnapshotFromUi()
    {
        var baseSnapshot = new StatBlockSnapshot(
            _original.StatId,
            GetBaseValue("STR"),
            GetBaseValue("VIT"),
            GetBaseValue("DEX"),
            GetBaseValue("AGI"),
            GetBaseValue("INT"),
            GetBaseValue("MEN"),
            GetBaseValue("LUK"));

        var stageSnapshots = _bonusRows
            .Select(x => new CreatureLevelBonusStageSnapshot(
                x.StageIndex,
                x.LookupId,
                NormalizeDecimal(x.Str),
                NormalizeDecimal(x.Vit),
                NormalizeDecimal(x.Dex),
                NormalizeDecimal(x.Agi),
                NormalizeDecimal(x.Int),
                NormalizeDecimal(x.Men),
                NormalizeDecimal(x.Luk),
                x.ExistsInDb))
            .OrderBy(x => x.StageIndex)
            .ToList();

        return new SummonStatScalingSnapshot(
            _original.SummonId,
            _original.StatId,
            _original.EvolveType,
            _original.GrowthDepth,
            baseSnapshot,
            stageSnapshots);
    }

    private int GetBaseValue(string name)
    {
        return _baseRows.First(x => string.Equals(x.StatName, name, StringComparison.Ordinal)).CurrentValue;
    }

    private void RefreshCalculator()
    {
        var requestedLevel = ParseLevel(TxtLevel.Text);
        var requestedEnhanceStage = ParseEnhanceStage(TxtEnhanceStage.Text);
        var result = _calculator.Calculate(new ScalingInput(
            requestedLevel,
            requestedEnhanceStage,
            _enhanceAmplifyByLevel,
            BuildSnapshotFromUi()));
        GridResult.ItemsSource = result.Rows;
        TxtEnhanceAmplify.Text = $"Stat amplify: {result.StatAmplify:F4}";
        TxtCalcInfo.Text = $"Level {result.RequestedLevel} -> {result.EffectiveLevel} | Enhance Stage {result.RequestedEnhanceStage} -> {result.EffectiveEnhanceStage}.";
    }

    private static int ParseLevel(string? raw)
    {
        return int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : 1;
    }

    private static int ParseEnhanceStage(string? raw)
    {
        return int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : 0;
    }

    private static decimal NormalizeDecimal(decimal value)
    {
        return decimal.Round(value, 4, MidpointRounding.AwayFromZero);
    }

    private void TxtLevel_TextChanged(object sender, System.Windows.Controls.TextChangedEventArgs e)
    {
        RefreshCalculator();
    }

    private void TxtEnhanceStage_TextChanged(object sender, System.Windows.Controls.TextChangedEventArgs e)
    {
        RefreshCalculator();
    }

    private void BtnApply_Click(object sender, RoutedEventArgs e)
    {
        var current = BuildSnapshotFromUi();

        StatResourceContext? statContext = null;
        if (IsBaseChanged())
        {
            statContext = new StatResourceContext(
                _original.SummonId,
                _original.StatId,
                current.BaseStats,
                CloneIfShared: false);
        }

        var bonusUpdates = _bonusRows
            .Where(x => x.IsDirty)
            .Select(x => new CreatureLevelBonusUpsert(
                x.LookupId,
                NormalizeDecimal(x.Str),
                NormalizeDecimal(x.Vit),
                NormalizeDecimal(x.Dex),
                NormalizeDecimal(x.Agi),
                NormalizeDecimal(x.Int),
                NormalizeDecimal(x.Men),
                NormalizeDecimal(x.Luk)))
            .ToList();

        var fieldChanges = BuildFieldChanges();
        Result = new StatScalingEditResult(statContext, bonusUpdates, fieldChanges);
        DialogResult = true;
        Close();
    }

    private bool IsBaseChanged()
    {
        return _baseRows.Any(x => x.IsDirty);
    }

    private IReadOnlyList<PendingFieldChange> BuildFieldChanges()
    {
        var output = new List<PendingFieldChange>();
        var statIdKey = _original.StatId.ToString(CultureInfo.InvariantCulture);

        foreach (var row in _baseRows.Where(x => x.IsDirty))
        {
            output.Add(new PendingFieldChange(
                "StatResource",
                statIdKey,
                row.StatName.ToLowerInvariant(),
                row.OriginalValue,
                row.CurrentValue));
        }

        foreach (var row in _bonusRows.Where(x => x.IsDirty))
        {
            var key = row.LookupId.ToString(CultureInfo.InvariantCulture);
            AddDecimalChange(output, key, "str", row.OriginalStr, row.Str);
            AddDecimalChange(output, key, "vit", row.OriginalVit, row.Vit);
            AddDecimalChange(output, key, "dex", row.OriginalDex, row.Dex);
            AddDecimalChange(output, key, "agi", row.OriginalAgi, row.Agi);
            AddDecimalChange(output, key, "int", row.OriginalInt, row.Int);
            AddDecimalChange(output, key, "men", row.OriginalMen, row.Men);
            AddDecimalChange(output, key, "luk", row.OriginalLuk, row.Luk);
        }

        return output;
    }

    private static void AddDecimalChange(List<PendingFieldChange> output, string key, string field, decimal before, decimal after)
    {
        var normalizedBefore = NormalizeDecimal(before);
        var normalizedAfter = NormalizeDecimal(after);
        if (normalizedBefore == normalizedAfter)
        {
            return;
        }

        output.Add(new PendingFieldChange(
            "CreatureLevelBonus",
            key,
            field,
            normalizedBefore,
            normalizedAfter));
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}
