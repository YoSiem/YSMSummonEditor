using System.Windows;
using YSMSummonEditor.Windows;

namespace YSMSummonEditor.Services;

public sealed class AppDialogService : IAppDialogService
{
    public DialogResult Show(
        Window? owner,
        string title,
        string message,
        DialogButtons buttons = DialogButtons.Ok,
        string? details = null,
        DialogResult defaultResult = DialogResult.Ok)
    {
        var dialog = new DarkDialogWindow(title, message, buttons, details, defaultResult)
        {
            Owner = owner
        };

        _ = dialog.ShowDialog();
        return dialog.Result;
    }
}
