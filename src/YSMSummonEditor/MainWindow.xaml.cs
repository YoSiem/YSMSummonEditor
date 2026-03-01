using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
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
using YSMSummonEditor.Windows;

namespace YSMSummonEditor;

public partial class MainWindow : Window
{
    private readonly ISettingsStore _settingsStore;
    private readonly IAssetResolver _assetResolver;
    private readonly SqlConnectionFactory _connectionFactory;
    private readonly ISkillUpdateBuilder _skillUpdateBuilder;

    private readonly ObservableCollection<SummonDisplayItem> _summonItems = [];
    private readonly ObservableCollection<EffectiveSkillRow> _effectiveRows = [];
    private readonly List<StageViewState> _stageStates = [];

    private readonly Dictionary<int, SkillUpdate> _pendingSkillUpdates = new();
    private readonly List<PendingFieldChange> _pendingFieldChanges = [];
    private readonly Dictionary<int, string> _skillNames = new();

    private SummonEditorSettings _settings = new();
    private ISummonEditorRepository? _repository;
    private EvolutionChainSnapshot? _currentChain;
    private string _lastDiffSql = string.Empty;

    public MainWindow(
        ISettingsStore settingsStore,
        IAssetResolver assetResolver,
        SqlConnectionFactory connectionFactory,
        ISkillUpdateBuilder skillUpdateBuilder)
    {
        _settingsStore = settingsStore;
        _assetResolver = assetResolver;
        _connectionFactory = connectionFactory;
        _skillUpdateBuilder = skillUpdateBuilder;

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

        TglIncludeRiding.IsOn = _settings.IncludeRidingOnlyByDefault;
        TglIncludeNonForm1.IsOn = _settings.IncludeNonForm1ByDefault;
        TglShowAllEvos.IsOn = _settings.ShowAllEvolutionsByDefault;
        EnsureRepository();
    }

    private async Task BuildAssetIndexAsync()
    {
        if (string.IsNullOrWhiteSpace(_settings.DumpFolderPath))
        {
            TxtStatus.Text = "Dump folder is not configured.";
            return;
        }

        await _assetResolver.RebuildIndexAsync(_settings.DumpFolderPath);
        TxtStatus.Text = "Asset index loaded.";
    }

    private void EnsureRepository()
    {
        var conn = _settings.Database.ToConnectionString();
        _repository = new SqlServerSummonEditorRepository(conn, _connectionFactory);
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

        try
        {
            TxtStatus.Text = "Loading summons...";
            _summonItems.Clear();

            var filter = new SummonListFilter(
                TxtSearch.Text.Trim(),
                TglIncludeRiding.IsOn,
                TglIncludeNonForm1.IsOn,
                TglShowAllEvos.IsOn);

            var summons = await _repository!.SearchSummonsAsync(filter);
            foreach (var summon in summons)
            {
                var item = new SummonDisplayItem(summon);
                item.Icon = ImageLoader.TryLoad(_assetResolver.ResolveSummonFace(summon.FaceFileName));
                _summonItems.Add(item);
            }

            TxtStatus.Text = $"Loaded {_summonItems.Count.ToString("N0", CultureInfo.InvariantCulture)} summons.";
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, ex.Message, "Search failed", MessageBoxButton.OK, MessageBoxImage.Error);
            TxtStatus.Text = "Search failed.";
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
            _skillNames.Clear();

            var final = await ResolveFinalEvolutionAsync(selected);
            var sameCard = await _repository.GetSummonsByCardIdAsync(final.CardId);
            _currentChain = EvolutionChainResolver.ResolveFixedThreeStageChain(final, sameCard);

            await BuildStageStateAsync(1, _currentChain.Stage1);
            await BuildStageStateAsync(2, _currentChain.Stage2);
            await BuildStageStateAsync(3, _currentChain.Stage3);

            await PopulateSkillNamesAsync();
            BuildEffectiveSkills();
            BindStageTabs();
            UpdateHeader(final);

            TxtStatus.Text = "Chain loaded.";
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, ex.Message, "Chain load failed", MessageBoxButton.OK, MessageBoxImage.Warning);
            TxtStatus.Text = "Chain load failed.";
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

    private async Task PopulateSkillNamesAsync()
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

        var names = await _repository.GetSkillNamesAsync(skillIds);
        foreach (var pair in names)
        {
            _skillNames[pair.Key] = pair.Value;
        }

        foreach (var row in _stageStates.SelectMany(x => x.Trees).SelectMany(x => x.CurrentEntries))
        {
            row.SkillName = _skillNames.TryGetValue(row.SkillId, out var name)
                ? name
                : $"Skill {row.SkillId}";
        }
    }

    private void BuildEffectiveSkills()
    {
        _effectiveRows.Clear();
        foreach (var stage in _stageStates)
        {
            foreach (var tree in stage.Trees)
            {
                foreach (var row in tree.CurrentEntries)
                {
                    _effectiveRows.Add(new EffectiveSkillRow
                    {
                        StageIndex = stage.StageIndex,
                        TreeSlotIndex = tree.TreeSlotIndex,
                        SkillId = row.SkillId,
                        SkillName = row.SkillName,
                        MinSkillLevel = row.MinSkillLv,
                        MaxSkillLevel = row.MaxSkillLv
                    });
                }
            }
        }
    }

    private void BindStageTabs()
    {
        TabStages.Items.Clear();
        foreach (var stage in _stageStates)
        {
            var item = new TabItem
            {
                Header = stage.Name,
                Tag = stage
            };
            TabStages.Items.Add(item);
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
        TxtSummonHeader.Text = $"{final.SummonName} ({final.SummonId})";
        TxtSummonMeta.Text = $"Card: {final.CardName ?? "Unknown"} / Card ID: {final.CardId}";
        TxtChainInfo.Text = _currentChain is null
            ? string.Empty
            : $"Chain IDs: {FormatChainIds(_currentChain)}";

        var large = _assetResolver.ResolveSummonIllust(final.IllustFileName);
        ImgLarge.Source = ImageLoader.TryLoad(large);
    }

    private static string FormatChainIds(EvolutionChainSnapshot chain)
    {
        var ids = chain.OrderedStages.Select(x => x.SummonId.ToString(CultureInfo.InvariantCulture));
        return string.Join(" -> ", ids);
    }

    private TreeContextState? GetSelectedTree()
    {
        return TabTrees.SelectedItem as TreeContextState;
    }

    private async void BtnAddTreeEntry_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree is null || tree.OriginalSkillTreeId <= 0)
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

    private async void BtnDeleteTreeEntry_Click(object sender, RoutedEventArgs e)
    {
        var tree = GetSelectedTree();
        if (tree?.SelectedEntry is null)
        {
            return;
        }

        tree.CurrentEntries.Remove(tree.SelectedEntry);
        await RefreshSkillNamesAndEffectiveAsync();
    }

    private async void GridTree_MouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if ((sender as DataGrid)?.SelectedItem is not SkillTreeEntryRow row || row.SkillId <= 0 || _repository is null)
        {
            return;
        }

        await OpenSkillEditDialogAsync(row.SkillId);
    }

    private async void GridTree_CellEditEnding(object sender, DataGridCellEditEndingEventArgs e)
    {
        if (e.EditAction != DataGridEditAction.Commit)
        {
            return;
        }

        await RefreshSkillNamesAndEffectiveAsync();
    }

    private async void GridEffective_MouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if ((sender as DataGrid)?.SelectedItem is not EffectiveSkillRow row || row.SkillId <= 0 || _repository is null)
        {
            return;
        }

        await OpenSkillEditDialogAsync(row.SkillId);
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
            MessageBox.Show(this, ex.Message, "Skill edit failed", MessageBoxButton.OK, MessageBoxImage.Error);
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
                MessageBox.Show(this, "No changes to save.", "Save", MessageBoxButton.OK, MessageBoxImage.Information);
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
            MessageBox.Show(this, "Changes saved successfully.", "Save", MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, ex.Message, "Save failed", MessageBoxButton.OK, MessageBoxImage.Error);
            TxtStatus.Text = "Save failed.";
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

            treeContexts.Add(new SkillTreeContext(
                tree.SummonId,
                tree.TreeSlotIndex,
                tree.OriginalSkillTreeId,
                current,
                tree.CloneIfShared));
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
                output.Add(new PendingTreeEntryChange(
                    TreeChangeType.Delete,
                    tree.OriginalSkillTreeId,
                    original[i].SkillId,
                    $"Stage {tree.StageIndex} Tree {tree.TreeSlotIndex}",
                    SnapshotToDictionary(original[i]),
                    null));
                continue;
            }

            if (!hasBefore && hasAfter)
            {
                output.Add(new PendingTreeEntryChange(
                    TreeChangeType.Insert,
                    tree.OriginalSkillTreeId,
                    current[i].SkillId,
                    $"Stage {tree.StageIndex} Tree {tree.TreeSlotIndex}",
                    null,
                    SnapshotToDictionary(current[i])));
                continue;
            }

            if (hasBefore && hasAfter && original[i] != current[i])
            {
                output.Add(new PendingTreeEntryChange(
                    TreeChangeType.Update,
                    tree.OriginalSkillTreeId,
                    current[i].SkillId,
                    $"Stage {tree.StageIndex} Tree {tree.TreeSlotIndex}",
                    SnapshotToDictionary(original[i]),
                    SnapshotToDictionary(current[i])));
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

                if (MessageBox.Show(this, sb.ToString(), "Global skill impact", MessageBoxButton.YesNo, MessageBoxImage.Warning) != MessageBoxResult.Yes)
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
            var treeContextLookup = request.TreeContexts.ToDictionary(
                x => (x.SummonId, x.TreeSlotIndex, x.OriginalSkillTreeId),
                x => x);

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

                var choice = MessageBox.Show(this, sb.ToString(), "Shared tree detected", MessageBoxButton.YesNoCancel, MessageBoxImage.Warning);
                if (choice == MessageBoxResult.Cancel)
                {
                    return null;
                }

                cloneFlags[tree] = choice == MessageBoxResult.Yes;
            }
        }

        var adjustedTrees = request.TreeContexts
            .Select(x => new SkillTreeContext(
                x.SummonId,
                x.TreeSlotIndex,
                x.OriginalSkillTreeId,
                x.Entries,
                cloneFlags[x]))
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
        await PopulateSkillNamesAsync();
        BuildEffectiveSkills();
    }

    private async void BtnSettings_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new SettingsWindow(_settings)
        {
            Owner = this
        };

        if (dialog.ShowDialog() != true)
        {
            return;
        }

        _settings = dialog.ResultSettings;
        _settings.IncludeRidingOnlyByDefault = TglIncludeRiding.IsOn;
        _settings.IncludeNonForm1ByDefault = TglIncludeNonForm1.IsOn;
        _settings.ShowAllEvolutionsByDefault = TglShowAllEvos.IsOn;
        await _settingsStore.SaveAsync(_settings);
        EnsureRepository();
        await BuildAssetIndexAsync();
        await LoadSummonsAsync();
    }

    private void BtnExportDiff_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(_lastDiffSql))
        {
            MessageBox.Show(this, "No diff SQL available yet.", "Export", MessageBoxButton.OK, MessageBoxImage.Information);
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
}
