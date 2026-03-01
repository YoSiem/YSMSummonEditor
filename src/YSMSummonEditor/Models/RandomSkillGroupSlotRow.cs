using System.Windows.Media.Imaging;

namespace YSMSummonEditor.Models;

public sealed class RandomSkillGroupSlotRow
{
    public int SlotIndex { get; init; }
    public int SkillId { get; init; }
    public string SkillName { get; init; } = string.Empty;
    public string TooltipMarkup { get; init; } = string.Empty;
    public string TooltipPlain { get; init; } = string.Empty;
    public BitmapImage? Icon { get; init; }
}
