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
    private readonly ISettingsStore _settingsStore;
    private readonly IAssetResolver _assetResolver;
    private readonly SqlConnectionFactory _connectionFactory;
    private readonly ISkillUpdateBuilder _skillUpdateBuilder;
    private readonly IAppDialogService _dialogService;

    private readonly ObservableCollection<SummonDisplayItem> _summonItems = [];
    private readonly ObservableCollection<EffectiveSkillMatrixRow> _effectiveRows = [];
    private readonly List<StageViewState> _stageStates = [];

    private readonly Dictionary<int, SkillUpdate> _pendingSkillUpdates = new();
    private readonly List<PendingFieldChange> _pendingFieldChanges = [];
    private readonly Dictionary<int, SkillDisplaySnapshot> _skillDisplay = new();

    private SummonEditorSettings _settings = new();
    private ISummonEditorRepository? _repository;
    private EvolutionChainSnapshot? _currentChain;
    private string _lastDiffSql = string.Empty;
    private int _effectiveContextStage;

    public MainWindow(
        ISettingsStore settingsStore,
        IAssetResolver assetResolver,
        SqlConnectionFactory connectionFactory,
        ISkillUpdateBuilder skillUpdateBuilder,
        IAppDialogService dialogService)
    {
        _settingsStore = settingsStore;
        _assetResolver = assetResolver;
        _connectionFactory = connectionFactory;
        _skillUpdateBuilder = skillUpdateBuilder;
        _dialogService = dialogService;

        InitializeComponent();

        ListSummons.ItemsSource = _summonItems;
        GridEffective.ItemsSource = _effectiveRows;

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

        ChkIncludeRiding.IsChecked = _settings.IncludeRidingOnlyByDefault;
        ChkIncludeNonForm1.IsChecked = _settings.IncludeNonForm1ByDefault;
        ChkShowAllEvos.IsChecked = _settings.ShowAllEvolutionsByDefault;

        EnsureRepository();
        if (_repository is null)
        {
            TxtStatus.Text = "Database is not configured. Add .env.local or set values in Settings.";
        }
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
                ChkIncludeRiding.IsChecked == true,
                ChkIncludeNonForm1.IsChecked == true,
                ChkShowAllEvos.IsChecked == true);

            var summons = await _repository.SearchSummonsAsync(filter);
            foreach (var summon in summons)
            {
                _summonItems.Add(new SummonDisplayItem(summon)
                {
                    Icon = ImageLoader.TryLoad(_assetResolver.ResolveSummonFace(summon.FaceFileName))
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
        if (ListSummons.SelectedItem is not SummonDisplayItem selected)
        {
            return;
        }

        await LoadEvolutionChainAndTreesAsync(selected.Summon);
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
            _pendingSkillUpdates.Clear();
            _pendingFieldChanges.Clear();
            _stageStates.Clear();
            _skillDisplay.Clear();

            var final = await ResolveFinalEvolutionAsync(selected);
            var sameCard = await _repository.GetSummonsByCardIdAsync(final.CardId);
            _currentChain = EvolutionChainResolver.ResolveFixedThreeStageChain(final, sameCard);

            await BuildStageStateAsync(1, _currentChain.Stage1);
            await BuildStageStateAsync(2, _currentChain.Stage2);
            await BuildStageStateAsync(3, _currentChain.Stage3);

            await PopulateSkillDisplayAsync();
            BuildEffectiveMatrix();
            BindStageTabs();
            UpdateHeader(final);

            TxtStatus.Text = "Chain loaded.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Chain load failed", "Could not load evolution chain.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Chain load failed (see details).";
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

        _stageStates.Add(new StageViewState
        {
            StageIndex = stageIndex,
            Name = $"Evo {stageIndex}",
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
                row.SkillIcon = ImageLoader.TryLoad(_assetResolver.ResolveSkillIcon(display.IconFileName));
            }
            else
            {
                row.SkillName = $"Skill {row.SkillId}";
                row.SkillTooltipMarkup = string.Empty;
                row.SkillTooltipPlain = $"Skill {row.SkillId}";
                row.SkillIcon = null;
            }
        }
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

    private void BuildEffectiveMatrix()
    {
        _effectiveRows.Clear();

        var map = new Dictionary<string, EffectiveSkillMatrixRow>(StringComparer.Ordinal);
        foreach (var stage in _stageStates)
        {
            foreach (var tree in stage.Trees)
            {
                foreach (var entry in tree.CurrentEntries)
                {
                    var key = $"{tree.TreeSlotIndex}:{entry.SkillId}:{entry.Lv}";
                    if (!map.TryGetValue(key, out var row))
                    {
                        row = new EffectiveSkillMatrixRow(tree.TreeSlotIndex, entry.SkillId, entry.Lv);
                        map[key] = row;
                    }

                    row.AttachStageEntry(stage.StageIndex, entry);
                }
            }
        }

        foreach (var row in map.Values.OrderBy(x => x.TreeSlotIndex).ThenBy(x => x.SkillId).ThenBy(x => x.Level))
        {
            _effectiveRows.Add(row);
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
        if (TabStages.SelectedItem is not TabItem item || item.Tag is not StageViewState stage)
        {
            return;
        }

        TabTrees.ItemsSource = stage.Trees;
        if (stage.Trees.Count > 0)
        {
            TabTrees.SelectedIndex = 0;
        }
    }

    private void UpdateHeader(SummonListItem final)
    {
        TxtSummonHeader.Text = final.SummonName;
        TxtSummonHeader.ToolTip = $"{final.SummonName} ({final.SummonId})";
        TxtSummonMeta.Text = $"ID: {final.SummonId} | Card: {final.CardId}";
        TxtSummonMeta.ToolTip = $"Card: {final.CardName ?? "Unknown"} ({final.CardId})";
        TxtChainInfo.Text = _currentChain is null ? string.Empty : $"Chain: {FormatChainIds(_currentChain)}";
        TxtChainInfo.ToolTip = "Resolved stages from the selected final evolution.";

        var large = _assetResolver.ResolveSummonIllust(final.IllustFileName);
        ImgLarge.Source = ImageLoader.TryLoad(large);
    }

    private static string FormatChainIds(EvolutionChainSnapshot chain)
    {
        var ids = chain.OrderedStages.Select(x => x.SummonId.ToString(CultureInfo.InvariantCulture));
        return string.Join(" -> ", ids);
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
            $"Delete skill {row.SkillId} from Evo {tree.StageIndex}, Tree {tree.TreeSlotIndex}?",
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

        if (menu.Items.Count > 1 && menu.Items[1] is MenuItem edit)
        {
            edit.IsEnabled = row is { SkillId: > 0 };
        }

        if (menu.Items.Count > 2 && menu.Items[2] is MenuItem delete)
        {
            delete.IsEnabled = row is not null;
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

    private void GridEffective_PreviewMouseRightButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not DataGrid grid)
        {
            return;
        }

        var row = FindVisualParent<DataGridRow>(e.OriginalSource as DependencyObject);
        var cell = FindVisualParent<DataGridCell>(e.OriginalSource as DependencyObject);
        if (row?.Item is EffectiveSkillMatrixRow matrixRow)
        {
            grid.SelectedItem = matrixRow;
            row.IsSelected = true;
            _effectiveContextStage = ResolveStageFromEffectiveColumn(cell?.Column);
        }
        else
        {
            _effectiveContextStage = 0;
        }
    }

    private void EffectiveContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        if (sender is not ContextMenu menu)
        {
            return;
        }

        var row = GridEffective.SelectedItem as EffectiveSkillMatrixRow;
        var stage = row is null ? 0 : ResolveEffectiveActionStage(row);

        if (menu.Items.Count > 0 && menu.Items[0] is MenuItem edit)
        {
            edit.IsEnabled = row is { SkillId: > 0 };
        }

        if (menu.Items.Count > 1 && menu.Items[1] is MenuItem delete)
        {
            delete.IsEnabled = row is not null && stage > 0 && row.TryGetEntry(stage, out _);
        }
    }

    private int ResolveEffectiveActionStage(EffectiveSkillMatrixRow row)
    {
        return _effectiveContextStage is >= 1 and <= 3
            ? _effectiveContextStage
            : row.FirstAvailableStage;
    }

    private static int ResolveStageFromEffectiveColumn(DataGridColumn? column)
    {
        if (column is null)
        {
            return 0;
        }

        return column.DisplayIndex switch
        {
            4 or 5 => 1,
            6 or 7 => 2,
            8 or 9 => 3,
            _ => 0
        };
    }

    private TreeContextState? FindTreeByStageAndSlot(int stageIndex, int treeSlotIndex)
    {
        return _stageStates.FirstOrDefault(x => x.StageIndex == stageIndex)?.Trees.FirstOrDefault(x => x.TreeSlotIndex == treeSlotIndex);
    }

    private async void MnuEffectiveEditSkill_Click(object sender, RoutedEventArgs e)
    {
        if (GridEffective.SelectedItem is not EffectiveSkillMatrixRow row || row.SkillId <= 0)
        {
            return;
        }

        await OpenSkillEditDialogAsync(row.SkillId);
    }

    private async void MnuEffectiveDeleteRow_Click(object sender, RoutedEventArgs e)
    {
        if (GridEffective.SelectedItem is not EffectiveSkillMatrixRow row)
        {
            return;
        }

        var stage = ResolveEffectiveActionStage(row);
        if (stage <= 0 || !row.TryGetEntry(stage, out var treeRow) || treeRow is null)
        {
            return;
        }

        var tree = FindTreeByStageAndSlot(stage, row.TreeSlotIndex);
        if (tree is null)
        {
            return;
        }

        await DeleteTreeEntryAsync(tree, treeRow);
    }

    private void GridEffective_CellEditEnding(object sender, DataGridCellEditEndingEventArgs e)
    {
        if (e.EditAction == DataGridEditAction.Commit)
        {
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

    private async void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        if (_repository is null)
        {
            return;
        }

        try
        {
            var commitRequest = BuildCommitRequest(out var treeReviewChanges);
            if (commitRequest.TreeContexts.Count == 0 && commitRequest.SkillUpdates.Count == 0)
            {
                _dialogService.Show(this, "Save", "No changes to save.");
                return;
            }

            var adjustedRequest = await ApplyWarningsAndCloneFlagsAsync(commitRequest);
            if (adjustedRequest is null)
            {
                return;
            }

            var reviewRows = BuildReviewRows(treeReviewChanges, _pendingFieldChanges);
            var reviewWindow = new SaveReviewWindow(reviewRows) { Owner = this };
            if (reviewWindow.ShowDialog() != true)
            {
                return;
            }

            TxtStatus.Text = "Saving changes...";
            var result = await _repository.CommitAsync(adjustedRequest);
            _lastDiffSql = result.DiffSql;

            _pendingSkillUpdates.Clear();
            _pendingFieldChanges.Clear();

            if (ListSummons.SelectedItem is SummonDisplayItem selected)
            {
                await LoadEvolutionChainAndTreesAsync(selected.Summon);
            }

            TxtStatus.Text = $"Saved trees: {result.UpdatedTreeCount}, skills: {result.UpdatedSkillCount}.";
            _dialogService.Show(this, "Save", "Changes saved successfully.");
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Save failed", "Could not save staged changes.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Save failed (see details).";
        }
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

        return new CommitRequest(treeContexts, _pendingSkillUpdates.Values.ToList());
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
                output.Add(new PendingTreeEntryChange(TreeChangeType.Delete, tree.OriginalSkillTreeId, original[i].SkillId, $"Evo {tree.StageIndex} Tree {tree.TreeSlotIndex}", SnapshotToDictionary(original[i]), null));
                continue;
            }

            if (!hasBefore && hasAfter)
            {
                output.Add(new PendingTreeEntryChange(TreeChangeType.Insert, tree.OriginalSkillTreeId, current[i].SkillId, $"Evo {tree.StageIndex} Tree {tree.TreeSlotIndex}", null, SnapshotToDictionary(current[i])));
                continue;
            }

            if (hasBefore && hasAfter && original[i] != current[i])
            {
                output.Add(new PendingTreeEntryChange(TreeChangeType.Update, tree.OriginalSkillTreeId, current[i].SkillId, $"Evo {tree.StageIndex} Tree {tree.TreeSlotIndex}", SnapshotToDictionary(original[i]), SnapshotToDictionary(current[i])));
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

        var cloneFlags = request.TreeContexts.ToDictionary(x => x, _ => false);
        var chainIds = _currentChain.OrderedStages.Select(x => x.SummonId).ToHashSet();

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

        var changedTreeIds = request.TreeContexts.Select(x => x.OriginalSkillTreeId).Distinct().ToArray();
        if (changedTreeIds.Length > 0)
        {
            var usage = await _repository.GetSharedTreeUsageAsync(changedTreeIds);
            var sharedPlan = SharedTreeClonePlanner.Build(request.TreeContexts, usage, chainIds);
            var treeContextLookup = request.TreeContexts.ToDictionary(x => (x.SummonId, x.TreeSlotIndex, x.OriginalSkillTreeId), x => x);

            foreach (var planRow in sharedPlan.Where(x => x.IsSharedExternally))
            {
                if (!treeContextLookup.TryGetValue((planRow.SummonId, planRow.TreeSlotIndex, planRow.SkillTreeId), out var tree))
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

                cloneFlags[tree] = choice == AppDialogResult.Yes;
            }
        }

        var adjustedTrees = request.TreeContexts
            .Select(x => new SkillTreeContext(x.SummonId, x.TreeSlotIndex, x.OriginalSkillTreeId, x.Entries, cloneFlags[x]))
            .ToList();

        return new CommitRequest(adjustedTrees, request.SkillUpdates);
    }

    private static IReadOnlyList<SaveReviewRow> BuildReviewRows(
        IReadOnlyList<PendingTreeEntryChange> treeChanges,
        IReadOnlyList<PendingFieldChange> fieldChanges)
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
            rows.Add(new SaveReviewRow
            {
                ChangeType = "Skill Field",
                Target = $"Skill {field.Key}",
                Field = field.Field,
                Before = field.OldValue?.ToString() ?? string.Empty,
                After = field.NewValue?.ToString() ?? string.Empty
            });
        }

        return rows;
    }

    private async Task RefreshSkillNamesAndEffectiveAsync()
    {
        await PopulateSkillDisplayAsync();
        BuildEffectiveMatrix();
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
        _settings.IncludeRidingOnlyByDefault = ChkIncludeRiding.IsChecked == true;
        _settings.IncludeNonForm1ByDefault = ChkIncludeNonForm1.IsChecked == true;
        _settings.ShowAllEvolutionsByDefault = ChkShowAllEvos.IsChecked == true;

        await _settingsStore.SaveAsync(_settings);
        EnsureRepository();
        await BuildAssetIndexAsync();
        await LoadSummonsAsync();
    }

    private void BtnExportDiff_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(_lastDiffSql))
        {
            _dialogService.Show(this, "Export", "No diff SQL available yet.");
            return;
        }

        var dialog = new SaveFileDialog
        {
            FileName = $"summon-editor-diff-{DateTime.Now:yyyyMMdd-HHmmss}.sql",
            Filter = "SQL files (*.sql)|*.sql|All files (*.*)|*.*"
        };

        if (dialog.ShowDialog(this) != true)
        {
            return;
        }

        File.WriteAllText(dialog.FileName, _lastDiffSql);
        TxtStatus.Text = $"Diff exported: {dialog.FileName}";
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
