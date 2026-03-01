using YSMLib.GameData.Abstractions.Models;
using YSMSummonEditor.Core.Models;

namespace YSMSummonEditor.Core.Services;

public interface ISkillUpdateBuilder
{
    SkillUpdate Build(int skillId, IReadOnlyCollection<EditableFieldDraft> fields);
}

public sealed class SkillUpdateBuilder(IValueConversionService conversionService) : ISkillUpdateBuilder
{
    private readonly IValueConversionService _conversionService = conversionService;

    public SkillUpdate Build(int skillId, IReadOnlyCollection<EditableFieldDraft> fields)
    {
        var updated = new Dictionary<string, object?>(StringComparer.OrdinalIgnoreCase);
        var types = new Dictionary<string, Type>(StringComparer.OrdinalIgnoreCase);
        var original = new Dictionary<string, object?>(StringComparer.OrdinalIgnoreCase);

        foreach (var field in fields.Where(x => x.IsDirty))
        {
            var convertedValue = _conversionService.ConvertTo(field.ValueType, field.CurrentValue);
            updated[field.Name] = convertedValue;
            types[field.Name] = field.ValueType;
            original[field.Name] = field.OriginalValue;
        }

        return new SkillUpdate(skillId, updated, types, original);
    }
}
