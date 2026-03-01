using System.IO;
using System.Linq;
using System.Data.Common;
using YSMLib.AppSettings.Models;

namespace YSMSummonEditor.Helpers;

public static class LocalEnvConnectionLoader
{
    private const string ConnectionStringKey = "YSM_DB_CONNECTION_STRING";
    private const string EnvFileName = ".env.local";

    public static bool TryLoadDbSettings(out DbConnectionSettings settings, out string? envPath)
    {
        settings = new DbConnectionSettings();
        envPath = FindEnvLocalPath();
        if (string.IsNullOrWhiteSpace(envPath) || !File.Exists(envPath))
        {
            return false;
        }

        if (envPath is null)
        {
            return false;
        }

        var connectionString = ReadConnectionString(envPath);
        if (string.IsNullOrWhiteSpace(connectionString))
        {
            return false;
        }

        return TryParseConnectionString(connectionString, out settings);
    }

    private static string? FindEnvLocalPath()
    {
        var roots = new[]
        {
            Directory.GetCurrentDirectory(),
            AppContext.BaseDirectory
        };

        foreach (var root in roots.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            var directory = new DirectoryInfo(root);
            while (directory is not null)
            {
                var candidate = Path.Combine(directory.FullName, EnvFileName);
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }
        }

        return null;
    }

    private static string? ReadConnectionString(string path)
    {
        foreach (var rawLine in File.ReadLines(path))
        {
            var line = rawLine.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            var separatorIndex = line.IndexOf('=');
            if (separatorIndex <= 0)
            {
                continue;
            }

            var key = line[..separatorIndex].Trim();
            if (!string.Equals(key, ConnectionStringKey, StringComparison.Ordinal))
            {
                continue;
            }

            var value = line[(separatorIndex + 1)..].Trim();
            if (value.Length >= 2 && value.StartsWith('"') && value.EndsWith('"'))
            {
                value = value[1..^1];
            }

            return value;
        }

        return null;
    }

    private static bool TryParseConnectionString(string connectionString, out DbConnectionSettings settings)
    {
        settings = new DbConnectionSettings();

        try
        {
            var builder = new DbConnectionStringBuilder
            {
                ConnectionString = connectionString
            };

            var serverRaw = GetString(builder, "Server",
                GetString(builder, "Data Source", settings.Server));

            settings.Server = serverRaw;
            if (!string.IsNullOrWhiteSpace(serverRaw))
            {
                var split = serverRaw.Split(',', 2, StringSplitOptions.TrimEntries);
                settings.Server = split[0];
                if (split.Length > 1 && int.TryParse(split[1], out var parsedPort))
                {
                    settings.Port = parsedPort;
                }
            }

            settings.Database = GetString(builder, "Database",
                GetString(builder, "Initial Catalog", settings.Database));
            settings.UserId = GetString(builder, "User Id",
                GetString(builder, "User ID",
                GetString(builder, "UID",
                GetString(builder, "User", settings.UserId))));
            settings.Password = GetString(builder, "Password",
                GetString(builder, "Pwd", settings.Password));
            settings.TrustServerCertificate = GetBool(builder, "TrustServerCertificate", settings.TrustServerCertificate);

            return !string.IsNullOrWhiteSpace(settings.Server)
                && !string.IsNullOrWhiteSpace(settings.Database)
                && !string.IsNullOrWhiteSpace(settings.UserId);
        }
        catch
        {
            return false;
        }
    }

    private static string GetString(DbConnectionStringBuilder builder, string key, string fallback)
    {
        if (!builder.TryGetValue(key, out var value) || value is null)
        {
            return fallback;
        }

        var text = Convert.ToString(value)?.Trim();
        return string.IsNullOrWhiteSpace(text) ? fallback : text;
    }

    private static bool GetBool(DbConnectionStringBuilder builder, string key, bool fallback)
    {
        if (!builder.TryGetValue(key, out var value) || value is null)
        {
            return fallback;
        }

        if (value is bool direct)
        {
            return direct;
        }

        return bool.TryParse(Convert.ToString(value), out var parsed) ? parsed : fallback;
    }
}
