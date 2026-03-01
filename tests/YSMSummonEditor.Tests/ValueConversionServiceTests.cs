using YSMSummonEditor.Core.Services;

namespace YSMSummonEditor.Tests;

public sealed class ValueConversionServiceTests
{
    [Fact]
    public void ConvertTo_Decimal_UsesInvariantCulture()
    {
        var service = new ValueConversionService();
        var value = service.ConvertTo(typeof(decimal), "1.25");
        Assert.Equal(1.25m, value);
    }

    [Fact]
    public void ConvertTo_Bool_ParsesNumeric()
    {
        var service = new ValueConversionService();
        Assert.Equal(true, service.ConvertTo(typeof(bool), "1"));
        Assert.Equal(false, service.ConvertTo(typeof(bool), "0"));
    }
}
