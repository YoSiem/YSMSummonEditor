using CommunityToolkit.Mvvm.ComponentModel;

namespace YSMSummonEditor.Models;

public sealed partial class BaseStatEditRow(string statName, int originalValue) : ObservableObject
{
    public string StatName { get; } = statName;
    public int OriginalValue { get; } = originalValue;

    [ObservableProperty]
    private int _currentValue = originalValue;

    public bool IsDirty => CurrentValue != OriginalValue;

    partial void OnCurrentValueChanged(int value)
    {
        OnPropertyChanged(nameof(IsDirty));
    }
}
