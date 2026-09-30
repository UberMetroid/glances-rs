#!/bin/sh
# replay.sh — prove the frozen wire contract against a captured baseline.
#
# The corpus in docs/spec/corpus/smoke/ was captured from the v0.10.72 oracle
# binary. This script re-captures the same endpoints from a running server and
# compares them to the baseline. Live readings, PIDs, and timestamps differ on
# every run, so the comparison is on JSON *shape* — the recursive key set of
# each payload — not on values. Shape equality is the contract: a renamed,
# dropped, or added key is a breaking change to every consumer (dashboard,
# homepage widget, install tooling).
#
# Usage:
#   ./replay.sh [base-url]      # default http://localhost:61208
#   ./replay.sh --record URL DIR   # refresh the baseline in place
#
# Exits nonzero if any endpoint's shape drifted. An endpoint whose baseline is
# an empty array carries no shape to compare and is reported as skipped, not
# silently passed.

set -eu

ROOT=$(dirname "$0")
CORPUS="$ROOT/docs/spec/corpus/smoke"
DEFAULT_BASE="http://localhost:61208"

RECORD=0
if [ "${1:-}" = "--record" ]; then
    RECORD=1
    BASE="${2:?--record needs a base-url}"
    OUT="${3:?--record needs an output dir}"
    sh "$ROOT/docs/spec/capture.sh" "$BASE" "$OUT"
    echo "recorded baseline into $OUT"
    exit 0
fi

BASE="${1:-$DEFAULT_BASE}"
command -v python3 >/dev/null 2>&1 || { echo "replay.sh needs python3" >&2; exit 2; }
[ -d "$CORPUS" ] || { echo "no corpus at $CORPUS" >&2; exit 2; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

for f in "$CORPUS"/*.json; do
    ep=$(basename "$f" .json)
    curl -fsSL "$BASE/api/4/$ep" -o "$TMP/$ep.json" \
        || { echo "FAIL $ep — endpoint unreachable at $BASE"; exit 1; }
done

python3 - "$CORPUS" "$TMP" <<'PY'
import json, os, sys

corpus, cur = sys.argv[1], sys.argv[2]

def keyset(o, pre=""):
    """Recursive dotted key set. Array elements are probed with one sample."""
    out = set()
    if isinstance(o, dict):
        for k, v in o.items():
            out.add(pre + k)
            out |= keyset(v, pre + k + ".")
    elif isinstance(o, list) and o:
        out |= keyset(o[0], pre + "[].")
    return out

def sampleable(o):
    """True if the payload exposes any structure to compare."""
    if isinstance(o, dict):
        return bool(o)
    if isinstance(o, list):
        return bool(o)
    return False

ok = drift = skipped = 0
for name in sorted(os.listdir(corpus)):
    if not name.endswith(".json"):
        continue
    ep = name[:-5]
    base = json.load(open(os.path.join(corpus, name)))
    live = json.load(open(os.path.join(cur, name)))

    if not sampleable(base):
        print(f"skip {ep} — baseline is an empty {type(base).__name__}, no shape to compare")
        skipped += 1
        continue

    kb, kl = keyset(base), keyset(live)
    if kb == kl:
        print(f"ok   {ep} — {len(kb)} keys, shape identical")
        ok += 1
    else:
        only_b, only_l = sorted(kb - kl), sorted(kl - kb)
        print(f"DRIFT {ep}")
        if only_b:
            print(f"       dropped since baseline: {only_b}")
        if only_l:
            print(f"       added since baseline:   {only_l}")
        drift += 1

print()
print(f"{ok} endpoints identical, {drift} drifted, {skipped} skipped")
sys.exit(1 if drift else 0)
PY

rc=$?
[ "$rc" -eq 0 ] && echo "wire contract holds against $(basename "$CORPUS")" || echo "WIRE CONTRACT DRIFT — see above"
exit "$rc"
