namespace YSMSummonEditor.Core.Models;

public sealed class EditableFieldDraft
{
    public required string Name { get; init; }
    public required Type ValueType { get; init; }
    public object? OriginalValue { get; init; }
    public object? CurrentValue { get; set; }

    public bool IsDirty => !Equals(OriginalValue, CurrentValue);
}
