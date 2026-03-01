using CommunityToolkit.Mvvm.ComponentModel;

namespace YSMSummonEditor.Models;

public sealed partial class CreatureLevelBonusEditRow(
    int stageIndex,
    int lookupId,
    bool existsInDb,
    decimal originalStr,
    decimal originalVit,
    decimal originalDex,
    decimal originalAgi,
    decimal originalInt,
    decimal originalMen,
    decimal originalLuk) : ObservableObject
{
    public int StageIndex { get; } = stageIndex;
    public int LookupId { get; } = lookupId;
    public bool ExistsInDb { get; } = existsInDb;

    public decimal OriginalStr { get; } = originalStr;
    public decimal OriginalVit { get; } = originalVit;
    public decimal OriginalDex { get; } = originalDex;
    public decimal OriginalAgi { get; } = originalAgi;
    public decimal OriginalInt { get; } = originalInt;
    public decimal OriginalMen { get; } = originalMen;
    public decimal OriginalLuk { get; } = originalLuk;

    [ObservableProperty]
    private decimal _str = originalStr;

    [ObservableProperty]
    private decimal _vit = originalVit;

    [ObservableProperty]
    private decimal _dex = originalDex;

    [ObservableProperty]
    private decimal _agi = originalAgi;

    [ObservableProperty]
    private decimal _int = originalInt;

    [ObservableProperty]
    private decimal _men = originalMen;

    [ObservableProperty]
    private decimal _luk = originalLuk;

    public bool IsDirty =>
        Str != OriginalStr ||
        Vit != OriginalVit ||
        Dex != OriginalDex ||
        Agi != OriginalAgi ||
        Int != OriginalInt ||
        Men != OriginalMen ||
        Luk != OriginalLuk;

    partial void OnStrChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnVitChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnDexChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnAgiChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnIntChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnMenChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
    partial void OnLukChanged(decimal value) => OnPropertyChanged(nameof(IsDirty));
}
