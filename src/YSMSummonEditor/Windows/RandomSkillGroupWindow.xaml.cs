using System.Collections.ObjectModel;
using System.Globalization;
using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media.Imaging;
using YSMLib.AssetIndexing;
using YSMLib.GameData.Abstractions;
using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Helpers;
using YSMSummonEditor.Models;
using YSMSummonEditor.Services;

namespace YSMSummonEditor.Windows;

public partial class RandomSkillGroupWindow : Window
{
    private static readonly Regex AnyTagRegex = new("<[^>]+>", RegexOptions.Compiled | RegexOptions.CultureInvariant);

    private readonly int _groupId;
    private readonly ISummonEditorRepository _repository;
    private readonly IAssetResolver _assetResolver;
    private readonly IAppDialogService _dialogService;
    private readonly BitmapImage? _fallbackIcon;
    private readonly Dictionary<int, SkillDisplaySnapshot> _skillDisplayCache;
    private readonly ObservableCollection<SkillSearchRow> _searchRows = [];
    private readonly ObservableCollection<RandomSkillGroupSlotRow> _selectedRows = [];
    private readonly Dictionary<int, RandomSkillGroupSlotRow> _selectedBySkillId = new();
    private bool _isLoaded;

    public RandomSkillGroupWindow(
        int groupId,
        RandomSkillGroupEditModel initialGroup,
        ISummonEditorRepository repository,
        IAssetResolver assetResolver,
        IAppDialogService dialogService,
        IReadOnlyDictionary<int, SkillDisplaySnapshot> skillDisplayCache,
        BitmapImage? fallbackIcon)
    {
        InitializeComponent();

        _groupId = groupId;
        _repository = repository;
        _assetResolver = assetResolver;
        _dialogService = dialogService;
        _fallbackIcon = fallbackIcon;
        _skillDisplayCache = new Dictionary<int, SkillDisplaySnapshot>(skillDisplayCache);

        GridSearch.ItemsSource = _searchRows;
        GridSelected.ItemsSource = _selectedRows;

        TxtHeader.Text = $"RND[{_groupId}]";
        TxtStatus.Text = "Ready.";
        UpdateUseCode();

        Loaded += async (_, _) =>
        {
            if (_isLoaded)
            {
                return;
            }

            _isLoaded = true;
            await LoadInitialSelectionAsync(initialGroup.SkillIds);
            await SearchAsync();
        };
    }

    public RandomSkillGroupEditModel? Result { get; private set; }

    private async Task LoadInitialSelectionAsync(IReadOnlyList<int> initialSkillIds)
    {
        var uniqueIds = initialSkillIds
            .Where(x => x > 0)
            .Distinct()
            .OrderBy(x => x)
            .Take(15)
            .ToArray();

        if (uniqueIds.Length == 0)
        {
            RebuildSelectedRows();
            return;
        }

        var missing = uniqueIds
            .Where(x => !_skillDisplayCache.ContainsKey(x))
            .ToArray();

        if (missing.Length > 0)
        {
            var fetched = await _repository.GetSkillDisplayAsync(missing);
            foreach (var row in fetched)
            {
                _skillDisplayCache[row.Key] = row.Value;
            }
        }

        foreach (var skillId in uniqueIds)
        {
            if (!_skillDisplayCache.TryGetValue(skillId, out var display))
            {
                display = new SkillDisplaySnapshot(skillId, $"Skill {skillId}", null, null);
            }

            _selectedBySkillId[skillId] = new RandomSkillGroupSlotRow
            {
                SkillId = skillId,
                SkillName = display.SkillName,
                TooltipMarkup = display.TooltipMarkup ?? string.Empty,
                TooltipPlain = BuildTooltipPlain(display.TooltipMarkup, skillId, display.SkillName),
                Icon = ResolveSkillIcon(display.IconFileName)
            };
        }

        RebuildSelectedRows();
    }

    private async Task SearchAsync()
    {
        try
        {
            TxtStatus.Text = "Searching skills...";
            var result = await _repository.SearchSkillsAsync(TxtSearch.Text.Trim(), 300);
            _searchRows.Clear();

            foreach (var item in result)
            {
                var display = new SkillDisplaySnapshot(item.SkillId, item.SkillName, item.IconFileName, item.TooltipMarkup);
                _skillDisplayCache[item.SkillId] = display;

                _searchRows.Add(new SkillSearchRow
                {
                    SkillId = item.SkillId,
                    SkillName = item.SkillName,
                    TooltipMarkup = item.TooltipMarkup ?? string.Empty,
                    TooltipPlain = BuildTooltipPlain(item.TooltipMarkup, item.SkillId, item.SkillName),
                    Icon = ResolveSkillIcon(item.IconFileName)
                });
            }

            TxtStatus.Text = $"Found {_searchRows.Count.ToString("N0", CultureInfo.InvariantCulture)} skills.";
        }
        catch (Exception ex)
        {
            _dialogService.Show(this, "Skill search failed", "Could not search skills.", DialogButtons.Ok, ex.Message);
            TxtStatus.Text = "Search failed (see details).";
        }
    }

    private void AddSelectedSearchSkill()
    {
        if (GridSearch.SelectedItem is not SkillSearchRow selected)
        {
            return;
        }

        if (_selectedBySkillId.ContainsKey(selected.SkillId))
        {
            TxtStatus.Text = $"Skill {selected.SkillId} is already in the group.";
            return;
        }

        if (_selectedBySkillId.Count >= 15)
        {
            _dialogService.Show(this, "Random group", "Random group can contain up to 15 skills.");
            TxtStatus.Text = "Cannot add more than 15 skills.";
            return;
        }

        _selectedBySkillId[selected.SkillId] = new RandomSkillGroupSlotRow
        {
            SkillId = selected.SkillId,
            SkillName = selected.SkillName,
            TooltipMarkup = selected.TooltipMarkup,
            TooltipPlain = selected.TooltipPlain,
            Icon = selected.Icon
        };

        RebuildSelectedRows();
        TxtStatus.Text = $"Added skill {selected.SkillId}.";
    }

    private void RemoveSelectedSkill()
    {
        if (GridSelected.SelectedItem is not RandomSkillGroupSlotRow selected)
        {
            return;
        }

        if (_selectedBySkillId.Remove(selected.SkillId))
        {
            RebuildSelectedRows();
            TxtStatus.Text = $"Removed skill {selected.SkillId}.";
        }
    }

    private void RebuildSelectedRows()
    {
        _selectedRows.Clear();
        var ordered = _selectedBySkillId
            .Values
            .OrderBy(x => x.SkillId)
            .ToList();

        for (var i = 0; i < ordered.Count; i++)
        {
            _selectedRows.Add(new RandomSkillGroupSlotRow
            {
                SlotIndex = i + 1,
                SkillId = ordered[i].SkillId,
                SkillName = ordered[i].SkillName,
                TooltipMarkup = ordered[i].TooltipMarkup,
                TooltipPlain = ordered[i].TooltipPlain,
                Icon = ordered[i].Icon
            });
        }

        UpdateUseCode();
    }

    private void UpdateUseCode()
    {
        TxtUseCode.Text = $"UseCode: {_selectedRows.Count}";
    }

    private BitmapImage? ResolveSkillIcon(string? iconFileName)
    {
        return ImageLoader.TryLoad(_assetResolver.ResolveSkillIcon(iconFileName)) ?? _fallbackIcon;
    }

    private static string BuildTooltipPlain(string? markup, int skillId, string skillName)
    {
        if (string.IsNullOrWhiteSpace(markup))
        {
            return $"Skill {skillId} | {skillName}";
        }

        var plain = AnyTagRegex.Replace(markup, string.Empty).Replace("\r", string.Empty).Trim();
        return string.IsNullOrWhiteSpace(plain)
            ? $"Skill {skillId} | {skillName}"
            : plain;
    }

    private async void BtnSearch_Click(object sender, RoutedEventArgs e)
    {
        await SearchAsync();
    }

    private async void TxtSearch_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter)
        {
            return;
        }

        e.Handled = true;
        await SearchAsync();
    }

    private void GridSearch_MouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        AddSelectedSearchSkill();
    }

    private void GridSelected_MouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        RemoveSelectedSkill();
    }

    private void BtnAdd_Click(object sender, RoutedEventArgs e)
    {
        AddSelectedSearchSkill();
    }

    private void BtnRemove_Click(object sender, RoutedEventArgs e)
    {
        RemoveSelectedSkill();
    }

    private void BtnClear_Click(object sender, RoutedEventArgs e)
    {
        _selectedBySkillId.Clear();
        RebuildSelectedRows();
        TxtStatus.Text = "Selection cleared.";
    }

    private void BtnApply_Click(object sender, RoutedEventArgs e)
    {
        var sortedSkillIds = _selectedRows
            .Select(x => x.SkillId)
            .Distinct()
            .OrderBy(x => x)
            .ToArray();

        Result = new RandomSkillGroupEditModel(_groupId, sortedSkillIds.Length, sortedSkillIds);
        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}
