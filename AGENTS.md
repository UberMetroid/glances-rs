# glances-rs — Engineering Rules

Rules for humans and agents working in this repository. They are derived from
what this codebase actually is: a pure-std Linux system monitor whose defining
property is that every shipped line is provably ours.

**Precedence.** Where this file and CI disagree, this file wins — but file a
follow-up to fix the CI in the same change. A rule that is not enforced anywhere
is a bug in this file, not a suggestion to ignore.

**Reading the markers.** Every rule below is tagged:

- `[GATE]` — a named test or CI step fails the build if broken. Cite the test.
- `[POLICY]` — not machine-enforced. You are expected to follow it; a human
  reads it. This is where originality, naming taste, and restraint live.

If you add a gate, upgrade the marker. If you remove one, delete the rule rather
than leaving a marker that lies.

---

## 1. Zero dependencies (AC-1)

`[GATE]` `Cargo.toml` has no `[dependencies]` section — `qa::lint::no_crates::cargo_toml_has_no_dependencies`.
`[GATE]` No `extern crate` in `src/` — `qa::lint::no_crates::no_extern_crate_in_src`.

**Why:** the zero-crate tree is the reason this project can be audited, vendored,
and built anywhere with nothing but a toolchain. A single dependency is a
licensing and supply-chain decision that must be made deliberately at the org
level, not absorbed by a feature. This also means `thiserror`, `tracing`, and
`tokio` are **not** available — we hand-write the error impls
(`src/core/error.rs`) and log through `src/core/logger.rs`.

**If you think you need a crate:** stop. The answer is almost always a few
hundred lines of `std`, which is also cheaper to read than a dependency's docs.

## 2. The page rule

`[GATE]` Every `.rs` file is **16–256 lines** inclusive, comments and blank lines
counted — `qa::lint::line_cap::every_rs_file_within_16_to_256_lines`. CI adds a
second 256-line-only guard in `.github/workflows/ci.yml`.
`[GATE]` `mod.rs` / `lib.rs` are exempt from the floor only.

**Why the floor:** a file under 16 lines is usually a whole function that belongs
inside its caller. **Why the cap:** a file over 256 is doing two or more things,
and the reader pays for it. The bounds are entropy limits, not style taste.

**Shims.** A `mod.rs` that only declares modules and re-exports is wiring, not
thought, and is exempt from the floor. If a shim starts carrying logic, it is no
longer a shim — split the logic out. The lint decides this by **content, not
filename**: after dropping blanks, comments, and attributes, every remaining
line must be a `mod` / `use` / `pub` declaration. Six files rely on the exemption
today, and all six are genuine shims.

**Never pad.** Adding blank lines or comments to clear the floor fails review.
The correct fix is to fold the file into its closest functionally-aligned
sibling.

**The page proves it.** If a page cannot state its own job in one line, it is two
pages wearing a trench coat.

## 3. Naming

`[GATE]` Name the function, not the drawer. These names are banned:
`util.rs`, `utils.rs`, `helper.rs`, `helpers.rs`, `common.rs`, `misc.rs`,
`shared.rs`, `base.rs`, `core.rs` — `qa::lint::naming::no_drawer_named_rs_files`.

**Why:** a file named `utils` is an admission that nobody could describe its job.
The tree should read as a map of the running system — `core/`, `plugins/`,
`platform/linux/`, `outputs/web/` — with paths that name subsystems and files
that name the one thing they own (`lease_table`, `verify_peer`, `seal_weights`).

Note the ban is on *filenames*: a `src/core/` **directory** is fine, a
`src/core.rs` **file** is not.

`[POLICY]` Test files name their subject: `plugins_cpu.rs` tests the cpu plugin.
`_tests.rs` siblings are for tests that would otherwise push a page over the cap.

## 4. Ownership and provenance

This is the project's reason for existing. The tree is LGPL-3.0-only, and that
license stays; we do not relicense. What changes is that no shipped line is
*inherited*.

`[GATE]` Every `.rs` file under `src/` appears **exactly once** in
`docs/ownership.md` with status `TAINTED` or `OWNED` —
`qa::lint::ownership::ledger_covers_every_rs_file_exactly_once`.

`[POLICY]` The zero-trust air-gap, applied per module, no exceptions:

1. **Observe** — capture real inputs/outputs from a running baseline binary.
2. **Specify** — write `docs/spec/<module>.md` from ground truth.
3. **Quarantine** — record the old file's hash in the ledger, archive it outside
   the tree, delete the working file.
4. **Implement** — from the spec and OWNED files only. Start from blank.
5. **Prove** — existing suite green, a new independent-oracle test green, black-box
   A/B diff against the captured baseline.
6. **Review** — no inherited identifiers, comments, or error strings.
7. **Flip** — ledger entry `TAINTED` → `OWNED` with spec link and commit hash.

`[POLICY]` Ground truth is the live OS (`/proc`, `/sys`, observed syscalls),
installed man pages, and our own fresh decisions. Upstream source is never opened
during implementation — the air-gap works in both directions. If a claim is not
grounded in observed behavior, treat it as untested and say so.

`[POLICY]` Independence of oracle: expected values in a new test are derived from
ground truth inside the test (raw `/proc` reads, captured vectors), never copied
from the old assertions. Old tests are contracts to satisfy, not text to
reproduce.

**The rewrite is complete — 66/66 flipped.** Do not re-open settled provenance
without new evidence. The marker sweep that closed it is recorded in the ledger's
flip log.

## 5. The frozen wire contract

`[POLICY]` Endpoint paths, JSON keys, CLI flags, exit codes, and default numeric
thresholds are **frozen**. The dashboard, the homepage widget, and `install.sh`
all depend on them; changing one is a breaking change needing its own migration
note, not a drive-by fix.

Interface constants are not code reuse — re-creating a path identically is
required. Everything behind them (types, logic, names, comments, error strings)
is ours and must be original.

## 6. Crash safety and process hygiene

`[GATE]` No shell invocation anywhere in `src/` —
`qa::lint::no_shell::no_sh_invocation_in_src`. Subprocesses are spawned argv-only
(`src/core/actions/run.rs`, `src/plugins/gpu_nvidia.rs`, `src/plugins/smart/`,
`src/plugins/vms/`); never add `sh -c` / `bash -c` / `cmd.exe /C`.

`[GATE]` No `unsafe` outside `src/platform/linux/` —
`qa::lint::unsafe_allowlist::no_unsafe_outside_allowlist`. Today that is exactly
three files: `statvfs.rs`, `uname.rs`, `sysconf.rs`, nine `unsafe` blocks
between them, each carrying a `// safe:` note.

`[GATE]` **A panicking plugin must not kill the refresh loop** —
`qa::edge::plugin_panic_isolation::panic_in_one_plugin_does_not_kill_loop`.

`[POLICY]` That last test is the real safety property, and it is the one to
preserve. It means `Plugin::update` is treated as fallible: return
`Result<(), Error>`, and keep genuinely infallible arithmetic off the error path
rather than inventing an error for it. Many `unwrap()` calls exist in this tree —
including inside in-file `#[cfg(test)]` modules — and that is acceptable here
only because the loop is panic-isolated. **New fallible production code returns
`Result`**; do not add new `unwrap()` to a path that can fail at runtime.

`[POLICY]` `unsafe` blocks carry a `// safe:` note stating the invariant that
makes them sound — pointer validity, lifetime, and the callee's error contract.
The lint honors the marker as a file-level escape for anything outside the
allowlist, so an unexplained `unsafe` fails the build. Write the note against
the actual call; a boilerplate note satisfies the text and defeats the point.

**Current production surface:** exactly two non-test `unwrap`/`expect`/`panic!`
sites in the whole tree, both sound — `src/platform/mod.rs` panics deliberately
as a compile-time non-Linux guard, and `src/core/actions/run.rs` pops from a
vector proven non-empty by the `is_empty()` guard above it. Everything else
lives in `#[cfg(test)]`. Keep it that way.

## 7. Verification

Order matters. Each stage must be green before the next.

1. `cargo test` — full suite, lints included (they are **not** `#[ignore]`d and
   do run by default).
2. `cargo clippy --all-targets` — zero errors *and* zero warnings.
3. `./crosscheck.sh http://localhost:61208` — live 20-check cross-check against
   the running service, comparing `/api/4` values against OS tools that share no
   code with the daemon (`df`, `/proc`, `iproute2`, `ss`, hwmon, `nvidia-smi`).
4. `./replay.sh http://localhost:61208` — replays the v0.10.72 capture corpus
   against a live server and compares JSON **shape**, not values. This is the
   frozen-wire-contract gate: a renamed, dropped, or added key fails it.
5. `./release.sh X.Y.Z --service` — bumps the version across its five pinned
   files (`Cargo.toml`, `Cargo.lock`, `README.md`, `docs/api.md`,
   `assets/static/openapi.json`), re-runs gates **after** the bump, commits,
   pushes, tags, publishes the release with SHA256SUMS, then cuts the service
   over and runs `verify-deploy.sh`.

`[POLICY]` Never commit with a failing suite. Never bump a version by hand —
`release.sh` owns the pins and aborts if they desync. Runtime version strings come
from `env!("CARGO_PKG_VERSION")`, so they follow `Cargo.toml` automatically; the
other four files are documentation and API contract surfaces that a human must
keep in step.

`[POLICY]` If a change measurably grows a page's shipped binary size or its CPU
share under QA load, the commit message must say why. A regression without a
written justification is a defect, not a trade-off.

`[POLICY]` The installed binary (`~/.local/bin/glances-rs`) and this working tree
are deliberately decoupled — a plain copy, not a symlink. Building here does not
touch the running service. Deployment is a separate, explicit step.

## 8. Scope

This tree is **Linux-only** and std-only. `src/platform/` contains exactly one
subdirectory, `linux/`, and every unsafe call in the tree lives there. There is
no macOS or Windows code, and the unsafe allowlist lists no slot for any — if a
port ever lands, add its directory to the allowlist deliberately, in the same
change that introduces the code. Don't add ports, and don't preserve dead
non-Linux branches.

The page rule covers `.rs` only. Shell scripts are governed by their own
structure, not by a line count borrowed from Rust.

## 9. Known gaps — not enforced

Stated plainly so nobody mistakes policy for a gate:

- **Directory density** is unbounded. `src/qa/unit/` holds 67 files, `src/plugins`
  41. Splitting crowded directories by functional area is desirable but not
  required, and is not a gate. Everything else in this document is enforced.
- **No `unwrap()` budget.** The ratchet is "no new fallible unwraps", not a count.
  See §6 for the current surface.

Adding a gate means writing a test in `src/qa/lint/` — and registering the new
file in `docs/ownership.md`, or the ownership lint will fail. Prefer a test over
prose here, because prose cannot fail a build.
