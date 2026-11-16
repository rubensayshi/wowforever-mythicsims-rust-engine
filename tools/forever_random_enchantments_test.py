from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import forever_random_enchantments as rolls


def bonus(list_id, kind, *values, order=0):
    values = list(values) + [0] * (4 - len(values))
    row = {"ID": str(list_id * 10 + order), "ParentItemBonusListID": str(list_id), "Type": str(kind), "OrderIndex": str(order)}
    row.update({f"Value_{i}": str(v) for i, v in enumerate(values)})
    return row


def node(tree, child_list=0, child_tree=0, context=0):
    return {
        "ParentItemBonusTreeID": str(tree),
        "ChildItemBonusListID": str(child_list),
        "ChildItemBonusTreeID": str(child_tree),
        "ItemContext": str(context),
    }


def points(level, good, superior):
    row = {"ID": str(level)}
    for column in range(5):
        row[f"Good_{column}"] = str(good[column])
        row[f"Superior_{column}"] = str(superior[column])
        row[f"Epic_{column}"] = "0"
    return row


def item(item_id, kind, level, stated=None, **extra):
    scaling = {"ilvl": level}
    if stated:
        scaling["rand_prop_points"] = stated
    return {"id": item_id, "name": f"Item {item_id}", "type": kind, "scaling_options": {"0": scaling}, **extra}


TABLES = {
    "ItemNameDescription": [
        {"ID": "1", "Description_lang": "of the Sorcerer"},
        {"ID": "2", "Description_lang": "of Eluding"},
        {"ID": "3", "Description_lang": "Molten"},
        {"ID": "4", "Description_lang": "of Regeneration"},
    ],
    "ItemEffect": [{"ID": "213334", "SpellID": "13669"}],
    "ItemBonus": [
        bonus(100, 2, 5, 7266),
        bonus(100, 2, 45, 3725, order=1),
        bonus(100, 5, 1, order=2),
        bonus(100, 54, order=3),
        bonus(101, 2, 3, 3333),
        bonus(101, 23, 213334, order=1),
        bonus(101, 5, 2, order=2),
        bonus(102, 5, 3),
        bonus(103, 2, 46, 8000),
        bonus(103, 5, 4, order=1),
        bonus(104, 2, 38, 20000),
        bonus(104, 5, 1, order=1),
    ],
    "ItemBonusTreeNode": [
        node(1, child_list=100),
        node(1, child_tree=2),
        node(2, child_list=101),
        node(2, child_list=102),
        node(2, child_list=103),
        node(2, child_list=104, context=21),
    ],
    "ItemXBonusTree": [
        {"ItemID": "10", "ItemBonusTreeID": "1"},
        {"ItemID": "11", "ItemBonusTreeID": "2"},
        {"ItemID": "12", "ItemBonusTreeID": "1"},
        {"ItemID": "13", "ItemBonusTreeID": "1"},
    ],
    "RandPropPoints": [points(50, [21, 16, 12, 9, 7], [28, 21, 16, 12, 9])],
    "ItemSparse": [{"ID": "10", "OverallQualityID": "2", "InventoryType": "8"}],
}
ITEMS = {
    10: item(10, "ItemTypeFeet", 50, stated=16),
    # A rare one hand weapon ItemSparse does not serve, with no points on its row.
    11: item(11, "ItemTypeWeapon", 50, hand_type="HandTypeOneHand"),
    # No quality anywhere: left out.
    13: item(13, "ItemTypeFinger", 50),
}


class GenerateTest(unittest.TestCase):
    def test_reads_lists_pools_and_points(self):
        enchantments, items, report = rolls.generate(TABLES, ITEMS, {"11": 3})
        self.assertEqual(
            enchantments,
            [
                {
                    "id": 100,
                    "name": "of the Sorcerer",
                    "allocations": [["Intellect", 7266], ["SpellDamage", 3725], ["HealingPower", 3725]],
                },
                {"id": 101, "name": "of Eluding", "allocations": [["Agility", 3333]], "stats": [["DodgeRating", 12]]},
                {"id": 103, "name": "of Regeneration", "unmodeled": [["Health per 5 sec", 8000]]},
            ],
        )
        self.assertEqual(
            items,
            [
                {"id": 10, "points": 16, "enchantments": [100, 101, 103]},
                {"id": 11, "points": 12, "enchantments": [101, 103]},
            ],
        )
        self.assertEqual(report, {"no_item_row": 1, "no_points": [(13, "Item 13")], "disagree": []})

    def test_reports_points_that_disagree(self):
        items = {10: item(10, "ItemTypeFeet", 50, stated=17)}
        _, _, report = rolls.generate(TABLES, items)
        self.assertEqual(report["disagree"], [(10, "Item 10", 17, 16)])

    def test_refuses_an_unknown_stat(self):
        tables = dict(TABLES, ItemBonus=TABLES["ItemBonus"] + [bonus(100, 2, 99, 1, order=9)])
        with self.assertRaises(rolls.Problem):
            rolls.generate(tables, ITEMS)

    def test_weapon_columns(self):
        self.assertEqual(rolls.row_column(item(1, "ItemTypeWeapon", 1, hand_type="HandTypeTwoHand")), 0)
        self.assertEqual(rolls.row_column(item(1, "ItemTypeWeapon", 1, hand_type="HandTypeOffHand")), 2)
        self.assertEqual(rolls.row_column(item(1, "ItemTypeWeapon", 1, hand_type="HandTypeMainHand")), 3)
        self.assertEqual(rolls.row_column(item(1, "ItemTypeRanged", 1)), 4)


if __name__ == "__main__":
    unittest.main()
