using System.Windows;
using System.Windows.Controls;
using DialogResultType = YSMSummonEditor.Services.DialogResult;
using YSMSummonEditor.Services;

namespace YSMSummonEditor.Windows;

public partial class DarkDialogWindow : Window
{
    private DialogResultType _closeResult = DialogResultType.Cancel;

    public DialogResultType Result { get; private set; } = DialogResultType.None;

    public DarkDialogWindow(
        string title,
        string message,
        DialogButtons buttons,
        string? details,
        DialogResultType defaultResult)
    {
        InitializeComponent();

        Title = title;
        TxtMessage.Text = message;

        if (!string.IsNullOrWhiteSpace(details))
        {
            DetailsExpander.Visibility = Visibility.Visible;
            TxtDetails.Text = details;
        }

        ConfigureButtons(buttons, defaultResult);
    }

    private void ConfigureButtons(DialogButtons buttons, DialogResultType defaultResult)
    {
        HideButton(BtnPrimary);
        HideButton(BtnSecondary);
        HideButton(BtnTertiary);

        switch (buttons)
        {
            case DialogButtons.Ok:
                _closeResult = DialogResultType.Ok;
                ConfigureButton(BtnPrimary, "OK", DialogResultType.Ok, isDefault: true, isCancel: true);
                break;
            case DialogButtons.YesNo:
            {
                _closeResult = DialogResultType.No;
                var yesDefault = defaultResult != DialogResultType.No;
                ConfigureButton(BtnPrimary, "Yes", DialogResultType.Yes, isDefault: yesDefault, isCancel: false);
                ConfigureButton(BtnSecondary, "No", DialogResultType.No, isDefault: !yesDefault, isCancel: true);
                break;
            }
            case DialogButtons.YesNoCancel:
            {
                _closeResult = DialogResultType.Cancel;
                var yesDefault = defaultResult == DialogResultType.Yes || defaultResult == DialogResultType.Ok;
                var noDefault = defaultResult == DialogResultType.No;
                var cancelDefault = defaultResult == DialogResultType.Cancel;

                ConfigureButton(BtnPrimary, "Yes", DialogResultType.Yes, isDefault: yesDefault, isCancel: false);
                ConfigureButton(BtnSecondary, "No", DialogResultType.No, isDefault: noDefault, isCancel: false);
                ConfigureButton(BtnTertiary, "Cancel", DialogResultType.Cancel, isDefault: cancelDefault, isCancel: true);
                break;
            }
            default:
                _closeResult = DialogResultType.Ok;
                ConfigureButton(BtnPrimary, "OK", DialogResultType.Ok, isDefault: true, isCancel: true);
                break;
        }
    }

    private static void HideButton(Button button)
    {
        button.Visibility = Visibility.Collapsed;
        button.IsDefault = false;
        button.IsCancel = false;
        button.Tag = null;
    }

    private static void ConfigureButton(Button button, string text, DialogResultType result, bool isDefault, bool isCancel)
    {
        button.Content = text;
        button.Tag = result;
        button.IsDefault = isDefault;
        button.IsCancel = isCancel;
        button.Visibility = Visibility.Visible;
    }

    private void DialogButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: DialogResultType result })
        {
            return;
        }

        Result = result;
        base.DialogResult = true;
        Close();
    }

    protected override void OnClosing(System.ComponentModel.CancelEventArgs e)
    {
        if (Result == DialogResultType.None)
        {
            Result = _closeResult;
        }

        base.OnClosing(e);
    }
}
