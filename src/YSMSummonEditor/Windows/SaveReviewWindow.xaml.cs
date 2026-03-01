using System.Collections.ObjectModel;
using System.Windows;

namespace YSMSummonEditor.Windows;

public sealed class SaveReviewRow
{
    public string ChangeType { get; init; } = string.Empty;
    public string Target { get; init; } = string.Empty;
    public string Field { get; init; } = string.Empty;
    public string Before { get; init; } = string.Empty;
    public string After { get; init; } = string.Empty;
}

public partial class SaveReviewWindow : Window
{
    public SaveReviewWindow(IEnumerable<SaveReviewRow> rows)
    {
        InitializeComponent();
        ListChanges.ItemsSource = new ObservableCollection<SaveReviewRow>(rows);
    }

    private void BtnConfirm_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}
