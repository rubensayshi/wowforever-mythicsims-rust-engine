#!/usr/bin/env python3
"""Write the health fight goldens: requests with `encounter.useHealth` and the pinned Go engine's
results for them, which tests/health_fight.rs prepares with the Go reference's rules and
compares.

The Go exporter marks a health fight as unrepresented, so these cases are not in the prepared v2
family: Rust prepares each request itself. Each request is an accepted fixture's with the fight
ending on the targets' health (`stats[32]`), a 60 second duration and no variation, and fewer
iterations.

    python3 tools/health_fight_goldens.py --oracle oracle-cache/forever-go-oracle-v2

Build the oracle first, for example with `python3 tools/prepared_v2.py compare`, which leaves it
in oracle-cache. The results keep tools/prepared_v2.py's comparable view; the cases marked so keep
the first fight's log, filtered as the family's logs are.
"""

import argparse
import copy
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAMILY = ROOT / "fixtures" / "mage" / "prepared-v2"
OUT = ROOT / "fixtures" / "health-fight"
sys.path.insert(0, str(ROOT / "tools"))
import prepared_v2  # noqa: E402

HEALTH, ARMOR = 32, 30
ITERATIONS = 200
# A level 60 normal mob, as the leveling tools ask for it.
MOB = {"level": 60, "minBaseDamage": 111.9, "damageSpread": 0.2324, "swingSpeed": 2.0}

# (case, base fixture, target health, changes to the target, healingModel, keeps its log)
CASES = [
    ("protection-warrior-level-60-mob", "production-protection-warrior", 4120, (MOB, 3791), None, True),
    ("protection-warrior-level-60-mob-healer-presim", "production-protection-warrior", 4120, (MOB, 3791),
     {"cadenceSeconds": 2, "burstWindow": 6}, False),
    ("fury-warrior-execute", "fury-warrior-demoralizing-shout", 60000, None, None, True),
    ("protection-warrior-3-targets", "production-protection-warrior-3-targets", 30000, None, None, False),
    ("beast-mastery-hunter-wolf", "beast-mastery-hunter-wolf", 50000, None, None, False),
]


def request(base, health, target, model, keep_log):
    request = copy.deepcopy(json.loads((FAMILY / f"{base}.request.json").read_text()))
    encounter = request["encounter"]
    encounter["useHealth"] = True
    encounter["duration"] = 60
    encounter["durationVariation"] = 0
    for options in encounter["targets"]:
        stats = options["stats"] + [0] * (41 - len(options["stats"]))
        stats[HEALTH] = health
        if target is not None:
            changes, armor = target
            options.update(changes)
            stats[ARMOR] = armor
        options["stats"] = stats
    request["simOptions"]["iterations"] = ITERATIONS
    request["simOptions"]["debugFirstIteration"] = keep_log
    if model is not None:
        request["raid"]["parties"][0]["players"][0]["healingModel"] = model
    return request


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--oracle", type=Path, required=True, help="the pinned forever-go-oracle-v2 binary")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    cases = []
    for case, base, health, target, model, keep_log in CASES:
        request_path = OUT / f"{case}.request.json"
        request_path.write_text(json.dumps(request(base, health, target, model, keep_log), indent=2) + "\n")
        with tempfile.TemporaryDirectory() as scratch:
            result, log = prepared_v2.go_golden(args.oracle, request_path, Path(scratch))
        (OUT / f"{case}.go-result.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        entry = {"id": case, "base": base, "target_health": health}
        if keep_log:
            (OUT / f"{case}.go-log.txt").write_text(log)
            entry["go_log"] = f"{case}.go-log.txt"
        cases.append(entry)
        print(f"{case}: Go dps {result['raidMetrics']['dps'].get('avg')}, "
              f"{result.get('avgIterationDuration')} s a fight", flush=True)
    manifest = {
        "reference_revision": prepared_v2.PIN,
        "client_build": prepared_v2.CLIENT_BUILD,
        "iterations": ITERATIONS,
        "generator": "python3 tools/health_fight_goldens.py --oracle oracle-cache/forever-go-oracle-v2",
        "cases": cases,
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
