using System.Collections.ObjectModel;
using System.Windows;
using YSMLib.ChangeTracking.Models;
using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Core.Models;
using YSMSummonEditor.Core.Services;
using YSMSummonEditor.Models;

namespace YSMSummonEditor.Windows;

public partial class SkillEditWindow : Window
{
    private readonly ObservableCollection<EditableFieldRow> _businessRows;
    private readonly ObservableCollection<EditableFieldRow> _rawRows;
    private readonly ISkillUpdateBuilder _skillUpdateBuilder;

    public SkillEditWindow(
        SkillEditableSnapshot snapshot,
        ISkillUpdateBuilder skillUpdateBuilder,
        int skillId)
    {
        InitializeComponent();
        _skillUpdateBuilder = skillUpdateBuilder;

        TxtHeader.Text = $"Skill {skillId}";

        _businessRows = new ObservableCollection<EditableFieldRow>(
            snapshot.BusinessFields
                .OrderBy(x => x.Key, StringComparer.OrdinalIgnoreCase)
                .Select(x => new EditableFieldRow(
                    x.Key,
                    x.Value,
                    snapshot.FieldTypes.TryGetValue(x.Key, out var type) ? type : typeof(string),
                    isBusiness: true,
                    isReadOnly: string.Equals(x.Key, "id", StringComparison.OrdinalIgnoreCase))));

        _rawRows = new ObservableCollection<EditableFieldRow>(
            snapshot.RawFields
                .OrderBy(x => x.Key, StringComparer.OrdinalIgnoreCase)
                .Select(x => new EditableFieldRow(
                    x.Key,
                    x.Value,
                    snapshot.FieldTypes.TryGetValue(x.Key, out var type) ? type : typeof(string),
                    isBusiness: snapshot.BusinessFields.ContainsKey(x.Key),
                    isReadOnly: string.Equals(x.Key, "id", StringComparison.OrdinalIgnoreCase))));

        GridBusiness.ItemsSource = _businessRows;
        GridRaw.ItemsSource = _rawRows;

        SkillId = skillId;
    }

    public int SkillId { get; }

    public SkillUpdate? ResultUpdate { get; private set; }

    public IReadOnlyList<PendingFieldChange> ResultFieldChanges { get; private set; } = [];

    private void BtnApply_Click(object sender, RoutedEventArgs e)
    {
        var dirtyRaw = _rawRows
            .Where(x => x.IsDirty && !x.IsReadOnly)
            .Select(x => new EditableFieldDraft
            {
                Name = x.Name,
                ValueType = x.ValueType,
                OriginalValue = x.OriginalValue,
                CurrentValue = x.CurrentValue
            })
            .ToList();

        ResultUpdate = _skillUpdateBuilder.Build(SkillId, dirtyRaw);
        ResultFieldChanges = dirtyRaw
            .Select(x => new PendingFieldChange("SkillResource", SkillId.ToString(), x.Name, x.OriginalValue, x.CurrentValue))
            .ToList();

        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}
