using System.Globalization;
using System.Windows;
using YSMLib.AppSettings.Models;
using YSMSummonEditor.Services;

namespace YSMSummonEditor.Windows;

public partial class SettingsWindow : Window
{
    private readonly SummonEditorSettings _working;
    private readonly IAppDialogService _dialogService;

    public SettingsWindow(SummonEditorSettings current, IAppDialogService dialogService)
    {
        InitializeComponent();
        _dialogService = dialogService;
        _working = Clone(current);

        TxtServer.Text = _working.Database.Server;
        TxtPort.Text = _working.Database.Port.ToString(CultureInfo.InvariantCulture);
        TxtDatabase.Text = _working.Database.Database;
        TxtUser.Text = _working.Database.UserId;
        TxtPassword.Password = _working.Database.Password;
        TxtDumpFolder.Text = _working.DumpFolderPath;
        ChkTrustServerCertificate.IsChecked = _working.Database.TrustServerCertificate;
    }

    public SummonEditorSettings ResultSettings => _working;

    private void BtnBrowse_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new Microsoft.Win32.OpenFolderDialog
        {
            Title = "Select dump folder"
        };

        if (dialog.ShowDialog(this) == true)
        {
            TxtDumpFolder.Text = dialog.FolderName;
        }
    }

    private void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        if (!int.TryParse(TxtPort.Text.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var port))
        {
            _dialogService.Show(this, "Validation", "Port must be a number.");
            return;
        }

        _working.Database.Server = TxtServer.Text.Trim();
        _working.Database.Port = port;
        _working.Database.Database = TxtDatabase.Text.Trim();
        _working.Database.UserId = TxtUser.Text.Trim();
        _working.Database.Password = TxtPassword.Password;
        _working.Database.TrustServerCertificate = ChkTrustServerCertificate.IsChecked == true;
        _working.DumpFolderPath = TxtDumpFolder.Text.Trim();

        if (string.IsNullOrWhiteSpace(_working.Database.Server)
            || string.IsNullOrWhiteSpace(_working.Database.Database)
            || string.IsNullOrWhiteSpace(_working.Database.UserId))
        {
            _dialogService.Show(this, "Validation", "Server, Database and User ID are required.");
            return;
        }

        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }

    private static SummonEditorSettings Clone(SummonEditorSettings input)
    {
        return new SummonEditorSettings
        {
            DumpFolderPath = input.DumpFolderPath,
            IncludeNonForm1ByDefault = input.IncludeNonForm1ByDefault,
            IncludeRidingOnlyByDefault = input.IncludeRidingOnlyByDefault,
            ShowAllEvolutionsByDefault = input.ShowAllEvolutionsByDefault,
            SaveToSqlMode = input.SaveToSqlMode,
            Database = new DbConnectionSettings
            {
                Server = input.Database.Server,
                Port = input.Database.Port,
                Database = input.Database.Database,
                UserId = input.Database.UserId,
                Password = input.Database.Password,
                TrustServerCertificate = input.Database.TrustServerCertificate
            }
        };
    }
}
