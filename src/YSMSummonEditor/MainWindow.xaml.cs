using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using System.Text.RegularExpressions;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Microsoft.Win32;
using YSMLib.AppSettings;
using YSMLib.AppSettings.Models;
using YSMLib.AssetIndexing;
using YSMLib.ChangeTracking;
using YSMLib.ChangeTracking.Models;
using YSMLib.GameData.Abstractions;
using YSMLib.GameData.Abstractions.Models;
using YSMLib.GameData.Abstractions.Services;
using YSMLib.GameData.SqlServer;
using YSMLib.GameData.SqlServer.Infrastructure;
using YSMSummonEditor.Core.Services;
using YSMSummonEditor.Helpers;
using YSMSummonEditor.Models;
using YSMSummonEditor.Services;
using YSMSummonEditor.Windows;
using AppDialogResult = YSMSummonEditor.Services.DialogResult;

namespace YSMSummonEditor;

public partial class MainWindow : Window
{
    private enum SaveRequestOrigin
    {
        Toolbar,
        SummonSwitch
    }

    private readonly ISettingsStore _settingsStore;
    private readonly IAssetResolver _assetResolver;
    private readonly SqlConnectionFactory _connectionFactory;
    private readonly ISkillUpdateBuilder _skillUpdateBuilder;
    private readonly IStatScalingCalculator _statScalingCalculator;
    private readonly IAppDialogService _dialogService;

    private readonly ObservableCollection<SummonDisplayItem> _summonItems = [];
    private readonly ObservableCollection<EffectiveStageSkillRow> _effectiveEvo1Rows = [];
    private readonly ObservableCollection<EffectiveStageSkillRow> _effectiveEvo2Rows = [];
    private readonly ObservableCollection<EffectiveStageSkillRow> _effectiveEvo3Rows = [];
    private readonly List<EvolutionViewState> _stageStates = [];

    private readonly Dictionary<int, SkillUpdate> _pendingSkillUpdates = new();
    private readonly List<PendingFieldChange> _pendingFieldChanges = [];
    private readonly Dictionary<int, StatResourceContext> _pendingStatContexts = new();
    private readonly Dictionary<int, CreatureLevelBonusUpsert> _pendingCreatureLevelBonusUpdates = new();
    private readonly Dictionary<int, RandomSkillGroupUpsert> _pendingRandomSkillGroupUpdates = new();
    private readonly List<PendingFieldChange> _pendingScalingFieldChanges = [];
    private readonly Dictionary<int, SkillDisplaySnapshot> _skillDisplay = new();
    private readonly Dictionary<int, RandomSkillGroupEditModel> _randomSkillGroups = new();

    private SummonEditorSettings _settings = new();
    private ISummonEditorRepository? _repository;
    private EvolutionChainSnapshot? _currentChain;
    private string _lastDiffSql = string.Empty;
    private bool _isInitializingSaveModeToggle;
    private bool _isRestoringSelection;
    private SummonDisplayItem? _activeSummon;
    private bool _isLoadingChainForSelection;
    private readonly System.Windows.Media.Imaging.BitmapImage? _fallbackIcon36;

    public MainWindow(
        ISettingsStore settingsStore,
        IAssetResolver assetResolver,
        SqlConnectionFactory connectionFactory,
        ISkillUpdateBuilder skillUpdateBuilder,
        IStatScalingCalculator statScalingCalculator,
        IAppDialogService dialogService)
    {
        _settingsStore = settingsStore;
        _assetResolver = assetResolver;
        _connectionFactory = connectionFactory;
        _skillUpdateBuilder = skillUpdateBuilder;
        _statScalingCalculator = statScalingCalculator;
        _dialogService = dialogService;

        InitializeComponent();
        _fallbackIcon36 = LoadEmbeddedImage("pack://application:,,,/Assets/common_mark_icon_unknown_middle.jpg");

        ListSummons.ItemsSource = _summonItems;
        GridEffective.ItemsSource = _effectiveEvo1Rows;

        Loaded += MainWindow_Loaded;
    }

    private async void MainWindow_Loaded(object sender, RoutedEventArgs e)
    {
        await LoadSettingsAsync();
        await BuildAssetIndexAsync();
        await LoadSummonsAsync();
    }

    private async Task LoadSettingsAsync()
    {
        _settings = await _settingsStore.LoadAsync();
        if (_settings.Database.Port <= 0)
        {
            _settings.Database.Port = 1433;
        }

        if (!IsDatabaseConfigured(_settings.Database)
            && LocalEnvConnectionLoader.TryLoadDbSettings(out var localDb, out var localEnvPath))
        {
            _settings.Database = localDb;
            TxtStatus.Text = $"Loaded DB defaults from {Path.GetFileName(localEnvPath)}.";
        }

        var defaultDumpApplied = await EnsureDefaultDumpFolderAsync();
        if (defaultDumpApplied)
        {
            const string dumpDefaultedMessage = "Dump folder defaulted to Desktop\\Dump.";
            TxtStatus.Text = string.IsNullOrWhiteSpace(TxtStatus.Text)
                ? dumpDefaultedMessage
                : $"{TxtStatus.Text} {dumpDefaultedMessage}";
        }

        ApplySaveModeToggleState(_settings.SaveToSqlMode);

        EnsureRepository();
        if (_repository is null)
        {
            const string dbNotConfiguredMessage = "Database is not configured. Add .env.local or set values in Settings.";
            TxtStatus.Text = string.IsNullOrWhiteSpace(TxtStatus.Text)
                ? dbNotConfiguredMessage
                : $"{TxtStatus.Text} {dbNotConfiguredMessage}";
        }

        UpdateStatsButtonState();
    }

    private void ApplySaveModeToggleState(bool saveToSqlMode)
    {
        _isInitializingSaveModeToggle = true;
        TglSaveMode.IsChecked = saveToSqlMode;
        TglSaveMode.Content = saveToSqlMode ? "Mode: .sql" : "Mode: DB";
        _isInitializingSaveModeToggle = false;
    }

    private async Task<bool> EnsureDefaultDumpFolderAsync()
    {
        if (!string.IsNullOrWhiteSpace(_settings.DumpFolderPath))
        {
            return false;
        }

        var desktopDumpPath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
            "Dump");

        Directory.CreateDirectory(desktopDumpPath);
        _settings.DumpFolderPath = desktopDumpPath;
        await _settingsStore.SaveAsync(_settings);
        return true;
    }

    private async Task BuildAssetIndexAsync()
    {
        if (string.IsNullOrWhiteSpace(_settings.DumpFolderPath))
        {
            if (string.IsNullOrWhiteSpace(TxtStatus.Text))
            {
                TxtStatus.Text = "Dump folder is not configured.";
            }

            return;
        }

        await _assetResolver.RebuildIndexAsync(_settings.DumpFolderPath);
        TxtStatus.Text = "Asset index loaded.";
    }

    private void EnsureRepository()
    {
        if (!IsDatabaseConfigured(_settings.Database))
        {
            _repository = null;
            return;
        }

        _repository = new SqlServerSummonEditorRepository(_settings.Database.ToConnectionString(), _connectionFactory);
    }

    private static bool IsDatabaseConfigured(DbConnectionSettings settings)
    {
        return !string.IsNullOrWhiteSpace(settings.Server)
            && !string.IsNullOrWhiteSpace(settings.Database)
            && !string.IsNullOrWhiteSpace(settings.UserId);
    }

    private async void BtnSearch_Click(object sender, RoutedEventArgs e)
    {
        await LoadSummonsAsync();
    }

    private async void TxtSearch_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter)
        {
            return;
        }

        e.Handled = true;
        await LoadSummonsAsync();
    }

    private async Task LoadSummonsAsync()
    {
        if (_repository is null)
        {
            EnsureRepository();
        }

        if (_repository is null)
        {
            _dialogService.Show(this, "Database", "Configure database connection first.");
            TxtStatus.Text = "Search skipped. Database is not configured.";
            return;
        }

        try
        {
            TxtStatus.Text = "Loading summons...";
            _summonItems.Clear();

            var filter = new SummonListFilter(
                TxtSearch.Text.Trim(),
                IncludeRidingOnly: false,
                IncludeNonForm1: true,
                ShowAllEvolutions: true);

            var summons = await _repository.SearchSummonsAsync(filter);
            foreach (var summon in summons)
            {
                _summonItems.Add(new SummonDisplayItem(summon)
                {
                    Icon = Resolve36IconOrFallback(_assetResolver.ResolveSummonFace(summon.FaceFileName))
                });
            }

            TxtStatus.Text = $"Loaded {_summonItems.Count.ToString("N0", CultureInfo.InvariantCulture)} summons.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Search failed", "Search query failed.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Search failed (see details).";
        }
    }

    private async void ListSummons_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_isRestoringSelection || _isLoadingChainForSelection)
        {
            return;
        }

        if (ListSummons.SelectedItem is not SummonDisplayItem selected)
        {
            _currentChain = null;
            _activeSummon = null;
            UpdateStatsButtonState();
            return;
        }

        var switchingSummon = _activeSummon is not null
            && selected.Summon.SummonId != _activeSummon.Summon.SummonId;

        if (switchingSummon && HasStagedChanges())
        {
            var choice = _dialogService.Show(
                this,
                "Unsaved changes",
                "Current summon has staged changes.\nYes = Save\nNo = Discard\nCancel = Stay on current summon.",
                DialogButtons.YesNoCancel,
                defaultResult: AppDialogResult.Cancel);

            if (choice == AppDialogResult.Cancel)
            {
                RestoreSummonSelection(_activeSummon);
                TxtStatus.Text = "Summon switch cancelled.";
                return;
            }

            if (choice == AppDialogResult.Yes)
            {
                var saveSucceeded = await SaveStagedChangesAsync(SaveRequestOrigin.SummonSwitch);
                if (!saveSucceeded)
                {
                    RestoreSummonSelection(_activeSummon);
                    return;
                }
            }
            else
            {
                ClearStagedChanges();
                TxtStatus.Text = "Staged changes discarded.";
            }
        }

        _isLoadingChainForSelection = true;
        try
        {
            await LoadEvolutionChainAndTreesAsync(selected.Summon);
            _activeSummon = selected;
        }
        finally
        {
            _isLoadingChainForSelection = false;
        }
    }

    private async Task LoadEvolutionChainAndTreesAsync(SummonListItem selected)
    {
        if (_repository is null)
        {
            return;
        }

        try
        {
            TxtStatus.Text = "Loading evolution chain...";
            _stageStates.Clear();
            _skillDisplay.Clear();
            _randomSkillGroups.Clear();

            var final = await ResolveFinalEvolutionAsync(selected);
            var sameCard = await _repository.GetSummonsByCardIdAsync(final.CardId);
            _currentChain = EvolutionChainResolver.ResolveFixedThreeStageChain(final, sameCard);

            await BuildStageStateAsync(1, _currentChain.Stage1);
            await BuildStageStateAsync(2, _currentChain.Stage2);
            await BuildStageStateAsync(3, _currentChain.Stage3);

            await PopulateSkillDisplayAsync();
            await LoadRandomSkillGroupsForVisibleRowsAsync();
            BuildEffectiveStageRows();
            ApplyRandomGroupDisplayToRows();
            BindStageTabs();
            UpdateHeader(final);
            UpdateStatsButtonState();

            TxtStatus.Text = "Chain loaded.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Chain load failed", "Could not load evolution chain.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Chain load failed (see details).";
            UpdateStatsButtonState();
        }
    }

    private async Task<SummonListItem> ResolveFinalEvolutionAsync(SummonListItem selected)
    {
        if (_repository is null)
        {
            throw new InvalidOperationException("Repository not initialized.");
        }

        var current = selected;
        var guard = 0;
        while (current.EvolveTarget != 0)
        {
            guard++;
            if (guard > 8)
            {
                throw new InvalidOperationException("Evolution chain appears invalid (loop).");
            }

            var next = await _repository.GetSummonByIdAsync(current.EvolveTarget);
            if (next is null)
            {
                throw new InvalidOperationException($"Could not resolve evolve_target {current.EvolveTarget}.");
            }

            current = next;
        }

        return current;
    }

    private async Task BuildStageStateAsync(int stageIndex, SummonListItem? summon)
    {
        var trees = new List<TreeContextState>();
        if (summon is not null)
        {
            var ids = new[] { summon.SkillTree1Id, summon.SkillTree2Id, summon.SkillTree3Id, summon.SkillTree4Id, summon.SkillTree5Id };
            for (var slot = 1; slot <= 5; slot++)
            {
                var id = ids[slot - 1] ?? 0;
                IReadOnlyList<SkillTreeEntrySnapshot> entries = [];
                if (_repository is not null && id > 0)
                {
                    entries = await _repository.GetSkillTreeEntriesAsync(id);
                }

                trees.Add(new TreeContextState(stageIndex, slot, summon.SummonId, id, entries));
            }
        }
        else
        {
            for (var slot = 1; slot <= 5; slot++)
            {
                trees.Add(new TreeContextState(stageIndex, slot, 0, 0, []));
            }
        }

        _stageStates.Add(new EvolutionViewState
        {
            StageIndex = stageIndex,
            Name = $"Evolution {stageIndex}",
            Trees = trees
        });
    }

    private async Task PopulateSkillDisplayAsync()
    {
        if (_repository is null)
        {
            return;
        }

        var skillIds = _stageStates
            .SelectMany(x => x.Trees)
            .SelectMany(x => x.CurrentEntries)
            .Select(x => x.SkillId)
            .Where(x => x > 0)
            .Distinct()
            .ToArray();

        if (skillIds.Length == 0)
        {
            return;
        }

        var displayRows = await _repository.GetSkillDisplayAsync(skillIds);
        foreach (var pair in displayRows)
        {
            _skillDisplay[pair.Key] = pair.Value;
        }

        foreach (var row in _stageStates.SelectMany(x => x.Trees).SelectMany(x => x.CurrentEntries))
        {
            if (_skillDisplay.TryGetValue(row.SkillId, out var display))
            {
                row.SkillName = display.SkillName;
                row.SkillTooltipMarkup = display.TooltipMarkup ?? string.Empty;
                row.SkillTooltipPlain = BuildSkillTooltipPlain(display.TooltipMarkup, row.SkillId, display.SkillName);
                row.SkillIcon = Resolve36IconOrFallback(_assetResolver.ResolveSkillIcon(display.IconFileName));
            }
            else
            {
                row.SkillName = $"Skill {row.SkillId}";
                row.SkillTooltipMarkup = string.Empty;
                row.SkillTooltipPlain = $"Skill {row.SkillId}";
                row.SkillIcon = _fallbackIcon36;
            }
        }
    }

    private async Task EnsureSkillDisplayAsync(IReadOnlyCollection<int> skillIds)
    {
        if (_repository is null || skillIds.Count == 0)
        {
            return;
        }

        var missing = skillIds
            .Where(x => x > 0 && !_skillDisplay.ContainsKey(x))
            .Distinct()
            .ToArray();
        if (missing.Length == 0)
        {
            return;
        }

        var fetched = await _repository.GetSkillDisplayAsync(missing);
        foreach (var pair in fetched)
        {
            _skillDisplay[pair.Key] = pair.Value;
        }
    }

    private async Task LoadRandomSkillGroupsForVisibleRowsAsync()
    {
        if (_repository is null)
        {
            return;
        }

        var groupIds = _stageStates
            .SelectMany(x => x.Trees)
            .SelectMany(x => x.CurrentEntries)
            .Select(x => x.SkillGroupId)
            .Where(x => x > 0)
            .Distinct()
            .ToArray();

        if (groupIds.Length == 0)
        {
            return;
        }

        var toLoad = groupIds
            .Where(id => !_pendingRandomSkillGroupUpdates.ContainsKey(id) && !_randomSkillGroups.ContainsKey(id))
            .ToArray();

        if (toLoad.Length > 0)
        {
            var loaded = await _repository.GetRandomSkillGroupsAsync(toLoad);
            foreach (var pair in loaded)
            {
                _randomSkillGroups[pair.Key] = ToEditModel(pair.Value);
            }
        }

        foreach (var groupId in groupIds)
        {
            if (!_pendingRandomSkillGroupUpdates.ContainsKey(groupId) && !_randomSkillGroups.ContainsKey(groupId))
            {
                _randomSkillGroups[groupId] = RandomSkillGroupEditModel.Empty(groupId);
            }
        }

        var candidateSkillIds = groupIds
            .Select(GetOverlayRandomGroup)
            .Where(x => x is not null)
            .SelectMany(x => x!.SkillIds)
            .Where(x => x > 0)
            .Distinct()
            .ToArray();

        await EnsureSkillDisplayAsync(candidateSkillIds);
    }

    private RandomSkillGroupEditModel? GetOverlayRandomGroup(int groupId)
    {
        if (groupId <= 0)
        {
            return null;
        }

        if (_pendingRandomSkillGroupUpdates.TryGetValue(groupId, out var pending))
        {
            return ToEditModel(pending);
        }

        return _randomSkillGroups.GetValueOrDefault(groupId);
    }

    private void ApplyRandomGroupDisplayToRows()
    {
        foreach (var row in _stageStates.SelectMany(x => x.Trees).SelectMany(x => x.CurrentEntries))
        {
            var groupId = row.SkillGroupId;
            if (groupId <= 0)
            {
                row.RandomGroupId = 0;
                row.IsRandomGroup = false;
                row.RandomGroupBadge = string.Empty;
                row.RandomGroupTooltip = string.Empty;
                row.RandomGroupState = string.Empty;
                continue;
            }

            var group = GetOverlayRandomGroup(groupId);
            row.RandomGroupId = groupId;
            row.IsRandomGroup = true;
            row.RandomGroupBadge = $"RND[{groupId}]";
            row.RandomGroupTooltip = BuildRandomGroupTooltip(groupId, group);
            row.RandomGroupState = ResolveRandomGroupState(groupId);
        }

        ApplyRandomGroupDisplayToEffectiveRows(_effectiveEvo1Rows);
        ApplyRandomGroupDisplayToEffectiveRows(_effectiveEvo2Rows);
        ApplyRandomGroupDisplayToEffectiveRows(_effectiveEvo3Rows);
    }

    private void ApplyRandomGroupDisplayToEffectiveRows(IEnumerable<EffectiveStageSkillRow> rows)
    {
        foreach (var row in rows)
        {
            var groupIds = row.LinkedEntries
                .Select(x => x.SkillGroupId)
                .Where(x => x > 0)
                .Distinct()
                .OrderBy(x => x)
                .ToArray();

            if (groupIds.Length == 0)
            {
                row.IsRandomGroup = false;
                row.RandomGroupBadge = string.Empty;
                row.RandomGroupTooltip = string.Empty;
                continue;
            }

            row.IsRandomGroup = true;
            row.RandomGroupBadge = groupIds.Length == 1
                ? $"RND[{groupIds[0]}]"
                : $"RND[{string.Join(",", groupIds.Select(x => x.ToString(CultureInfo.InvariantCulture)))}]";

            var tooltipParts = groupIds
                .Select(x => BuildRandomGroupTooltip(x, GetOverlayRandomGroup(x)))
                .Distinct(StringComparer.Ordinal)
                .ToArray();
            row.RandomGroupTooltip = string.Join("\n\n", tooltipParts);
        }
    }

    private string ResolveRandomGroupState(int groupId)
    {
        if (_pendingRandomSkillGroupUpdates.ContainsKey(groupId))
        {
            return "Pending";
        }

        return _randomSkillGroups.ContainsKey(groupId)
            ? "Loaded"
            : "Missing";
    }

    private string BuildRandomGroupTooltip(int groupId, RandomSkillGroupEditModel? group)
    {
        if (group is null)
        {
            return $"Random group {groupId}\nStatus: missing";
        }

        var builder = new StringBuilder();
        builder.AppendLine($"Random group: {groupId}");
        builder.AppendLine($"UseCode: {group.UseCode}");

        if (group.SkillIds.Count == 0)
        {
            builder.Append("Candidates: <empty>");
            return builder.ToString();
        }

        builder.AppendLine("Candidates:");
        foreach (var skillId in group.SkillIds)
        {
            var label = _skillDisplay.TryGetValue(skillId, out var display)
                ? display.SkillName
                : $"Skill {skillId}";
            builder.Append(" - ");
            builder.Append(skillId.ToString(CultureInfo.InvariantCulture));
            builder.Append(": ");
            builder.AppendLine(label);
        }

        return builder.ToString().TrimEnd();
    }

    private static RandomSkillGroupEditModel ToEditModel(RandomSkillGroupSnapshot snapshot)
    {
        var ids = snapshot.SkillIds
            .Where(x => x > 0)
            .Distinct()
            .OrderBy(x => x)
            .Take(15)
            .ToArray();
        return new RandomSkillGroupEditModel(snapshot.GroupId, ids.Length, ids);
    }

    private static RandomSkillGroupEditModel ToEditModel(RandomSkillGroupUpsert upsert)
    {
        var ids = new[]
        {
            upsert.SkillId1, upsert.SkillId2, upsert.SkillId3, upsert.SkillId4, upsert.SkillId5,
            upsert.SkillId6, upsert.SkillId7, upsert.SkillId8, upsert.SkillId9, upsert.SkillId10,
            upsert.SkillId11, upsert.SkillId12, upsert.SkillId13, upsert.SkillId14, upsert.SkillId15
        }
        .Where(x => x > 0)
        .Distinct()
        .OrderBy(x => x)
        .Take(15)
        .ToArray();

        return new RandomSkillGroupEditModel(upsert.GroupId, ids.Length, ids);
    }

    private static RandomSkillGroupUpsert ToRandomSkillGroupUpsert(RandomSkillGroupEditModel edit)
    {
        var ids = edit.SkillIds
            .Where(x => x > 0)
            .Distinct()
            .OrderBy(x => x)
            .Take(15)
            .ToArray();

        var slots = new int[15];
        for (var i = 0; i < ids.Length; i++)
        {
            slots[i] = ids[i];
        }

        return new RandomSkillGroupUpsert(
            edit.GroupId,
            ids.Length,
            slots[0], slots[1], slots[2], slots[3], slots[4],
            slots[5], slots[6], slots[7], slots[8], slots[9],
            slots[10], slots[11], slots[12], slots[13], slots[14]);
    }

    private static string BuildSkillTooltipPlain(string? markup, int skillId, string skillName)
    {
        if (string.IsNullOrWhiteSpace(markup))
        {
            return $"Skill {skillId} | {skillName}";
        }

        var plain = Regex.Replace(markup, "<[^>]+>", string.Empty, RegexOptions.CultureInvariant);
        plain = plain.Replace("\r", string.Empty).Trim();
        return string.IsNullOrWhiteSpace(plain)
            ? $"Skill {skillId} | {skillName}"
            : plain;
    }

    private void BuildEffectiveStageRows()
    {
        BuildEffectiveRowsForStage(1, _effectiveEvo1Rows);
        BuildEffectiveRowsForStage(2, _effectiveEvo2Rows);
        BuildEffectiveRowsForStage(3, _effectiveEvo3Rows);
    }

    private void BuildEffectiveRowsForStage(int stageIndex, ObservableCollection<EffectiveStageSkillRow> target)
    {
        target.Clear();
        var stage = _stageStates.FirstOrDefault(x => x.StageIndex == stageIndex);
        if (stage is null)
        {
            return;
        }

        var map = new Dictionary<string, EffectiveStageSkillRow>(StringComparer.Ordinal);
        foreach (var tree in stage.Trees)
        {
            foreach (var entry in tree.CurrentEntries)
            {
                if (entry.SkillId <= 0)
                {
                    continue;
                }

                var key = $"{entry.SkillId}:{entry.Lv}";
                if (!map.TryGetValue(key, out var row))
                {
                    row = new EffectiveStageSkillRow(entry.SkillId, entry.Lv)
                    {
                        SkillName = entry.SkillName,
                        SkillIcon = entry.SkillIcon,
                        SkillTooltipMarkup = entry.SkillTooltipMarkup,
                        SkillTooltipPlain = entry.SkillTooltipPlain,
                        MinSkillLv = entry.MinSkillLv,
                        MaxSkillLv = entry.MaxSkillLv,
                        IsDirty = entry.IsDirty
                    };
                    map[key] = row;
                }
                else
                {
                    row.MinSkillLv = Math.Min(row.MinSkillLv, entry.MinSkillLv);
                    row.MaxSkillLv = Math.Max(row.MaxSkillLv, entry.MaxSkillLv);
                    row.IsDirty = row.IsDirty || entry.IsDirty;
                }

                row.LinkedEntries.Add(entry);
            }
        }

        foreach (var row in map.Values.OrderBy(x => x.SkillId).ThenBy(x => x.Level))
        {
            target.Add(row);
        }
    }

    private void BindStageTabs()
    {
        TabStages.Items.Clear();
        foreach (var stage in _stageStates)
        {
            TabStages.Items.Add(new TabItem
            {
                Header = stage.Name,
                Tag = stage,
                ToolTip = $"Skills shown for {stage.Name}"
            });
        }

        if (TabStages.Items.Count > 0)
        {
            TabStages.SelectedIndex = 0;
        }
    }

    private void TabStages_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (TabStages.SelectedItem is not TabItem item || item.Tag is not EvolutionViewState stage)
        {
            return;
        }

        TabTrees.ItemsSource = stage.Trees;
        if (stage.Trees.Count > 0)
        {
            TabTrees.SelectedIndex = 0;
        }

        UpdateEffectiveGridForStage(stage.StageIndex);
    }

    private void UpdateEffectiveGridForStage(int stageIndex)
    {
        GridEffective.ItemsSource = stageIndex switch
        {
            1 => _effectiveEvo1Rows,
            2 => _effectiveEvo2Rows,
            3 => _effectiveEvo3Rows,
            _ => null
        };
    }

    private void UpdateHeader(SummonListItem final)
    {
        TxtSummonHeader.Text = final.SummonName;
        TxtSummonHeader.ToolTip = $"{final.SummonName} ({final.SummonId})";
        var featureText = final.FeatureText?.Trim();
        TxtSummonFeature.Text = featureText ?? string.Empty;
        TxtSummonFeature.ToolTip = string.IsNullOrWhiteSpace(featureText) ? null : featureText;
        TxtSummonFeature.Visibility = string.IsNullOrWhiteSpace(featureText) ? Visibility.Collapsed : Visibility.Visible;
        ImgSummonHeaderIcon.Source = Resolve36IconOrFallback(_assetResolver.ResolveSummonFace(final.FaceFileName));
        TxtSummonMeta.Text = $"ID: {final.SummonId} | Card: {final.CardId}";
        TxtSummonMeta.ToolTip = $"Card: {final.CardName ?? "Unknown"} ({final.CardId})";
        UpdateChainVisual(_currentChain);

        var large = _assetResolver.ResolveSummonIllust(final.IllustFileName);
        ImgLarge.Source = ImageLoader.TryLoad(large);
    }

    private void UpdateStatsButtonState()
    {
        BtnStats.IsEnabled = _repository is not null && _currentChain?.Stage3 is not null;
    }

    private void UpdateChainVisual(EvolutionChainSnapshot? chain)
    {
        UpdateChainStage(chain?.Stage1, ChainEvo1Panel, ImgChainEvo1, TxtChainEvo1Id);
        UpdateChainStage(chain?.Stage2, ChainEvo2Panel, ImgChainEvo2, TxtChainEvo2Id);
        UpdateChainStage(chain?.Stage3, ChainEvo3Panel, ImgChainEvo3, TxtChainEvo3Id);

        TxtChainArrow12.Visibility = chain is { Stage1: not null, Stage2: not null }
            ? Visibility.Visible
            : Visibility.Collapsed;
        TxtChainArrow23.Visibility = chain is { Stage2: not null, Stage3: not null }
            ? Visibility.Visible
            : Visibility.Collapsed;
    }

    private void UpdateChainStage(
        SummonListItem? stage,
        FrameworkElement stageContainer,
        Image stageIcon,
        TextBlock stageIdText)
    {
        if (stage is null)
        {
            stageContainer.Visibility = Visibility.Collapsed;
            stageIcon.Source = null;
            stageIdText.Text = string.Empty;
            return;
        }

        stageContainer.Visibility = Visibility.Visible;
        stageIcon.Source = Resolve36IconOrFallback(_assetResolver.ResolveSummonFace(stage.FaceFileName));
        stageIdText.Text = stage.SummonId.ToString(CultureInfo.InvariantCulture);
    }

    private System.Windows.Media.Imaging.BitmapImage? Resolve36IconOrFallback(string? path)
    {
        return ImageLoader.TryLoad(path) ?? _fallbackIcon36;
    }

    private static System.Windows.Media.Imaging.BitmapImage? LoadEmbeddedImage(string packUri)
    {
        try
        {
            var image = new System.Windows.Media.Imaging.BitmapImage();
            image.BeginInit();
            image.CacheOption = System.Windows.Media.Imaging.BitmapCacheOption.OnLoad;
            image.UriSource = new Uri(packUri, UriKind.Absolute);
            image.EndInit();
            image.Freeze();
            return image;
        }
        catch
        {
            return null;
        }
    }

    private TreeContextState? GetSelectedTree() => TabTrees.SelectedItem as TreeContextState;

    private async Task AddTreeEntryAsync(TreeContextState tree)
    {
        if (tree.OriginalSkillTreeId <= 0)
        {
            return;
        }

        var nextLv = tree.CurrentEntries.Count == 0 ? 1 : tree.CurrentEntries.Max(x => x.Lv) + 1;
        var entry = new SkillTreeEntrySnapshot(tree.OriginalSkillTreeId, 0, 0, 1, 1, nextLv, 1, 0, 0, 0, 0, 0, 0, 0, 0);
        tree.CurrentEntries.Add(new SkillTreeEntryRow(entry, isNew: true)
        {
            SkillName = "New skill"
        });

        await RefreshSkillNamesAndEffectiveAsync();
    }

    private async Task DeleteTreeEntryAsync(TreeContextState tree, SkillTreeEntryRow row)
    {
        var confirm = _dialogService.Show(
            this,
            "Confirm delete",
            $"Delete skill {row.SkillId} from Evolution {tree.StageIndex}, Tree {tree.TreeSlotIndex}?",
            DialogButtons.YesNo,
            defaultResult: AppDialogResult.No);

        if (confirm != AppDialogResult.Yes)
        {
            return;
        }

        tree.CurrentEntries.Remove(row);
        await RefreshSkillNamesAndEffectiveAsync();
    }

    private void GridTree_PreviewMouseRightButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not DataGrid grid)
        {
            return;
        }

        var row = FindVisualParent<DataGridRow>(e.OriginalSource as DependencyObject);
        if (row?.Item is not SkillTreeEntryRow item)
        {
            return;
        }

        grid.SelectedItem = item;
        row.IsSelected = true;
    }

    private async void GridTree_CellEditEnding(object sender, DataGridCellEditEndingEventArgs e)
    {
        if (e.EditAction != DataGridEditAction.Commit)
        {
            return;
        }

        if (e.Row.Item is SkillTreeEntryRow entry
            && e.Column.Header is string header
            && string.Equals(header, "Grp", StringComparison.OrdinalIgnoreCase))
        {
            await EnsurePlaceholderGroupForManualGroupIdAsync(entry);
        }

        await RefreshSkillNamesAndEffectiveAsync();
    }

    private void TreeContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        if (sender is not ContextMenu menu)
        {
            return;
        }

        var tree = GetSelectedTree();
        var row = tree?.SelectedEntry;

        if (menu.Items.Count > 0 && menu.Items[0] is MenuItem add)
        {
            add.IsEnabled = tree is { OriginalSkillTreeId: > 0 };
        }

        if (menu.Items.Count > 1 && menu.Items[1] is MenuItem editSkill)
        {
            editSkill.IsEnabled = row is { SkillId: > 0 };
        }

        if (menu.Items.Count > 2 && menu.Items[2] is MenuItem delete)
        {
            delete.IsEnabled = row is not null;
        }

        if (menu.Items.Count > 4 && menu.Items[4] is MenuItem createGroup)
        {
            createGroup.IsEnabled = row is not null && row.SkillGroupId <= 0;
        }

        if (menu.Items.Count > 5 && menu.Items[5] is MenuItem editGroup)
        {
            editGroup.IsEnabled = row is not null && row.SkillGroupId > 0;
        }

        if (menu.Items.Count > 6 && menu.Items[6] is MenuItem detachGroup)
        {
            detachGroup.IsEnabled = row is not null && row.SkillGroupId > 0;
        }
    }

    private async void MnuTreeAddRow_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree is null)
        {
            return;
        }

        await AddTreeEntryAsync(tree);
    }

    private async void MnuTreeEditSkill_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree?.SelectedEntry is not { SkillId: > 0 } row)
        {
            return;
        }

        await OpenSkillEditDialogAsync(row.SkillId);
    }

    private async void MnuTreeDeleteRow_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree?.SelectedEntry is not SkillTreeEntryRow row)
        {
            return;
        }

        await DeleteTreeEntryAsync(tree, row);
    }

    private async void MnuTreeCreateRandomGroup_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree?.SelectedEntry is not SkillTreeEntryRow row)
        {
            return;
        }

        if (row.SkillGroupId > 0)
        {
            await EditRandomGroupForRowAsync(row, row.SkillGroupId, false);
            return;
        }

        var nextId = await GetNextAvailableRandomGroupIdAsync();
        var previous = row.SkillGroupId;
        row.SkillGroupId = nextId;
        await EnsurePlaceholderGroupForManualGroupIdAsync(row);

        var applied = await EditRandomGroupForRowAsync(row, nextId, true);
        if (!applied)
        {
            row.SkillGroupId = previous;
            _pendingRandomSkillGroupUpdates.Remove(nextId);
            _randomSkillGroups.Remove(nextId);
            await RefreshSkillNamesAndEffectiveAsync();
        }
    }

    private async void MnuTreeEditRandomGroup_Click(object sender, RoutedEventArgs e)
    {
        var row = GetSelectedTree()?.SelectedEntry;
        if (row is null || row.SkillGroupId <= 0)
        {
            return;
        }

        await EditRandomGroupForRowAsync(row, row.SkillGroupId, false);
    }

    private async void MnuTreeDetachRandomGroup_Click(object sender, RoutedEventArgs e)
    {
        var row = GetSelectedTree()?.SelectedEntry;
        if (row is null || row.SkillGroupId <= 0)
        {
            return;
        }

        var groupId = row.SkillGroupId;
        var choice = _dialogService.Show(
            this,
            "Detach random group",
            $"Detach random group {groupId} from this tree row?",
            DialogButtons.YesNo,
            defaultResult: AppDialogResult.No);

        if (choice != AppDialogResult.Yes)
        {
            return;
        }

        row.SkillGroupId = 0;
        await RefreshSkillNamesAndEffectiveAsync();
        TxtStatus.Text = $"Detached random group {groupId} from selected row.";
    }

    private async Task EnsurePlaceholderGroupForManualGroupIdAsync(SkillTreeEntryRow row)
    {
        if (row.SkillGroupId <= 0)
        {
            return;
        }

        if (_pendingRandomSkillGroupUpdates.ContainsKey(row.SkillGroupId) || _randomSkillGroups.ContainsKey(row.SkillGroupId))
        {
            return;
        }

        if (_repository is not null)
        {
            var loaded = await _repository.GetRandomSkillGroupsAsync([row.SkillGroupId]);
            if (loaded.TryGetValue(row.SkillGroupId, out var snapshot))
            {
                _randomSkillGroups[row.SkillGroupId] = ToEditModel(snapshot);
                await EnsureSkillDisplayAsync(snapshot.SkillIds.ToArray());
                ApplyRandomGroupDisplayToRows();
                return;
            }
        }

        var placeholder = RandomSkillGroupEditModel.Empty(row.SkillGroupId);
        _randomSkillGroups[row.SkillGroupId] = placeholder;
        _pendingRandomSkillGroupUpdates[row.SkillGroupId] = ToRandomSkillGroupUpsert(placeholder);
        ApplyRandomGroupDisplayToRows();
        TxtStatus.Text = $"Created placeholder random group {row.SkillGroupId}.";
    }

    private async Task<bool> EditRandomGroupForRowAsync(SkillTreeEntryRow row, int groupId, bool isCreateFlow)
    {
        if (_repository is null)
        {
            return false;
        }

        await EnsurePlaceholderGroupForManualGroupIdAsync(row);

        var model = GetOverlayRandomGroup(groupId) ?? RandomSkillGroupEditModel.Empty(groupId);
        var window = new RandomSkillGroupWindow(
            groupId,
            model,
            _repository,
            _assetResolver,
            _dialogService,
            _skillDisplay,
            _fallbackIcon36)
        {
            Owner = this
        };

        if (window.ShowDialog() != true || window.Result is null)
        {
            return !isCreateFlow;
        }

        var normalized = new RandomSkillGroupEditModel(
            groupId,
            window.Result.SkillIds.Where(x => x > 0).Distinct().OrderBy(x => x).Take(15).Count(),
            window.Result.SkillIds.Where(x => x > 0).Distinct().OrderBy(x => x).Take(15).ToArray());

        _randomSkillGroups[groupId] = normalized;
        _pendingRandomSkillGroupUpdates[groupId] = ToRandomSkillGroupUpsert(normalized);
        await EnsureSkillDisplayAsync(normalized.SkillIds.ToArray());

        row.SkillGroupId = groupId;
        await RefreshSkillNamesAndEffectiveAsync();
        TxtStatus.Text = $"Random group {groupId} staged (UseCode: {normalized.UseCode}).";
        return true;
    }

    private static (int SummonId, int TreeSlotIndex, int SkillTreeId) TreeContextKey(TreeContextState tree) =>
        (tree.SummonId, tree.TreeSlotIndex, tree.OriginalSkillTreeId);

    private void EnsureTreeContextsForRandomGroup(
        IDictionary<(int SummonId, int TreeSlotIndex, int SkillTreeId), SkillTreeContext> contextMap,
        IDictionary<(int SummonId, int TreeSlotIndex, int SkillTreeId), bool> cloneFlags,
        int groupId)
    {
        foreach (var tree in _stageStates.SelectMany(x => x.Trees))
        {
            if (tree.SummonId <= 0 || tree.OriginalSkillTreeId <= 0)
            {
                continue;
            }

            if (!tree.CurrentEntries.Any(x => x.SkillGroupId == groupId))
            {
                continue;
            }

            var key = TreeContextKey(tree);
            if (!contextMap.ContainsKey(key))
            {
                var snapshots = tree.CurrentEntries
                    .Select(x => x.ToSnapshot(tree.OriginalSkillTreeId))
                    .OrderBy(x => x.Lv)
                    .ThenBy(x => x.SkillId)
                    .ToList();

                contextMap[key] = new SkillTreeContext(
                    tree.SummonId,
                    tree.TreeSlotIndex,
                    tree.OriginalSkillTreeId,
                    snapshots,
                    CloneIfShared: false);
            }

            if (!cloneFlags.ContainsKey(key))
            {
                cloneFlags[key] = false;
            }
        }
    }

    private async Task<int> GetNextAvailableRandomGroupIdAsync(ISet<int>? extraReserved = null)
    {
        if (_repository is null)
        {
            return 1;
        }

        var next = await _repository.GetNextRandomSkillGroupIdAsync();
        var reserved = _pendingRandomSkillGroupUpdates.Keys
            .Concat(_randomSkillGroups.Keys)
            .Concat(_stageStates.SelectMany(x => x.Trees).SelectMany(x => x.CurrentEntries).Select(x => x.SkillGroupId))
            .Where(x => x > 0)
            .ToHashSet();

        if (extraReserved is not null)
        {
            reserved.UnionWith(extraReserved.Where(x => x > 0));
        }

        while (reserved.Contains(next))
        {
            next++;
        }

        return next;
    }

    private void GridEffective_PreviewMouseRightButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not DataGrid grid)
        {
            return;
        }

        var row = FindVisualParent<DataGridRow>(e.OriginalSource as DependencyObject);
        if (row?.Item is EffectiveStageSkillRow stageRow)
        {
            grid.SelectedItem = stageRow;
            row.IsSelected = true;
        }
    }

    private void EffectiveContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        if (sender is not ContextMenu menu)
        {
            return;
        }

        var (stage, row) = GetSelectedEffectiveRow();

        if (menu.Items.Count > 0 && menu.Items[0] is MenuItem edit)
        {
            edit.IsEnabled = row is { SkillId: > 0 };
        }

        if (menu.Items.Count > 1 && menu.Items[1] is MenuItem delete)
        {
            delete.IsEnabled = row is not null && stage > 0 && row.LinkedEntries.Count > 0;
        }
    }

    private int GetActiveStageIndexFromTabStages()
    {
        return TabStages.SelectedItem is TabItem item && item.Tag is EvolutionViewState stage
            ? stage.StageIndex
            : 0;
    }

    private (int Stage, EffectiveStageSkillRow? Row) GetSelectedEffectiveRow()
    {
        var stage = GetActiveStageIndexFromTabStages();
        return GridEffective.SelectedItem is EffectiveStageSkillRow row
            ? (stage, row)
            : (stage, null);
    }

    private async void MnuEffectiveEditSkill_Click(object sender, RoutedEventArgs e)
    {
        var (_, row) = GetSelectedEffectiveRow();
        if (row is null || row.SkillId <= 0)
        {
            return;
        }

        await OpenSkillEditDialogAsync(row.SkillId);
    }

    private async void MnuEffectiveDeleteRow_Click(object sender, RoutedEventArgs e)
    {
        var (stage, row) = GetSelectedEffectiveRow();
        if (row is null)
        {
            return;
        }

        if (stage <= 0 || row.LinkedEntries.Count == 0)
        {
            return;
        }

        var confirm = _dialogService.Show(
            this,
            "Confirm delete",
            $"Delete skill {row.SkillId} (Lv {row.Level}) from Evolution {stage}? This will remove {row.LinkedEntries.Count} row(s).",
            DialogButtons.YesNo,
            defaultResult: AppDialogResult.No);
        if (confirm != AppDialogResult.Yes)
        {
            return;
        }

        var stageState = _stageStates.FirstOrDefault(x => x.StageIndex == stage);
        if (stageState is null)
        {
            return;
        }

        foreach (var treeEntry in row.LinkedEntries.ToList())
        {
            foreach (var tree in stageState.Trees)
            {
                _ = tree.CurrentEntries.Remove(treeEntry);
            }
        }

        await RefreshSkillNamesAndEffectiveAsync();
    }

    private void GridEffective_CellEditEnding(object sender, DataGridCellEditEndingEventArgs e)
    {
        if (e.EditAction == DataGridEditAction.Commit)
        {
            var stage = GetActiveStageIndexFromTabStages();
            if (stage > 0 && e.Row.Item is EffectiveStageSkillRow row)
            {
                foreach (var linked in row.LinkedEntries)
                {
                    linked.MinSkillLv = row.MinSkillLv;
                    linked.MaxSkillLv = row.MaxSkillLv;
                }

                row.IsDirty = row.LinkedEntries.Any(x => x.IsDirty);
            }

            TxtStatus.Text = "Matrix change staged.";
        }
    }

    private async Task OpenSkillEditDialogAsync(int skillId)
    {
        if (_repository is null)
        {
            return;
        }

        try
        {
            var snapshot = await _repository.GetSkillEditableAsync(skillId);
            var window = new SkillEditWindow(snapshot, _skillUpdateBuilder, skillId)
            {
                Owner = this
            };

            if (window.ShowDialog() != true || window.ResultUpdate is null)
            {
                return;
            }

            if (window.ResultUpdate.UpdatedFields.Count == 0)
            {
                return;
            }

            _pendingSkillUpdates[skillId] = window.ResultUpdate;
            _pendingFieldChanges.RemoveAll(x => string.Equals(x.Key, skillId.ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal));
            _pendingFieldChanges.AddRange(window.ResultFieldChanges);
            TxtStatus.Text = $"Skill {skillId} staged for save.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Skill edit failed", "Could not open skill editor.", DialogButtons.Ok, ex.Message);
        }
    }

    private async void BtnStats_Click(object sender, RoutedEventArgs e)
    {
        if (_repository is null || _currentChain?.Stage3 is null)
        {
            UpdateStatsButtonState();
            return;
        }

        try
        {
            var finalSummon = _currentChain.Stage3;
            var snapshot = await _repository.GetSummonStatScalingAsync(finalSummon.SummonId);
            var creatureEnhance = await _repository.GetCreatureEnhanceAsync();
            snapshot = ApplyPendingScaling(snapshot);

            var window = new StatScalingWindow(snapshot, _statScalingCalculator, creatureEnhance)
            {
                Owner = this
            };

            if (window.ShowDialog() != true || window.Result is null)
            {
                return;
            }

            ApplyScalingEditResult(snapshot, window.Result);
            TxtStatus.Text = "Stat scaling changes staged.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Stat scaling", "Could not open stat scaling editor.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Stat scaling open failed (see details).";
        }
    }

    private SummonStatScalingSnapshot ApplyPendingScaling(SummonStatScalingSnapshot snapshot)
    {
        var baseStats = _pendingStatContexts.TryGetValue(snapshot.SummonId, out var statContext)
            ? statContext.Stats
            : snapshot.BaseStats;

        var stagedBonuses = snapshot.StageBonuses
            .Select(x =>
            {
                if (_pendingCreatureLevelBonusUpdates.TryGetValue(x.LookupId, out var pending))
                {
                    return x with
                    {
                        Str = pending.Str,
                        Vit = pending.Vit,
                        Dex = pending.Dex,
                        Agi = pending.Agi,
                        Int = pending.Int,
                        Men = pending.Men,
                        Luk = pending.Luk,
                        ExistsInDb = true
                    };
                }

                return x;
            })
            .ToList();

        return snapshot with { BaseStats = baseStats, StageBonuses = stagedBonuses };
    }

    private void ApplyScalingEditResult(SummonStatScalingSnapshot snapshot, StatScalingEditResult result)
    {
        if (result.StatContext is null)
        {
            _pendingStatContexts.Remove(snapshot.SummonId);
        }
        else
        {
            _pendingStatContexts[snapshot.SummonId] = result.StatContext;
        }

        var lookupIds = snapshot.StageBonuses.Select(x => x.LookupId).ToHashSet();
        foreach (var lookupId in lookupIds)
        {
            _pendingCreatureLevelBonusUpdates.Remove(lookupId);
        }

        foreach (var upsert in result.BonusUpdates)
        {
            _pendingCreatureLevelBonusUpdates[upsert.BonusId] = upsert;
        }

        RemoveScalingFieldChanges(snapshot);
        _pendingScalingFieldChanges.AddRange(result.FieldChanges);
    }

    private void RemoveScalingFieldChanges(SummonStatScalingSnapshot snapshot)
    {
        var statKey = snapshot.StatId.ToString(CultureInfo.InvariantCulture);
        var bonusKeys = snapshot.StageBonuses
            .Select(x => x.LookupId.ToString(CultureInfo.InvariantCulture))
            .ToHashSet(StringComparer.Ordinal);

        _pendingScalingFieldChanges.RemoveAll(x =>
            (string.Equals(x.Entity, "StatResource", StringComparison.Ordinal) && string.Equals(x.Key, statKey, StringComparison.Ordinal))
            || (string.Equals(x.Entity, "CreatureLevelBonus", StringComparison.Ordinal) && bonusKeys.Contains(x.Key)));
    }

    private async void TglSaveMode_Checked(object sender, RoutedEventArgs e)
    {
        await SetSaveModeAsync(true);
    }

    private async void TglSaveMode_Unchecked(object sender, RoutedEventArgs e)
    {
        await SetSaveModeAsync(false);
    }

    private async Task SetSaveModeAsync(bool saveToSqlMode)
    {
        TglSaveMode.Content = saveToSqlMode ? "Mode: .sql" : "Mode: DB";
        if (_isInitializingSaveModeToggle || _settings.SaveToSqlMode == saveToSqlMode)
        {
            return;
        }

        _settings.SaveToSqlMode = saveToSqlMode;
        await _settingsStore.SaveAsync(_settings);
        TxtStatus.Text = saveToSqlMode
            ? "Save mode set to .sql (staged export)."
            : "Save mode set to DB.";
    }

    private async void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        _ = await SaveStagedChangesAsync(SaveRequestOrigin.Toolbar);
    }

    private bool HasStagedChanges()
    {
        var commitRequest = BuildCommitRequest(out _);
        return commitRequest.TreeContexts.Count > 0
            || commitRequest.SkillUpdates.Count > 0
            || commitRequest.StatContexts.Count > 0
            || commitRequest.CreatureLevelBonusUpdates.Count > 0
            || commitRequest.RandomSkillGroupUpdates.Count > 0;
    }

    private void ClearStagedChanges()
    {
        _pendingSkillUpdates.Clear();
        _pendingFieldChanges.Clear();
        _pendingStatContexts.Clear();
        _pendingCreatureLevelBonusUpdates.Clear();
        _pendingRandomSkillGroupUpdates.Clear();
        _pendingScalingFieldChanges.Clear();
    }

    private void RestoreSummonSelection(SummonDisplayItem? selection)
    {
        _isRestoringSelection = true;
        try
        {
            ListSummons.SelectedItem = selection;
        }
        finally
        {
            _isRestoringSelection = false;
        }
    }

    private async Task<bool> SaveStagedChangesAsync(SaveRequestOrigin origin)
    {
        if (_repository is null)
        {
            return false;
        }

        try
        {
            var commitRequest = BuildCommitRequest(out var treeReviewChanges);
            if (commitRequest.TreeContexts.Count == 0
                && commitRequest.SkillUpdates.Count == 0
                && commitRequest.StatContexts.Count == 0
                && commitRequest.CreatureLevelBonusUpdates.Count == 0
                && commitRequest.RandomSkillGroupUpdates.Count == 0)
            {
                if (origin == SaveRequestOrigin.Toolbar)
                {
                    _dialogService.Show(this, "Save", "No changes to save.");
                }

                return true;
            }

            var adjustedRequest = await ApplyWarningsAndCloneFlagsAsync(commitRequest);
            if (adjustedRequest is null)
            {
                return false;
            }

            var allFieldChanges = _pendingFieldChanges
                .Concat(_pendingScalingFieldChanges)
                .ToList();
            var reviewRows = BuildReviewRows(treeReviewChanges, allFieldChanges, adjustedRequest.RandomSkillGroupUpdates);
            var reviewWindow = new SaveReviewWindow(reviewRows) { Owner = this };
            if (reviewWindow.ShowDialog() != true)
            {
                return false;
            }

            if (_settings.SaveToSqlMode)
            {
                var saveDialog = new SaveFileDialog
                {
                    FileName = $"summon-editor-diff-{DateTime.Now:yyyyMMdd-HHmmss}.sql",
                    Filter = "SQL files (*.sql)|*.sql|All files (*.*)|*.*"
                };

                if (saveDialog.ShowDialog(this) != true)
                {
                    return false;
                }

                var diffSql = DiffSqlRenderer.Render(adjustedRequest);
                File.WriteAllText(saveDialog.FileName, diffSql);
                _lastDiffSql = diffSql;

                if (origin == SaveRequestOrigin.SummonSwitch)
                {
                    ClearStagedChanges();
                    TxtStatus.Text = $"SQL diff saved: {saveDialog.FileName}. Staged changes cleared for summon switch.";
                }
                else
                {
                    TxtStatus.Text = $"SQL diff saved: {saveDialog.FileName}. Pending changes remain staged.";
                }

                _dialogService.Show(this, "Save", "SQL file generated. No database write was performed.");
                return true;
            }

            if (!ConfirmDatabaseSave(adjustedRequest))
            {
                return false;
            }

            TxtStatus.Text = "Saving changes to database...";
            var result = await _repository.CommitAsync(adjustedRequest);
            _lastDiffSql = result.DiffSql;

            ClearStagedChanges();

            if (origin == SaveRequestOrigin.Toolbar && ListSummons.SelectedItem is SummonDisplayItem selected)
            {
                await LoadEvolutionChainAndTreesAsync(selected.Summon);
                _activeSummon = selected;
            }

            TxtStatus.Text = $"Saved trees: {result.UpdatedTreeCount}, skills: {result.UpdatedSkillCount}, random groups: {result.UpdatedRandomSkillGroupCount}.";
            _dialogService.Show(this, "Save", "Changes saved successfully.");
            return true;
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Save failed", "Could not save staged changes.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Save failed (see details).";
            return false;
        }
    }

    private bool ConfirmDatabaseSave(CommitRequest request)
    {
        var details = new StringBuilder();
        details.AppendLine("Database target:");
        details.AppendLine($"Server: {_settings.Database.Server},{_settings.Database.Port}");
        details.AppendLine($"Database: {_settings.Database.Database}");
        details.AppendLine();
        details.AppendLine("Pending payload:");
        details.AppendLine($"SkillTree changes: {request.TreeContexts.Count}");
        details.AppendLine($"Skill field updates: {request.SkillUpdates.Count}");
        details.AppendLine($"StatResource updates: {request.StatContexts.Count}");
        details.AppendLine($"Growth bonus updates: {request.CreatureLevelBonusUpdates.Count}");
        details.AppendLine($"Random group updates: {request.RandomSkillGroupUpdates.Count}");

        return _dialogService.Show(
            this,
            "Confirm database save",
            "Apply staged changes to SQL database now?",
            DialogButtons.YesNo,
            details.ToString(),
            defaultResult: AppDialogResult.No) == AppDialogResult.Yes;
    }

    private CommitRequest BuildCommitRequest(out List<PendingTreeEntryChange> treeChanges)
    {
        treeChanges = [];
        var treeContexts = new List<SkillTreeContext>();

        foreach (var tree in _stageStates.SelectMany(x => x.Trees))
        {
            if (tree.OriginalSkillTreeId <= 0 || tree.SummonId <= 0)
            {
                continue;
            }

            var original = tree.OriginalEntries.OrderBy(x => x.Lv).ThenBy(x => x.SkillId).ToList();
            var current = tree.CurrentEntries.Select(x => x.ToSnapshot(tree.OriginalSkillTreeId)).OrderBy(x => x.Lv).ThenBy(x => x.SkillId).ToList();
            if (AreSnapshotsEqual(original, current))
            {
                continue;
            }

            BuildTreeDiffRows(tree, original, current, treeChanges);
            treeContexts.Add(new SkillTreeContext(tree.SummonId, tree.TreeSlotIndex, tree.OriginalSkillTreeId, current, tree.CloneIfShared));
        }

        var statContexts = _pendingStatContexts.Values.ToList();
        var bonusUpdates = _pendingCreatureLevelBonusUpdates.Values.ToList();
        var randomGroupUpdates = _pendingRandomSkillGroupUpdates.Values.ToList();
        return new CommitRequest(treeContexts, _pendingSkillUpdates.Values.ToList(), statContexts, bonusUpdates, randomGroupUpdates);
    }

    private static bool AreSnapshotsEqual(IReadOnlyList<SkillTreeEntrySnapshot> a, IReadOnlyList<SkillTreeEntrySnapshot> b)
    {
        if (a.Count != b.Count)
        {
            return false;
        }

        for (var i = 0; i < a.Count; i++)
        {
            if (a[i] != b[i])
            {
                return false;
            }
        }

        return true;
    }

    private static void BuildTreeDiffRows(
        TreeContextState tree,
        IReadOnlyList<SkillTreeEntrySnapshot> original,
        IReadOnlyList<SkillTreeEntrySnapshot> current,
        List<PendingTreeEntryChange> output)
    {
        var max = Math.Max(original.Count, current.Count);
        for (var i = 0; i < max; i++)
        {
            var hasBefore = i < original.Count;
            var hasAfter = i < current.Count;

            if (hasBefore && !hasAfter)
            {
                output.Add(new PendingTreeEntryChange(TreeChangeType.Delete, tree.OriginalSkillTreeId, original[i].SkillId, $"Evolution {tree.StageIndex} Tree {tree.TreeSlotIndex}", SnapshotToDictionary(original[i]), null));
                continue;
            }

            if (!hasBefore && hasAfter)
            {
                output.Add(new PendingTreeEntryChange(TreeChangeType.Insert, tree.OriginalSkillTreeId, current[i].SkillId, $"Evolution {tree.StageIndex} Tree {tree.TreeSlotIndex}", null, SnapshotToDictionary(current[i])));
                continue;
            }

            if (hasBefore && hasAfter && original[i] != current[i])
            {
                output.Add(new PendingTreeEntryChange(TreeChangeType.Update, tree.OriginalSkillTreeId, current[i].SkillId, $"Evolution {tree.StageIndex} Tree {tree.TreeSlotIndex}", SnapshotToDictionary(original[i]), SnapshotToDictionary(current[i])));
            }
        }
    }

    private static Dictionary<string, object?> SnapshotToDictionary(SkillTreeEntrySnapshot snapshot)
    {
        return new Dictionary<string, object?>
        {
            ["skill_id"] = snapshot.SkillId,
            ["skill_group_id"] = snapshot.SkillGroupId,
            ["min_skill_lv"] = snapshot.MinSkillLv,
            ["max_skill_lv"] = snapshot.MaxSkillLv,
            ["lv"] = snapshot.Lv,
            ["job_lv"] = snapshot.JobLv,
            ["need_skill_id_1"] = snapshot.NeedSkillId1,
            ["need_skill_lv_1"] = snapshot.NeedSkillLv1,
            ["need_skill_id_2"] = snapshot.NeedSkillId2,
            ["need_skill_lv_2"] = snapshot.NeedSkillLv2,
            ["need_skill_id_3"] = snapshot.NeedSkillId3,
            ["need_skill_lv_3"] = snapshot.NeedSkillLv3,
            ["cenhance_min"] = snapshot.CEnhanceMin,
            ["cenhance_max"] = snapshot.CEnhanceMax
        };
    }

    private async Task<CommitRequest?> ApplyWarningsAndCloneFlagsAsync(CommitRequest request)
    {
        if (_repository is null || _currentChain is null)
        {
            return request;
        }

        var treeContextMap = request.TreeContexts.ToDictionary(
            x => (x.SummonId, x.TreeSlotIndex, x.OriginalSkillTreeId),
            x => x);
        var treeCloneFlags = treeContextMap.Keys.ToDictionary(x => x, _ => false);
        var statCloneFlags = request.StatContexts.ToDictionary(x => x, _ => false);
        var randomGroupUpdates = request.RandomSkillGroupUpdates.ToDictionary(x => x.GroupId, x => x);
        var chainIds = _currentChain.OrderedStages.Select(x => x.SummonId).ToHashSet();
        var reservedRandomGroupIds = new HashSet<int>(
            treeContextMap.Values
                .SelectMany(x => x.Entries)
                .Select(x => x.SkillGroupId)
                .Where(x => x > 0)
                .Concat(randomGroupUpdates.Keys));

        var changedSkillIds = request.SkillUpdates.Select(x => x.SkillId).ToArray();
        if (changedSkillIds.Length > 0)
        {
            var usage = await _repository.GetSkillUsageImpactAsync(changedSkillIds);
            var external = usage.Where(x => !chainIds.Contains(x.SummonId)).Take(15).ToList();
            if (external.Count > 0)
            {
                var sb = new StringBuilder();
                sb.AppendLine("Edited skills are also used by other summons.");
                sb.AppendLine("Continue anyway?");
                sb.AppendLine();
                foreach (var row in external)
                {
                    sb.AppendLine($"Skill {row.SkillId} -> Summon {row.SummonName} ({row.SummonId}), Tree {row.SkillTreeId}");
                }

                if (_dialogService.Show(this, "Global skill impact", sb.ToString(), DialogButtons.YesNo, defaultResult: AppDialogResult.No) != AppDialogResult.Yes)
                {
                    return null;
                }
            }
        }

        var changedRandomGroupIds = randomGroupUpdates.Keys.ToArray();
        if (changedRandomGroupIds.Length > 0)
        {
            var usage = await _repository.GetRandomSkillGroupUsageImpactAsync(changedRandomGroupIds);
            var usageByGroup = usage
                .GroupBy(x => x.GroupId)
                .ToDictionary(x => x.Key, x => x.ToList());

            foreach (var groupId in changedRandomGroupIds.OrderBy(x => x))
            {
                if (!usageByGroup.TryGetValue(groupId, out var rows))
                {
                    continue;
                }

                var external = rows.Where(x => !chainIds.Contains(x.SummonId)).Take(20).ToList();
                if (external.Count == 0)
                {
                    continue;
                }

                var sb = new StringBuilder();
                sb.AppendLine($"Random group {groupId} is used by other summons.");
                sb.AppendLine("Yes = Clone group and remap current edited rows");
                sb.AppendLine("No = Edit in place");
                sb.AppendLine("Cancel = Abort save");
                sb.AppendLine();
                foreach (var row in external)
                {
                    sb.AppendLine($"Summon {row.SummonName} ({row.SummonId}) Tree {row.TreeSlotIndex} (SkillTree {row.SkillTreeId})");
                }

                var choice = _dialogService.Show(this, "Shared random group detected", sb.ToString(), DialogButtons.YesNoCancel, defaultResult: AppDialogResult.Cancel);
                if (choice == AppDialogResult.Cancel)
                {
                    return null;
                }

                if (choice == AppDialogResult.Yes)
                {
                    if (!randomGroupUpdates.TryGetValue(groupId, out var current))
                    {
                        continue;
                    }

                    EnsureTreeContextsForRandomGroup(treeContextMap, treeCloneFlags, groupId);

                    var newGroupId = await GetNextAvailableRandomGroupIdAsync(reservedRandomGroupIds);
                    reservedRandomGroupIds.Add(newGroupId);

                    randomGroupUpdates.Remove(groupId);
                    randomGroupUpdates[newGroupId] = current with { GroupId = newGroupId };

                    var keys = treeContextMap.Keys.ToArray();
                    foreach (var key in keys)
                    {
                        var tree = treeContextMap[key];
                        var changed = false;
                        var adjustedEntries = tree.Entries
                            .Select(entry =>
                            {
                                if (entry.SkillGroupId != groupId)
                                {
                                    return entry;
                                }

                                changed = true;
                                return entry with { SkillGroupId = newGroupId };
                            })
                            .ToList();

                        if (changed)
                        {
                            treeContextMap[key] = new SkillTreeContext(
                                tree.SummonId,
                                tree.TreeSlotIndex,
                                tree.OriginalSkillTreeId,
                                adjustedEntries,
                                tree.CloneIfShared);
                        }
                    }
                }
            }
        }

        var changedTreeIds = treeContextMap.Values.Select(x => x.OriginalSkillTreeId).Distinct().ToArray();
        if (changedTreeIds.Length > 0)
        {
            var usage = await _repository.GetSharedTreeUsageAsync(changedTreeIds);
            var sharedPlan = SharedTreeClonePlanner.Build(treeContextMap.Values.ToList(), usage, chainIds);

            foreach (var planRow in sharedPlan.Where(x => x.IsSharedExternally))
            {
                var key = (planRow.SummonId, planRow.TreeSlotIndex, planRow.SkillTreeId);
                if (!treeContextMap.ContainsKey(key))
                {
                    continue;
                }

                var sb = new StringBuilder();
                sb.AppendLine($"Tree {planRow.SkillTreeId} is shared with other summons.");
                sb.AppendLine("Yes = Clone for this summon slot");
                sb.AppendLine("No = Edit in place");
                sb.AppendLine("Cancel = Abort save");
                sb.AppendLine();
                foreach (var row in planRow.ExternalUsageRows.Take(20))
                {
                    sb.AppendLine($"Summon {row.SummonName} ({row.SummonId}) slot {row.TreeSlotIndex}");
                }

                var choice = _dialogService.Show(this, "Shared tree detected", sb.ToString(), DialogButtons.YesNoCancel, defaultResult: AppDialogResult.Cancel);
                if (choice == AppDialogResult.Cancel)
                {
                    return null;
                }

                treeCloneFlags[key] = choice == AppDialogResult.Yes;
            }
        }

        var changedStatIds = request.StatContexts
            .Select(x => x.OriginalStatId)
            .Where(x => x > 0)
            .Distinct()
            .ToArray();
        if (changedStatIds.Length > 0)
        {
            var usage = await _repository.GetStatUsageImpactAsync(changedStatIds);
            var usageByStat = usage.GroupBy(x => x.StatId).ToDictionary(x => x.Key, x => x.ToList());

            foreach (var stat in request.StatContexts)
            {
                if (!usageByStat.TryGetValue(stat.OriginalStatId, out var rows))
                {
                    continue;
                }

                var external = rows.Where(x => !chainIds.Contains(x.SummonId)).Take(20).ToList();
                if (external.Count == 0)
                {
                    continue;
                }

                var sb = new StringBuilder();
                sb.AppendLine($"StatResource {stat.OriginalStatId} is shared with other summons.");
                sb.AppendLine("Yes = Clone stat and remap selected summon");
                sb.AppendLine("No = Edit in place");
                sb.AppendLine("Cancel = Abort save");
                sb.AppendLine();
                foreach (var row in external)
                {
                    sb.AppendLine($"Summon {row.SummonName} ({row.SummonId})");
                }

                var choice = _dialogService.Show(this, "Shared stat detected", sb.ToString(), DialogButtons.YesNoCancel, defaultResult: AppDialogResult.Cancel);
                if (choice == AppDialogResult.Cancel)
                {
                    return null;
                }

                statCloneFlags[stat] = choice == AppDialogResult.Yes;
            }
        }

        var changedBonusIds = request.CreatureLevelBonusUpdates.Select(x => x.BonusId).Distinct().ToArray();
        if (changedBonusIds.Length > 0)
        {
            var usage = await _repository.GetCreatureLevelBonusUsageImpactAsync(changedBonusIds);
            var external = usage.Where(x => !chainIds.Contains(x.SummonId)).Take(30).ToList();
            if (external.Count > 0)
            {
                var sb = new StringBuilder();
                sb.AppendLine("Edited growth bonus rows are used by other summons.");
                sb.AppendLine("Continue anyway?");
                sb.AppendLine();
                foreach (var row in external)
                {
                    sb.AppendLine($"Bonus {row.BonusId} -> Summon {row.SummonName} ({row.SummonId}) Growth Stage {row.StageIndex}");
                }

                var choice = _dialogService.Show(this, "Shared growth bonus detected", sb.ToString(), DialogButtons.YesNo, defaultResult: AppDialogResult.No);
                if (choice != AppDialogResult.Yes)
                {
                    return null;
                }
            }
        }

        var adjustedTrees = treeContextMap.Values
            .Select(x =>
            {
                var key = (x.SummonId, x.TreeSlotIndex, x.OriginalSkillTreeId);
                return new SkillTreeContext(
                    x.SummonId,
                    x.TreeSlotIndex,
                    x.OriginalSkillTreeId,
                    x.Entries,
                    treeCloneFlags.GetValueOrDefault(key));
            })
            .ToList();

        var adjustedStats = request.StatContexts
            .Select(x => new StatResourceContext(x.SummonId, x.OriginalStatId, x.Stats, statCloneFlags[x]))
            .ToList();

        return new CommitRequest(adjustedTrees, request.SkillUpdates, adjustedStats, request.CreatureLevelBonusUpdates, randomGroupUpdates.Values.ToList());
    }

    private static IReadOnlyList<SaveReviewRow> BuildReviewRows(
        IReadOnlyList<PendingTreeEntryChange> treeChanges,
        IReadOnlyList<PendingFieldChange> fieldChanges,
        IReadOnlyList<RandomSkillGroupUpsert> randomGroupUpdates)
    {
        var rows = new List<SaveReviewRow>();
        foreach (var change in treeChanges)
        {
            rows.Add(new SaveReviewRow
            {
                ChangeType = $"Tree {change.Type}",
                Target = $"{change.Description} / {change.SkillTreeId}",
                Field = "skill_id",
                Before = change.Before is null ? string.Empty : (change.Before.TryGetValue("skill_id", out var beforeSkillId) ? beforeSkillId?.ToString() ?? string.Empty : string.Empty),
                After = change.After is null ? string.Empty : (change.After.TryGetValue("skill_id", out var afterSkillId) ? afterSkillId?.ToString() ?? string.Empty : string.Empty)
            });
        }

        foreach (var field in fieldChanges)
        {
            var (changeType, target) = field.Entity switch
            {
                "SkillResource" => ("Skill Field", $"Skill {field.Key}"),
                "StatResource" => ("Stat Field", $"Stat {field.Key}"),
                "CreatureLevelBonus" => ("Growth Bonus Field", $"Bonus {field.Key}"),
                _ => ($"{field.Entity} Field", $"{field.Entity} {field.Key}")
            };

            rows.Add(new SaveReviewRow
            {
                ChangeType = changeType,
                Target = target,
                Field = field.Field,
                Before = field.OldValue?.ToString() ?? string.Empty,
                After = field.NewValue?.ToString() ?? string.Empty
            });
        }

        foreach (var group in randomGroupUpdates.OrderBy(x => x.GroupId))
        {
            rows.Add(new SaveReviewRow
            {
                ChangeType = "Random Group Upsert",
                Target = $"Group {group.GroupId}",
                Field = "use_code",
                Before = string.Empty,
                After = group.UseCode.ToString(CultureInfo.InvariantCulture)
            });

            var slots = new[]
            {
                group.SkillId1, group.SkillId2, group.SkillId3, group.SkillId4, group.SkillId5,
                group.SkillId6, group.SkillId7, group.SkillId8, group.SkillId9, group.SkillId10,
                group.SkillId11, group.SkillId12, group.SkillId13, group.SkillId14, group.SkillId15
            };

            for (var i = 0; i < slots.Length; i++)
            {
                if (slots[i] <= 0)
                {
                    continue;
                }

                rows.Add(new SaveReviewRow
                {
                    ChangeType = "Random Group Slot",
                    Target = $"Group {group.GroupId}",
                    Field = $"skill_id_{i + 1}",
                    Before = string.Empty,
                    After = slots[i].ToString(CultureInfo.InvariantCulture)
                });
            }
        }

        return rows;
    }

    private async Task RefreshSkillNamesAndEffectiveAsync()
    {
        await PopulateSkillDisplayAsync();
        await LoadRandomSkillGroupsForVisibleRowsAsync();
        BuildEffectiveStageRows();
        ApplyRandomGroupDisplayToRows();
    }

    private async void BtnSettings_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new SettingsWindow(_settings, _dialogService)
        {
            Owner = this
        };

        if (dialog.ShowDialog() != true)
        {
            return;
        }

        _settings = dialog.ResultSettings;
        _settings.SaveToSqlMode = TglSaveMode.IsChecked == true;
        ApplySaveModeToggleState(_settings.SaveToSqlMode);

        await _settingsStore.SaveAsync(_settings);
        EnsureRepository();
        UpdateStatsButtonState();
        await BuildAssetIndexAsync();
        await LoadSummonsAsync();
    }

    private static T? FindVisualParent<T>(DependencyObject? child) where T : DependencyObject
    {
        while (child is not null)
        {
            if (child is T target)
            {
                return target;
            }

            child = VisualTreeHelper.GetParent(child);
        }

        return null;
    }
}
