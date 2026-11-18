"""Write the class spells a levelling character's spellbook export is the word on.

The MythicSim addon exports a levelling character's spellbook (`knownSpells`). The engine trusts
it for every rank of a spell the class trainer teaches: a trained spell is in the book, so one
the book lacks is not trained, whatever the spell's level row says. Shadowfiend's row (401977)
reads level 1, Shank's rank 2 (1329549) has no row in the store, and both were cast by
characters that had not trained them (18 Nov 2026, docs/levels.md).

A family is the ranks of one spell: the client's SkillLineAbility rows on a class skill line
(SkillLine category 7) with acquire method 0 (taught by the trainer), grouped by skill line and
spell name. The ranks after a talent's first (Holy Shock, Holy Shield) carry no class mask, so
the mask is not read; a family the addon has never listed stays out instead. Only families the addon exports are written: one a real export has
listed a rank of (OBSERVED), or one listed in UNLISTED below. Every other trainer row is a
passive, a rune or a pet's spell the spellbook does not list, and the level decides it as before.
A spell a talent grants (the Talent table or the trait tree) is never written: the talent string
decides it.

data/forever-trainer-spells.jsonl, one rank per row, keyed by spell id:

    {"id":1329283,"family":1329283,"level":24}

  "family" is the family's first rank (lowest level, then lowest id), "level" the rank's
  SpellLevels level (BaseLevel where SpellLevel is 0), which orders the ranks.

The tables come from wago.tools, product wowf:

    python3 tools/forever_trainer_spells.py fetch DIR [--build 1.60.1.70959]
    python3 tools/forever_trainer_spells.py build DIR
    python3 tools/forever_trainer_spells.py check DIR
"""

import argparse
import csv
import json
import sys
import urllib.request
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "forever-trainer-spells.jsonl"
# Trainer rows that real addon exports have listed (1,384 levelling characters' spellbooks from
# MythicSim's prod Quick Sim and Best Gear runs, 12 to 18 Nov 2026, client 70959). Spell ids only.
OBSERVED = Path(__file__).resolve().parent / "forever_trainer_spells_observed.json"
TABLES = ["SkillLine", "SkillLineAbility", "SpellName", "SpellLevels", "Talent", "TraitDefinition"]
CLASS_SKILL_CATEGORY = "7"
ACQUIRE_TRAINER = "0"
# Trainer families no export lists that the engine registers for a levelling character, each
# with the evidence that the spellbook would list it once trained.
UNLISTED = {
    # Shadowfiend (Shadow, priest mask, acquire method 0): none of 38 levelling priests (levels
    # 13 to 59) lists it, a level 30 Discipline Priest reported it is not trained or talented
    # at that level (18 Nov 2026), and the engine's presets cast it at every level.
    401977: "Shadowfiend",
}


def read(directory, table):
    with open(directory / f"{table}.csv", newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def talent_spells(tables):
    """Every spell a talent grants: the Talent table's ranks and the trait tree's definitions.

    A talent is the talent string's word, not the spellbook's: Swift Judgement (1310994) sits on
    a trainer row and in the trait tree, and a level 35 paladin's string takes it while its
    spellbook does not list it.
    """
    spells = set()
    for row in tables["Talent"]:
        spells.update(int(row[f"SpellRank_{rank}"] or 0) for rank in range(9))
    for row in tables["TraitDefinition"]:
        spells.update(int(row[field] or 0) for field in ("SpellID", "VisibleSpellID", "OverridesSpellID"))
    spells.discard(0)
    return spells


def generate(tables, observed, unlisted=UNLISTED):
    """The rows of data/forever-trainer-spells.jsonl, sorted by id."""
    class_lines = {
        row["ID"] for row in tables["SkillLine"] if row["CategoryID"] == CLASS_SKILL_CATEGORY
    }
    names = {int(row["ID"]): row["Name_lang"] for row in tables["SpellName"]}
    levels = {}
    for row in tables["SpellLevels"]:
        if row["DifficultyID"] == "0":
            levels[int(row["SpellID"])] = int(row["SpellLevel"] or 0) or int(row["BaseLevel"] or 0)
    talents = talent_spells(tables)
    families = defaultdict(set)
    for row in tables["SkillLineAbility"]:
        if int(row["Spell"]) in talents:
            continue
        if row["SkillLine"] in class_lines and row["AcquireMethod"] == ACQUIRE_TRAINER:
            spell = int(row["Spell"])
            families[(row["SkillLine"], names.get(spell, ""))].add(spell)
    observed = set(observed)
    rows = {}
    for spells in families.values():
        if not (spells & observed or spells & set(unlisted)):
            continue
        ranks = sorted(spells, key=lambda spell: (levels.get(spell, 0), spell))
        for spell in ranks:
            # A spell on two class lines (a shared spell id) keeps its first family.
            rows.setdefault(spell, {"id": spell, "family": ranks[0], "level": levels.get(spell, 0)})
    missing = sorted(spell for spell in unlisted if spell not in rows)
    if missing:
        raise ValueError(f"UNLISTED spells with no trainer row: {missing}")
    return [rows[spell] for spell in sorted(rows)]


def render(rows):
    return "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows)


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


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["fetch", "build", "check"])
    parser.add_argument("directory", type=Path, help="where the client tables are kept")
    parser.add_argument("--build", help="client build to fetch (default: the newest wowf build)")
    args = parser.parse_args()
    if args.command == "fetch":
        fetch(args.directory, args.build)
        return
    tables = {table: read(args.directory, table) for table in TABLES}
    text = render(generate(tables, json.loads(OBSERVED.read_text())))
    if args.command == "build":
        OUT.write_text(text)
        print(f"wrote {OUT.relative_to(ROOT)} ({text.count(chr(10))} ranks)")
    elif OUT.read_text() != text:
        sys.exit(f"{OUT.relative_to(ROOT)} is out of date: run build")


if __name__ == "__main__":
    main()
