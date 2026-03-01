using System.Collections.ObjectModel;
using YSMLib.GameData.Abstractions.Models;

namespace YSMSummonEditor.Models;

public sealed class TreeContextState(
    int stageIndex,
    int treeSlotIndex,
    int summonId,
    int originalSkillTreeId,
    IReadOnlyList<SkillTreeEntrySnapshot> originalEntries)
{
    public int StageIndex { get; } = stageIndex;
    public int TreeSlotIndex { get; } = treeSlotIndex;
    public int SummonId { get; } = summonId;
    public int OriginalSkillTreeId { get; } = originalSkillTreeId;
    public bool CloneIfShared { get; set; }

    public IReadOnlyList<SkillTreeEntrySnapshot> OriginalEntries { get; } = originalEntries;

    public ObservableCollection<SkillTreeEntryRow> CurrentEntries { get; } = new(
        originalEntries.Select(x => new SkillTreeEntryRow(x)));

    public string Header => $"SkillTree {TreeSlotIndex}";
    public string HeaderToolTip => $"Tree {TreeSlotIndex}, SkillTreeId: {OriginalSkillTreeId}";

    public SkillTreeEntryRow? SelectedEntry { get; set; }
}
