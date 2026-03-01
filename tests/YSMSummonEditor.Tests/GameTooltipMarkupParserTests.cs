using YSMSummonEditor.Core.Services;

namespace YSMSummonEditor.Tests;

public sealed class GameTooltipMarkupParserTests
{
    [Fact]
    public void Parse_SupportsGameTagsAndHexColor()
    {
        var parser = new GameTooltipMarkupParser();
        var input = "<B>Bold</B><BR><#FFAA00><SIZE=18>Gold</SIZE></#FFAA00>";

        var segments = parser.Parse(input);

        Assert.True(segments.Count >= 3);
        Assert.Equal("Bold", segments[0].Text);
        Assert.True(segments[0].Bold);

        Assert.Contains(segments, x => x.Text == "\n");
        Assert.Contains(segments, x =>
            x.Text == "Gold"
            && x.FontSize == 18
            && x.ColorHex == "#FFAA00");
    }

    [Fact]
    public void Parse_IgnoresUnknownTags_AndPreservesText()
    {
        var parser = new GameTooltipMarkupParser();
        var input = "<UNKNOWN>Hello</UNKNOWN> <I>world</I>";

        var segments = parser.Parse(input);
        var combined = string.Concat(segments.Select(x => x.Text));

        Assert.Equal("Hello world", combined);
        Assert.Contains(segments, x => x.Text.Contains("world", StringComparison.Ordinal) && x.Italic);
    }

    [Fact]
    public void ToPlainText_StripsTagsAndKeepsBreaks()
    {
        var parser = new GameTooltipMarkupParser();
        var input = "<B>One</B><BR><#00FF00>Two</#00FF00>";

        var plain = parser.ToPlainText(input);

        Assert.Equal("One\nTwo", plain);
    }
}
