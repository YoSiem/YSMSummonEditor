using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Media;
using YSMSummonEditor.Core.Services;

namespace YSMSummonEditor.Converters;

public sealed class GameTooltipToToolTipConverter : IMultiValueConverter
{
    private static readonly GameTooltipMarkupParser Parser = new();

    public object Convert(object[] values, Type targetType, object parameter, CultureInfo culture)
    {
        var markup = values.Length > 0 ? values[0] as string : null;
        var plainFallback = values.Length > 1 ? values[1] as string : null;
        var source = string.IsNullOrWhiteSpace(markup) ? plainFallback : markup;
        if (string.IsNullOrWhiteSpace(source))
        {
            return string.Empty;
        }

        var segments = Parser.Parse(source);
        if (segments.Count == 0)
        {
            return plainFallback ?? string.Empty;
        }

        var textBlock = new TextBlock
        {
            TextWrapping = TextWrapping.Wrap,
            MaxWidth = 620
        };

        foreach (var segment in segments)
        {
            AppendSegment(textBlock.Inlines, segment.Text, segment.Bold, segment.Italic, segment.Underline, segment.FontSize, segment.ColorHex);
        }

        return textBlock;
    }

    public object[] ConvertBack(object value, Type[] targetTypes, object parameter, CultureInfo culture)
    {
        throw new NotSupportedException();
    }

    private static void AppendSegment(
        InlineCollection target,
        string text,
        bool bold,
        bool italic,
        bool underline,
        double? fontSize,
        string? colorHex)
    {
        if (string.IsNullOrEmpty(text))
        {
            return;
        }

        var lines = text.Split('\n');
        for (var i = 0; i < lines.Length; i++)
        {
            var part = lines[i];
            if (part.Length > 0)
            {
                var run = new Run(part)
                {
                    FontWeight = bold ? FontWeights.Bold : FontWeights.Normal,
                    FontStyle = italic ? FontStyles.Italic : FontStyles.Normal
                };

                if (underline)
                {
                    run.TextDecorations = TextDecorations.Underline;
                }

                if (fontSize is > 0)
                {
                    run.FontSize = fontSize.Value;
                }

                if (TryCreateColorBrush(colorHex, out var brush))
                {
                    run.Foreground = brush;
                }

                target.Add(run);
            }

            if (i < lines.Length - 1)
            {
                target.Add(new LineBreak());
            }
        }
    }

    private static bool TryCreateColorBrush(string? colorHex, out SolidColorBrush brush)
    {
        brush = null!;
        if (string.IsNullOrWhiteSpace(colorHex))
        {
            return false;
        }

        try
        {
            var converted = ColorConverter.ConvertFromString(colorHex);
            if (converted is not Color color)
            {
                return false;
            }

            brush = new SolidColorBrush(color);
            brush.Freeze();
            return true;
        }
        catch
        {
            return false;
        }
    }
}
