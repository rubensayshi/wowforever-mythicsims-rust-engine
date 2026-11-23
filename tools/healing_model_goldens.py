#!/usr/bin/env python3
"""Write the healing model goldens: requests with a healingModel and the pinned Go engine's
results for them, which tests/healing_model.rs prepares with the Go reference's rules and
compares.

The Go exporter still marks a healing model as unrepresented, so these cases are not in the
prepared v2 family: Rust prepares each request itself. Each request is an accepted fixture's
with a model added and fewer iterations.

    python3 tools/healing_model_goldens.py --oracle oracle-cache/forever-go-oracle-v2

Build the oracle first, for example with `python3 tools/prepared_v2.py compare`, which leaves
it in oracle-cache. The results keep tools/prepared_v2.py's comparable view; the first case
keeps the first fight's log, filtered as the family's logs are.
"""

import argparse
import copy
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAMILY = ROOT / "fixtures" / "mage" / "prepared-v2"
OUT = ROOT / "fixtures" / "healing-model"
sys.path.insert(0, str(ROOT / "tools"))
import prepared_v2  # noqa: E402

# (case, base fixture, healingModel, whether the case keeps its log)
CASES = [
    ("tank-heals", "production-protection-warrior",
     {"hps": 700, "cadenceSeconds": 2, "cadenceVariation": 1, "absorbFrac": 0.2, "burstWindow": 6}, True),
    ("tank-presim", "production-protection-warrior", {"cadenceSeconds": 2, "burstWindow": 6}, False),
    ("tank-wide-cadence-full-shield", "production-protection-warrior",
     {"hps": 1000, "cadenceSeconds": 1.5, "cadenceVariation": 3, "absorbFrac": 1, "burstWindow": 5}, False),
    ("tank-default-cadence", "production-protection-warrior", {"hps": 900, "absorbFrac": 0.5}, False),
    ("not-tanking", "production-arms-warrior",
     {"hps": 700, "cadenceSeconds": 2, "absorbFrac": 0.3, "burstWindow": 6}, False),
    ("not-tanking-presim", "production-arms-warrior", {"cadenceSeconds": 2}, False),
]
ITERATIONS = 200


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--oracle", type=Path, required=True, help="the pinned forever-go-oracle-v2 binary")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    cases = []
    for case, base, model, keep_log in CASES:
        request = json.loads((FAMILY / f"{base}.request.json").read_text())
        request = copy.deepcopy(request)
        request["simOptions"]["iterations"] = ITERATIONS
        request["simOptions"]["debugFirstIteration"] = keep_log
        request["raid"]["parties"][0]["players"][0]["healingModel"] = model
        request_path = OUT / f"{case}.request.json"
        request_path.write_text(json.dumps(request, indent=2) + "\n")
        with tempfile.TemporaryDirectory() as scratch:
            result, log = prepared_v2.go_golden(args.oracle, request_path, Path(scratch))
        (OUT / f"{case}.go-result.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        entry = {"id": case, "base": base, "healing_model": model}
        if keep_log:
            (OUT / f"{case}.go-log.txt").write_text(log)
            entry["go_log"] = f"{case}.go-log.txt"
        cases.append(entry)
        print(f"{case}: Go dps {result['raidMetrics']['dps'].get('avg')}", flush=True)
    manifest = {
        "reference_revision": prepared_v2.PIN,
        "client_build": prepared_v2.CLIENT_BUILD,
        "iterations": ITERATIONS,
        "generator": "python3 tools/healing_model_goldens.py --oracle oracle-cache/forever-go-oracle-v2",
        "cases": cases,
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
