using System.Globalization;
using System.Windows;
using System.Windows.Controls;

namespace YSMSummonEditor.Helpers;

public static class DataGridColumnAutoFit
{
    private sealed record ColumnRule(int Index, bool IsFixed, double Value, double MinWidth);

    public static readonly DependencyProperty LayoutSpecProperty = DependencyProperty.RegisterAttached(
        "LayoutSpec",
        typeof(string),
        typeof(DataGridColumnAutoFit),
        new PropertyMetadata(string.Empty, OnLayoutSpecChanged));

    private static readonly DependencyProperty IsHookedProperty = DependencyProperty.RegisterAttached(
        "IsHooked",
        typeof(bool),
        typeof(DataGridColumnAutoFit),
        new PropertyMetadata(false));

    public static string GetLayoutSpec(DependencyObject obj) => (string)obj.GetValue(LayoutSpecProperty);

    public static void SetLayoutSpec(DependencyObject obj, string value) => obj.SetValue(LayoutSpecProperty, value);

    private static bool GetIsHooked(DependencyObject obj) => (bool)obj.GetValue(IsHookedProperty);

    private static void SetIsHooked(DependencyObject obj, bool value) => obj.SetValue(IsHookedProperty, value);

    private static void OnLayoutSpecChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not DataGrid grid)
        {
            return;
        }

        var hasSpec = !string.IsNullOrWhiteSpace(e.NewValue as string);
        var isHooked = GetIsHooked(grid);

        if (hasSpec && !isHooked)
        {
            grid.Loaded += GridLayoutChanged;
            grid.SizeChanged += GridLayoutChanged;
            SetIsHooked(grid, true);
        }
        else if (!hasSpec && isHooked)
        {
            grid.Loaded -= GridLayoutChanged;
            grid.SizeChanged -= GridLayoutChanged;
            SetIsHooked(grid, false);
        }

        if (hasSpec)
        {
            Apply(grid);
        }
    }

    private static void GridLayoutChanged(object sender, RoutedEventArgs e)
    {
        if (sender is DataGrid grid)
        {
            Apply(grid);
        }
    }

    private static void Apply(DataGrid grid)
    {
        var spec = GetLayoutSpec(grid);
        if (string.IsNullOrWhiteSpace(spec) || grid.Columns.Count == 0 || grid.ActualWidth <= 0)
        {
            return;
        }

        var rules = ParseRules(spec, grid.Columns.Count);
        if (rules.Count == 0)
        {
            return;
        }

        var rowHeaderWidth = grid.HeadersVisibility.HasFlag(DataGridHeadersVisibility.Row)
            ? Math.Max(grid.RowHeaderWidth, 22d)
            : 0d;
        var available = grid.ActualWidth - rowHeaderWidth - SystemParameters.VerticalScrollBarWidth - 10d;
        if (available <= 0)
        {
            return;
        }

        var fixedWidth = 0d;
        var weightedMinWidth = 0d;
        var weightSum = 0d;

        foreach (var rule in rules)
        {
            if (rule.IsFixed)
            {
                fixedWidth += Math.Max(rule.Value, rule.MinWidth);
            }
            else
            {
                weightedMinWidth += rule.MinWidth;
                weightSum += Math.Max(0.0001d, rule.Value);
            }
        }

        var freeWidth = Math.Max(0d, available - fixedWidth - weightedMinWidth);

        foreach (var rule in rules)
        {
            if (rule.Index < 0 || rule.Index >= grid.Columns.Count)
            {
                continue;
            }

            var column = grid.Columns[rule.Index];
            double width;
            if (rule.IsFixed)
            {
                width = Math.Max(rule.Value, rule.MinWidth);
            }
            else
            {
                var normalizedWeight = Math.Max(0.0001d, rule.Value) / weightSum;
                width = rule.MinWidth + (freeWidth * normalizedWeight);
            }

            column.Width = new DataGridLength(width);
        }
    }

    private static List<ColumnRule> ParseRules(string spec, int fallbackColumnCount)
    {
        var result = new List<ColumnRule>();
        var sections = spec.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        foreach (var section in sections)
        {
            var parts = section.Split('|', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            if (parts.Length < 3)
            {
                continue;
            }

            if (!int.TryParse(parts[0], NumberStyles.Integer, CultureInfo.InvariantCulture, out var index))
            {
                continue;
            }

            var mode = parts[1];
            if (!double.TryParse(parts[2], NumberStyles.Float, CultureInfo.InvariantCulture, out var value))
            {
                continue;
            }

            var minWidth = 42d;
            if (parts.Length >= 4)
            {
                _ = double.TryParse(parts[3], NumberStyles.Float, CultureInfo.InvariantCulture, out minWidth);
            }

            var isFixed = string.Equals(mode, "f", StringComparison.OrdinalIgnoreCase);
            result.Add(new ColumnRule(index, isFixed, value, minWidth));
        }

        if (result.Count > 0)
        {
            return result;
        }

        for (var i = 0; i < fallbackColumnCount; i++)
        {
            result.Add(new ColumnRule(i, false, 1d, 42d));
        }

        return result;
    }
}
