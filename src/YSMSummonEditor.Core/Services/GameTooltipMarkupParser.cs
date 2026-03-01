using System.Globalization;
using System.Text.RegularExpressions;
using YSMSummonEditor.Core.Models;

namespace YSMSummonEditor.Core.Services;

public sealed class GameTooltipMarkupParser
{
    private static readonly Regex TagRegex = new("<[^>]+>", RegexOptions.Compiled | RegexOptions.CultureInvariant | RegexOptions.IgnoreCase);

    public IReadOnlyList<TooltipTextSegment> Parse(string? markup)
    {
        if (string.IsNullOrWhiteSpace(markup))
        {
            return [];
        }

        var result = new List<TooltipTextSegment>();
        var source = markup.Replace("\r", string.Empty);

        var boldDepth = 0;
        var italicDepth = 0;
        var underlineDepth = 0;
        var currentSize = default(double?);
        var currentColor = default(string?);
        var sizeStack = new Stack<double?>();
        var colorStack = new Stack<string?>();

        var index = 0;
        foreach (Match match in TagRegex.Matches(source))
        {
            if (match.Index > index)
            {
                AppendText(result, source[index..match.Index], boldDepth, italicDepth, underlineDepth, currentSize, currentColor);
            }

            var token = match.Value[1..^1].Trim();
            HandleTag(token, ref boldDepth, ref italicDepth, ref underlineDepth, ref currentSize, ref currentColor, sizeStack, colorStack, result);
            index = match.Index + match.Length;
        }

        if (index < source.Length)
        {
            AppendText(result, source[index..], boldDepth, italicDepth, underlineDepth, currentSize, currentColor);
        }

        return result;
    }

    public string ToPlainText(string? markup)
    {
        var segments = Parse(markup);
        return string.Concat(segments.Select(x => x.Text));
    }

    private static void HandleTag(
        string token,
        ref int boldDepth,
        ref int italicDepth,
        ref int underlineDepth,
        ref double? currentSize,
        ref string? currentColor,
        Stack<double?> sizeStack,
        Stack<string?> colorStack,
        ICollection<TooltipTextSegment> output)
    {
        if (string.IsNullOrWhiteSpace(token))
        {
            return;
        }

        var normalized = token.ToUpperInvariant();
        if (normalized is "BR" or "BR/" or "/BR")
        {
            output.Add(new TooltipTextSegment(
                "\n",
                boldDepth > 0,
                italicDepth > 0,
                underlineDepth > 0,
                currentSize,
                currentColor));
            return;
        }

        if (normalized == "B")
        {
            boldDepth++;
            return;
        }

        if (normalized == "/B")
        {
            boldDepth = Math.Max(0, boldDepth - 1);
            return;
        }

        if (normalized == "I")
        {
            italicDepth++;
            return;
        }

        if (normalized == "/I")
        {
            italicDepth = Math.Max(0, italicDepth - 1);
            return;
        }

        if (normalized == "U")
        {
            underlineDepth++;
            return;
        }

        if (normalized == "/U")
        {
            underlineDepth = Math.Max(0, underlineDepth - 1);
            return;
        }

        if (normalized.StartsWith("SIZE=", StringComparison.Ordinal))
        {
            var raw = token[5..].Trim();
            if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) && parsed > 0)
            {
                sizeStack.Push(currentSize);
                currentSize = parsed;
            }

            return;
        }

        if (normalized == "/SIZE")
        {
            currentSize = sizeStack.Count > 0 ? sizeStack.Pop() : null;
            return;
        }

        if (token.StartsWith('#'))
        {
            var hex = token[1..].Trim();
            if (IsValidHexColor(hex))
            {
                colorStack.Push(currentColor);
                currentColor = "#" + hex.ToUpperInvariant();
            }

            return;
        }

        if (token.StartsWith("/#", StringComparison.Ordinal))
        {
            currentColor = colorStack.Count > 0 ? colorStack.Pop() : null;
        }
    }

    private static void AppendText(
        ICollection<TooltipTextSegment> output,
        string text,
        int boldDepth,
        int italicDepth,
        int underlineDepth,
        double? currentSize,
        string? currentColor)
    {
        if (string.IsNullOrEmpty(text))
        {
            return;
        }

        output.Add(new TooltipTextSegment(
            text,
            boldDepth > 0,
            italicDepth > 0,
            underlineDepth > 0,
            currentSize,
            currentColor));
    }

    private static bool IsValidHexColor(string raw)
    {
        if (raw.Length is not (6 or 8))
        {
            return false;
        }

        foreach (var c in raw)
        {
            var isHex = (c >= '0' && c <= '9')
                || (c >= 'A' && c <= 'F')
                || (c >= 'a' && c <= 'f');
            if (!isHex)
            {
                return false;
            }
        }

        return true;
    }
}
