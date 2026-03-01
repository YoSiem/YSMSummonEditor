using CommunityToolkit.Mvvm.ComponentModel;

namespace YSMSummonEditor.Models;

public sealed partial class EditableFieldRow : ObservableObject
{
    public EditableFieldRow(string name, object? value, Type valueType, bool isBusiness, bool isReadOnly)
    {
        Name = name;
        OriginalValue = value;
        ValueType = valueType;
        IsBusiness = isBusiness;
        IsReadOnly = isReadOnly;
        _currentValue = value;
    }

    public string Name { get; }
    public object? OriginalValue { get; }
    public Type ValueType { get; }
    public bool IsBusiness { get; }
    public bool IsReadOnly { get; }

    [ObservableProperty]
    private object? _currentValue;

    [ObservableProperty]
    private bool _isDirty;

    partial void OnCurrentValueChanged(object? value)
    {
        IsDirty = !Equals(OriginalValue, value);
    }
}
