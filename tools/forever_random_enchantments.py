"""Write the random enchantments ("of the Sorcerer") the Forever client rolls on its items.

The Forever client has no ItemRandomProperties table. An item that reads "<Random enchantment>"
names an item bonus tree (ItemXBonusTree), whose nodes (ItemBonusTreeNode) list the bonus lists
it can roll. A bonus list (ItemBonus rows of one ParentItemBonusListID) holds the suffix name
(type 5, an ItemNameDescription), stat allocations (type 2: an item stat type and a share of the
item's random property points in ten thousandths) and, for "of Eluding", an item effect (type
23). The points are the RandPropPoints cell of the item's level, quality and slot. A stat is
its share of the points, rounded half up: Potent Boots (item level 50, uncommon feet, 16
points) of the Sorcerer (12745: Intellect 7266, Stamina 4106, spell power 3725) reads +12
Intellect, +7 Stamina and 6 spell power, as Wowhead's Forever tooltip of item 15171 with bonus
12745 does.

Two tables, read by src/prepare/forever_random_enchantments.rs:

data/forever-random-enchantments.jsonl, one bonus list per row, keyed by its ID, which a
request states as an item's random_suffix (the IDs start above the reference database's):

    {"id":12745,"name":"of the Sorcerer","allocations":[["Intellect",7266],...]}

  "allocations" pairs a stat of src/prepare/stats.rs with its share. Attack power is stated for
  melee and ranged, spell power for damage and healing. "stats" pairs a stat with a fixed value
  (the 1% dodge of "of Eluding"). "unmodeled" pairs a label with the share of a stat the sim has
  none for (health per 5 sec), for a tooltip alone.

data/forever-random-enchantment-items.jsonl, one item per row:

    {"id":15171,"points":16,"enchantments":[12718,...]}

  Only items the item data holds. "points" is the item row's rand_prop_points where it states
  them and the RandPropPoints cell otherwise; where both exist they must agree. The cell needs
  the item's quality: ItemSparse's, or for an item ItemSparse does not serve the quality of
  Wowhead's Forever tooltip, which fetch keeps in item-qualities.json.

The tables come from wago.tools, product wowf:

    python3 tools/forever_random_enchantments.py fetch DIR [--build 1.60.1.70938]
    python3 tools/forever_random_enchantments.py build DIR
    python3 tools/forever_random_enchantments.py check DIR
"""

import argparse
import csv
import json
import sys
import time
import urllib.error
import urllib.request
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"
ENCHANTMENTS = DATA / "forever-random-enchantments.jsonl"
ITEMS = DATA / "forever-random-enchantment-items.jsonl"
TABLES = [
    "ItemBonus",
    "ItemBonusTreeNode",
    "ItemXBonusTree",
    "ItemNameDescription",
    "RandPropPoints",
    "ItemSparse",
    "ItemEffect",
]
# The item rows preparation reads under Forever's rules, first match first (src/prepare/items.rs).
QUALITY_FILE = "item-qualities.json"
ITEM_FILES = ["forever-live-items.jsonl", "items.jsonl", "forever-client-items.jsonl"]

BONUS_STAT = 2
BONUS_NAME = 5
BONUS_ITEM_EFFECT = 23
# A marker on the caster weapon lists with no values of its own.
BONUS_IGNORED = {54}

# The client's item stat types, as the stats of src/prepare/stats.rs they pay into.
STAT_TYPES = {
    3: ["Agility"],
    4: ["Strength"],
    5: ["Intellect"],
    6: ["Spirit"],
    7: ["Stamina"],
    12: ["DefenseRating"],
    38: ["AttackPower", "RangedAttackPower"],
    41: ["HealingPower"],
    42: ["SpellDamage"],
    43: ["MP5"],
    45: ["SpellDamage", "HealingPower"],
    51: ["FireResistance"],
    52: ["FrostResistance"],
    54: ["ShadowResistance"],
    55: ["NatureResistance"],
    56: ["ArcaneResistance"],
    84: ["HolyDamage"],
    85: ["FireDamage"],
    86: ["NatureDamage"],
    87: ["FrostDamage"],
    88: ["ShadowDamage"],
    89: ["ArcaneDamage"],
}
# Stat types the sim has no stat for, by the label a tooltip gives them.
UNMODELED = {46: "Health per 5 sec"}
# The equip spell of an item effect a suffix adds, as fixed stats: 13669 is Increased Dodge 12,
# "Increases your chance to dodge an attack by 1%", which the reference's "of Eluding" rows
# state as 12 dodge rating.
EFFECT_SPELLS = {13669: [["DodgeRating", 12]]}

QUALITIES = {2: "Good", 3: "Superior", 4: "Epic"}
# RandPropPoints column by the client's inventory type.
INVENTORY_COLUMN = {
    1: 0, 5: 0, 7: 0, 20: 0, 17: 0,
    3: 1, 6: 1, 8: 1, 10: 1, 12: 1,
    2: 2, 9: 2, 11: 2, 16: 2, 14: 2, 23: 2,
    13: 3, 21: 3, 22: 3,
    15: 4, 25: 4, 26: 4, 28: 4,
}  # fmt: skip


# The same column from an item row, for an item the client's ItemSparse table does not serve.
ROW_COLUMN = {
    "ItemTypeHead": 0, "ItemTypeChest": 0, "ItemTypeLegs": 0,
    "ItemTypeShoulder": 1, "ItemTypeWaist": 1, "ItemTypeFeet": 1, "ItemTypeHands": 1,
    "ItemTypeTrinket": 1,
    "ItemTypeNeck": 2, "ItemTypeWrist": 2, "ItemTypeFinger": 2, "ItemTypeBack": 2,
    "ItemTypeRanged": 4,
}  # fmt: skip


def row_column(row):
    """The RandPropPoints column of an item row: a weapon by its hands, a shield or held item
    (an off hand with no damage) with the small armor slots."""
    if row.get("type") != "ItemTypeWeapon":
        return ROW_COLUMN.get(row.get("type"))
    if row.get("hand_type") == "HandTypeTwoHand":
        return 0
    scaling = row.get("scaling_options", {}).get("0", {})
    if row.get("hand_type") == "HandTypeOffHand" and not scaling.get("weapon_damage_max"):
        return 2
    return 3


class Problem(Exception):
    pass


def read(directory, table):
    path = directory / f"{table}.csv"
    if not path.exists():
        raise Problem(f"{path} is missing: run fetch first")
    with open(path, newline="") as handle:
        return list(csv.DictReader(handle))


def bonus_lists(rows):
    lists = defaultdict(list)
    for row in sorted(rows, key=lambda r: (int(r["ParentItemBonusListID"]), int(r["OrderIndex"]), int(r["ID"]))):
        values = [int(row[f"Value_{i}"]) for i in range(4)]
        lists[int(row["ParentItemBonusListID"])].append((int(row["Type"]), values))
    return lists


def enchantment(list_id, bonuses, names, effects):
    """The row of a bonus list that names a suffix, or None for any other list."""
    name = next((names.get(v[0], "") for kind, v in bonuses if kind == BONUS_NAME), "")
    if not name.startswith("of "):
        return None
    allocations, fixed, unmodeled = [], [], []
    for kind, values in bonuses:
        if kind == BONUS_NAME or kind in BONUS_IGNORED:
            continue
        if kind == BONUS_STAT:
            stat, share = values[0], values[1]
            if stat in UNMODELED:
                unmodeled.append([UNMODELED[stat], share])
            elif stat in STAT_TYPES:
                allocations += [[name_, share] for name_ in STAT_TYPES[stat]]
            else:
                raise Problem(f"bonus list {list_id} ({name}) has unknown stat type {stat}")
        elif kind == BONUS_ITEM_EFFECT:
            spell = effects.get(values[0])
            if spell not in EFFECT_SPELLS:
                raise Problem(f"bonus list {list_id} ({name}) adds item effect {values[0]}, spell {spell}")
            fixed += EFFECT_SPELLS[spell]
        else:
            raise Problem(f"bonus list {list_id} ({name}) has unknown bonus type {kind}")
    row = {"id": list_id, "name": name}
    if allocations:
        row["allocations"] = allocations
    if fixed:
        row["stats"] = fixed
    if unmodeled:
        row["unmodeled"] = unmodeled
    return row


def tree_lists(tree, nodes, seen=()):
    """The bonus lists a tree rolls anywhere, its child trees included."""
    out = []
    for node in nodes.get(tree, []):
        if int(node["ItemContext"]) != 0:
            continue
        if int(node["ChildItemBonusListID"]):
            out.append(int(node["ChildItemBonusListID"]))
        child = int(node["ChildItemBonusTreeID"])
        if child and child not in seen:
            out += tree_lists(child, nodes, seen + (tree,))
    return out


def item_rows():
    rows = {}
    for name in ITEM_FILES:
        with open(DATA / name) as handle:
            for line in handle:
                row = json.loads(line)
                rows.setdefault(row["id"], row)
    return rows


def generate(tables, items, qualities=None):
    names = {int(r["ID"]): r["Description_lang"] for r in tables["ItemNameDescription"]}
    effects = {int(r["ID"]): int(r["SpellID"]) for r in tables["ItemEffect"]}
    enchantments = {}
    for list_id, bonuses in bonus_lists(tables["ItemBonus"]).items():
        row = enchantment(list_id, bonuses, names, effects)
        if row:
            enchantments[list_id] = row
    nodes = defaultdict(list)
    for node in tables["ItemBonusTreeNode"]:
        nodes[int(node["ParentItemBonusTreeID"])].append(node)
    trees = defaultdict(list)
    for row in tables["ItemXBonusTree"]:
        trees[int(row["ItemID"])].append(int(row["ItemBonusTreeID"]))
    points = {int(r["ID"]): r for r in tables["RandPropPoints"]}
    sparse = {int(r["ID"]): r for r in tables["ItemSparse"]}

    out, report = [], {"no_item_row": 0, "no_points": [], "disagree": []}
    for item_id in sorted(trees):
        pool = sorted({l for tree in trees[item_id] for l in tree_lists(tree, nodes) if l in enchantments})
        if not pool:
            continue
        row = items.get(item_id)
        if row is None:
            report["no_item_row"] += 1
            continue
        scaling = row.get("scaling_options", {}).get("0", {})
        stated = scaling.get("rand_prop_points", 0)
        cell = 0
        client = sparse.get(item_id)
        if client:
            quality = QUALITIES.get(int(client["OverallQualityID"]))
            column = INVENTORY_COLUMN.get(int(client["InventoryType"]))
        else:
            quality = QUALITIES.get((qualities or {}).get(str(item_id)))
            column = row_column(row)
        level = points.get(scaling.get("ilvl", 0))
        if quality and column is not None and level:
            cell = int(level[f"{quality}_{column}"])
        if stated and cell and stated != cell:
            report["disagree"].append((item_id, row["name"], stated, cell))
        budget = stated or cell
        if not budget:
            report["no_points"].append((item_id, row["name"]))
            continue
        out.append({"id": item_id, "points": budget, "enchantments": pool})
    used = {l for row in out for l in row["enchantments"]}
    return [enchantments[l] for l in sorted(used)], out, report


def dump(rows):
    return "".join(json.dumps(row, separators=(",", ":"), ensure_ascii=False) + "\n" for row in rows)


def fetch(directory, build):
    directory.mkdir(parents=True, exist_ok=True)
    if not build:
        with urllib.request.urlopen("https://wago.tools/api/builds") as response:
            build = json.load(response)["wowf"][0]["version"]
    for table in TABLES:
        url = f"https://wago.tools/db2/{table}/csv?build={build}"
        with urllib.request.urlopen(url) as response:
            (directory / f"{table}.csv").write_bytes(response.read())
    (directory / "build.txt").write_text(build + "\n")
    # The quality of the items whose row states no points and ItemSparse does not serve, from
    # Wowhead's Forever tooltips.
    tables = {table: read(directory, table) for table in TABLES}
    _, _, report = generate(tables, item_rows())
    qualities = {}
    for item_id, _name in report["no_points"]:
        request = urllib.request.Request(
            f"https://nether.wowhead.com/forever/tooltip/item/{item_id}?x={item_id}",
            headers={"User-Agent": "Mozilla/5.0"},
        )
        try:
            with urllib.request.urlopen(request) as response:
                qualities[str(item_id)] = json.load(response).get("quality")
        except urllib.error.HTTPError:
            continue
        time.sleep(0.2)
    (directory / QUALITY_FILE).write_text(json.dumps(qualities, indent=0, sort_keys=True) + "\n")
    print(f"fetched {len(TABLES)} tables of client {build} and {len(qualities)} item qualities into {directory}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["fetch", "build", "check"])
    parser.add_argument("directory", type=Path, help="where the client tables are kept")
    parser.add_argument("--build", help="client build to fetch (default: the newest wowf build)")
    args = parser.parse_args()
    if args.command == "fetch":
        fetch(args.directory, args.build)
        return
    try:
        tables = {table: read(args.directory, table) for table in TABLES}
        quality_path = args.directory / QUALITY_FILE
        qualities = json.loads(quality_path.read_text()) if quality_path.exists() else {}
        enchantments, items, report = generate(tables, item_rows(), qualities)
    except Problem as problem:
        sys.exit(str(problem))
    if report["disagree"]:
        sys.exit(f"item rows and RandPropPoints disagree on the points of: {report['disagree'][:10]}")
    texts = {ENCHANTMENTS: dump(enchantments), ITEMS: dump(items)}
    if args.command == "check":
        stale = [path.name for path, text in texts.items() if not path.exists() or path.read_text() != text]
        if stale:
            sys.exit(f"stale: {', '.join(stale)}; run build")
        print("random enchantment tables are current")
        return
    for path, text in texts.items():
        path.write_text(text)
    print(
        f"wrote {len(enchantments)} enchantments and {len(items)} items; "
        f"{report['no_item_row']} client items have no item row, "
        f"{len(report['no_points'])} have no points: {report['no_points'][:10]}"
    )


if __name__ == "__main__":
    main()
