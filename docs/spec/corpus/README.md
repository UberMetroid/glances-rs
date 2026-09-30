# Capture corpus

Black-box baselines captured from the v0.10.72 oracle binary (2026-09-25).
`replay.sh` re-captures these endpoints from a running server and compares
**JSON shape** (the recursive key set) rather than values, since live readings,
PIDs, and timestamps change every run. Shape equality is the wire contract.

```
./replay.sh http://localhost:61208     # gate: nonzero exit on any drift
./replay.sh --record http://host:61208 docs/spec/corpus/smoke   # refresh
```

`replay.sh` (repo root) is the gate. `capture.sh` (this directory) only fetches
raw payloads; the comparison lives in the script.

## Known weak baseline

`alert.json` was captured as `[]` — no alerts were firing at capture time — so
it carries no element shape. `replay.sh` reports it as **skipped**, not passed.
Any future drift in the alert payload's fields will go undetected until the
baseline is re-recorded during a period when alerts are active.

