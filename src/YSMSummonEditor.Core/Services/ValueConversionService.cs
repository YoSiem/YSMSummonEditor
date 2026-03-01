using System.Globalization;

namespace YSMSummonEditor.Core.Services;

public interface IValueConversionService
{
    object? ConvertTo(Type targetType, object? rawValue);
}

public sealed class ValueConversionService : IValueConversionService
{
    public object? ConvertTo(Type targetType, object? rawValue)
    {
        if (rawValue is null)
        {
            return null;
        }

        var normalizedType = Nullable.GetUnderlyingType(targetType) ?? targetType;

        if (normalizedType.IsAssignableFrom(rawValue.GetType()))
        {
            return rawValue;
        }

        var rawText = rawValue.ToString()?.Trim();
        if (string.IsNullOrWhiteSpace(rawText))
        {
            return normalizedType.IsValueType ? Activator.CreateInstance(normalizedType) : null;
        }

        if (normalizedType == typeof(bool))
        {
            if (rawText is "1")
            {
                return true;
            }

            if (rawText is "0")
            {
                return false;
            }
        }

        return Convert.ChangeType(rawText, normalizedType, CultureInfo.InvariantCulture);
    }
}
