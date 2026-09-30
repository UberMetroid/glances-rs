## Goal
Every line of shipped code in glances-rs is original to this project: the 65 upstream-ported files (of 213 total, ~10.7k lines) are rewritten from first principles under a zero-trust protocol, with provenance proven by process and verification rather than asserted. No license change; the tree stays LGPL-3.0-only.

## Success Criteria
- All 65 ledger files flipped from TAINTED to OWNED; ledger-accuracy lint green.
- Every rewritten module passed the air-gap protocol (observe → specify → quarantine → implement → prove → review) with a log entry.
- Full suite green (existing oracle tests plus new independent-oracle tests), clippy clean, 256-line cap holds.
- Black-box A/B against the v0.10.72 baseline is clean on the frozen wire contract.
- No `parity`/`mirrors`/upstream markers anywhere outside the historical notes in the ownership ledger.

## Context And Current Facts
- Tainted groups: CLI surface 5 files, core primitives 11, core services 7, outputs 7, plugins 28, parity-marked tests 7. The other 148 files are original in-project expression and stay as-is.
- Gates that must hold at every commit: 664-test suite, `cargo clippy --all-targets`, 256-line lint, openapi version-tracking test.
- The published v0.10.72 release asset (`glances-rs-linux-x86_64.tar.gz` + `SHA256SUMS`) is the frozen behavioral oracle; the live service runs it now.
- Standing rule in force: commit + push + version bump per landed unit; full release loop (tag + GH release + service) at completion.

## Constraints And Non-goals
- Non-goals: license change, API breaks, new features, performance work, non-Linux ports.
- Pure std / zero dependencies retained. The live service stays on released code until the final cutover.
- In-place rewrite on the `rust` branch (see decision 1), so history and the oracle suite stay usable throughout.

## Key Decisions
1. **In-place, not fresh repo.** With no relicense goal, history is an asset: the oracle suite stays runnable at every step and CI/releases/service continue uninterrupted. Taint is tracked per file in the ledger, not by repo boundary. Rejected: fresh repo (destroys the step-by-step oracle for no gain once provenance-for-relicense is off the table).
2. **Wire contract frozen as functional constants.** Endpoint paths, JSON keys, CLI flags, exit codes, and default numeric thresholds are re-created identically and documented as interoperability requirements (dashboard, homepage widget, install tooling depend on them). Interface constants are not code reuse. All implementation — types, logic, names, comments, error strings — is original. Rejected: full API redesign (breaks consumers, buys no ownership).
3. **Old source is readable only during behavior enumeration**, transcribed into specs as behavior statements, then quarantined (archived by hash, deleted from the tree) before implementation starts. Upstream source is never opened, even for comparison — the quarantine works both ways.
4. **New code may depend only on OWNED-ledger files**, enforced by the per-module review checklist; bottom-up ordering makes this natural.
5. **Independent-oracle test required per rewritten module**: expected values derived from ground truth inside the test (raw `/proc` snapshots, captured vectors), never copied assertions. Old tests are oracles to satisfy, not expressions to imitate.
6. **Completion is ledger-driven, never red-test-driven.** The ledger-accuracy lint (every file listed, statuses valid) stays green throughout; a human reads 65/65 OWNED as done. No commit with a failing suite, ever.
7. **Release cadence**: commit + push + patch bump per batch; GH binary release + service cutover once at completion (per-batch binaries waste effort; source is always available).
8. **File organization: functional alignment first, 16–256 lines enforced.** One concept per file; split multi-concept files freely (target 20–150, backstop 256). Both bounds are hard and lint-enforced: the existing line-cap lint gains a 16-line floor. Only `mod.rs`/`lib.rs` module-wiring shims are exempt (6 exist today, all shims); everything else below 16 folds into its closest aligned sibling. Padding to hit the floor fails the step-6 review.

## Recommended Approach
Zero-trust first-principles rewrite: trust nothing inherited, derive everything from ground truth, prove every module twice (suite + independent oracle).

**Trust tiers.** T0 ground truth (trusted): live OS interfaces (`/proc`, `/sys`, syscalls as observed), installed manual pages, HTTP behavior as observed, captured input/output vectors from the v0.10.72 binary, our own fresh design decisions. T1 specs (derived only from T0): `docs/spec/<module>.md`. T2 code (derived only from T1 plus OWNED files). Banned at all times: upstream source, ported code during implementation, ported identifiers/comments/error strings beyond the frozen wire constants.

**Per-module air-gap (every one of the 65, no exceptions).**
1. Observe: run the v0.10.72 binary, capture the module's inputs and outputs as vectors/fixtures. Old source may be skimmed only to inventory *what* to observe.
2. Specify: write `docs/spec/<module>.md` from T0 — OS facts read, computation defined fresh, outputs and edge cases.
3. Quarantine: record the old file's hash in the ledger, archive it outside the tree, delete the working file. Implementation starts from a blank file.
4. Implement: spec only, OWNED dependencies only, house rules apply (pure std, ≤256 lines, clippy clean, decision 8 file sizing).
5. Prove: existing suite green, new independent-oracle test green, black-box A/B diff against the v0.10.72 capture clean (for observable modules).
6. Review: checklist — no old identifiers beyond wire constants, no old comments/strings, no new `parity` markers, ledger deps all OWNED, one concept per file, no padding to meet the floor.
7. Flip: ledger entry TAINTED → OWNED with spec link and commit hash.

**First-principles grounding per subsystem.** CPU/memory/process/disk/network/fs: re-derive field semantics from live `/proc` observation plus installed manuals; define sampling math fresh in the spec. Thresholds/alerts/events/views: our own band logic and log design against the frozen numeric defaults. Password/PBKDF2/auth: respecify from captured input/output vectors (round-trip behavior is the contract). SNMP client: respecify from captured request/response vectors against a local target. Web router/outputs/CLI: route table and formats from the frozen contract, control flow redesigned.

## Work Plan
Commit split (binding at publish): 9 commits, each independently green (full suite + clippy), pushed with a patch bump.
- **P0 — Ledger + baseline (1 commit).** `docs/ownership.md` (65 TAINTED / 148 OWNED) + ledger-accuracy lint (`src/qa/lint/ownership.rs`) + 16-line floor added to the line-cap lint (mod.rs/lib.rs exempt) + spec template + v0.10.72 capture corpus checked in as fixtures.
- **P1 — Core (2 commits).** Batch 1: `plugin`, `stats`, `threshold`, `timer`, `history`, `events`, `alerts`, `alert_views`, `config_dir`, `filter/*`. Batch 2: `actions/*`, `password/*`, `pbkdf2`, `snmp/*`. Rewrite the 3 core `qa/unit` test files from the new specs.
- **P2 — Outputs + CLI (2 commits).** Batch 1: `outputs/*`, `web/*`. Batch 2: `cli/*`, `main.rs`. Rewrite the 2 CLI `qa/unit` test files.
- **P3 — Plugins (3 commits).** Batch A (simple): `mod` (registry), `version`, `uptime`, `help`, `load`, `mem`, `cpu`, `percpu`, `quicklook`, `processcount`. Batch B (IO): `diskio`, `network`, `fs`, `connections`, `ports`, `irq`, `programlist`, `alert`. Batch C (complex): `processlist/*`, `containers`, `vms/*`, `smart/*`, `gpu_format`, `ip/*`, `amps`, `cloud`. Rewrite the plugin `qa/unit` test files with their batches.
- **P4 — Final audit + release (1 commit).** Ledger reads 65/65 OWNED; marker grep clean; full hygiene; version bump; tag + GH release + service cutover + live verification per the standing rule.

## Validation Plan
- Per module: focused `cargo test <module>`, new independent-oracle test, A/B replay of its capture corpus (`diff` old-asset output vs new-binary output, modulo timestamps/PIDs).
- Per commit: full `cargo test`, `cargo clippy --all-targets`, ledger-accuracy lint green.
- Final: full corpus replay; live CPU-spike A/B (old asset on one port, new binary on another, diff `/api/4/alert`, `/api/4/health`, `/api/4/cpu`); ownership audit (`docs/ownership.md` 65/65, marker grep empty outside the ledger).
- Highest-risk validation: the live-spike A/B on `processlist` and `alerts` — the most underspecified edge behavior lives there.
- Manual check that cannot be automated: the per-module review checklist (identifier/string originality) at step 6 of the air-gap.

## Risks / Rollback
- **Effort**: ~10.7k lines plus specs and oracle tests; weeks, not days. Mitigation: batch commits keep every step shippable; work can pause after any green commit.
- **Subtle behavioral drift**: mitigated by the phase-0 capture corpus and per-module A/B replays.
- **Accidental reuse from memory**: mitigated by quarantine (blank-file start), the dependency rule, and the review checklist.
- **Rollback**: none needed structurally — each batch commit is independently green and revertible on its own; the service stays on the last full release until P4.

9. **Internal API surfaces stay signature-stable.** Trait methods, enum variants, and cross-module function signatures of rewritten modules keep their shapes (they are interface facts, like the wire contract); every body, doc, private helper, and module organization is written fresh. Rationale: changing signatures would force same-commit rewrites of other batches and break the plan's commit structure. The no-old-identifiers checklist applies to private items, comments, strings, and structure.

## Open Questions
- **Q1: Is the wire contract (paths, keys, flags, exit codes, default thresholds) frozen with zero exceptions?** Default: yes. Any exception would need its own migration note for the dashboard and homepage widget.
