using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using YSMLib.AppSettings;
using YSMLib.AssetIndexing;
using YSMLib.GameData.SqlServer.Infrastructure;
using YSMSummonEditor.Core.Services;

namespace YSMSummonEditor;

public partial class App : System.Windows.Application
{
    private IHost? _host;

    protected override async void OnStartup(System.Windows.StartupEventArgs e)
    {
        base.OnStartup(e);

        _host = Host.CreateDefaultBuilder()
            .ConfigureServices(services =>
            {
                services.AddSingleton<ISettingsStore, JsonSettingsStore>();
                services.AddSingleton<IAssetResolver, FileSystemAssetResolver>();
                services.AddSingleton<SqlConnectionFactory>();
                services.AddSingleton<IValueConversionService, ValueConversionService>();
                services.AddSingleton<ISkillUpdateBuilder, SkillUpdateBuilder>();
                services.AddSingleton<MainWindow>();
            })
            .Build();

        await _host.StartAsync();

        var mainWindow = _host.Services.GetRequiredService<MainWindow>();
        mainWindow.Show();
    }

    protected override async void OnExit(System.Windows.ExitEventArgs e)
    {
        if (_host is not null)
        {
            await _host.StopAsync();
            _host.Dispose();
        }

        base.OnExit(e);
    }
}
