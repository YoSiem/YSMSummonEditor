using System.Windows.Media.Imaging;
using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Models;

public sealed class SummonDisplayItem(SummonListItem summon)
{
    public SummonListItem Summon { get; } = summon;

    public BitmapImage? Icon { get; set; }

    public string DisplayName => $"{Summon.SummonName} ({Summon.SummonId})";
}
