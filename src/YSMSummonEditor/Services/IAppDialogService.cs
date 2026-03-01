using System.Windows;

namespace YSMSummonEditor.Services;

public interface IAppDialogService
{
    DialogResult Show(
        Window? owner,
        string title,
        string message,
        DialogButtons buttons = DialogButtons.Ok,
        string? details = null,
        DialogResult defaultResult = DialogResult.Ok);
}
