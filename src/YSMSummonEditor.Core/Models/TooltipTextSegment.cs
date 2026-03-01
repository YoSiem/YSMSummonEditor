namespace YSMSummonEditor.Core.Models;

public sealed record TooltipTextSegment(
    string Text,
    bool Bold,
    bool Italic,
    bool Underline,
    double? FontSize,
    string? ColorHex);
