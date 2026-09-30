#!/bin/sh
# release.sh — one-command version bump + gates + tag + publish.
#
# Bakes in the release checklist so no step can be skipped (the P3B
# release missed the openapi pin and a ledger note — this script
# exists so that never happens again):
#   1. tree clean, on rust branch, all 5 version pins agree
#   2. bump pins, rebuild lockfile
#   3. full test suite + clippy gates (AFTER the bump)
#   4. commit + push, tag + push tag
#   5. portable release build -> tarball + SHA256SUMS -> gh release
#   6. optional: native service build + restart + verify-deploy
#
# Usage: ./release.sh X.Y.Z [--service] [--crosscheck] [--dry-run]
set -u

VER="${1:-}"
SERVICE=0; CROSSCHECK=0; DRY=0
for a in "$@"; do
    case "$a" in
        --service) SERVICE=1 ;;
        --crosscheck) CROSSCHECK=1 ;;
        --dry-run) DRY=1 ;;
    esac
done

case "$VER" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "usage: $0 X.Y.Z [--service] [--crosscheck] [--dry-run]" >&2; exit 1 ;;
esac

ROOT=$(dirname "$0")
cd "$ROOT" || exit 1

run() { # dry-run aware
    if [ "$DRY" = 1 ]; then echo "dry-run: $*"; else "$@"; fi
}

# ---- 1. preconditions (read-only, always run) ----
BRANCH=$(git branch --show-current)
[ "$BRANCH" = "rust" ] || { echo "must run on rust branch (on $BRANCH)" >&2; exit 1; }
DIRTY=$(git status --porcelain | grep -v '^??' || true)
[ -z "$DIRTY" ] || { echo "tree must be clean; found:" >&2; echo "$DIRTY" >&2; exit 1; }
CUR=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')
[ -n "$CUR" ] || { echo "cannot read current version" >&2; exit 1; }
echo "bump $CUR -> $VER"
for f in Cargo.toml Cargo.lock README.md docs/api.md assets/static/openapi.json; do
    grep -qF "$CUR" "$f" || { echo "pin desync: $f lacks $CUR" >&2; exit 1; }
done
echo "pins agree on $CUR"
[ "$DRY" = 1 ] && { echo "dry-run: would bump, gate, commit, tag v$VER, build, publish"; exit 0; }

# ---- 2. bump ----
sed -i "s/$CUR/$VER/g" Cargo.toml README.md docs/api.md assets/static/openapi.json
cargo build --quiet || exit 1   # refreshes Cargo.lock
grep -qF "$VER" Cargo.lock || { echo "lockfile did not pick up $VER" >&2; exit 1; }

# ---- 3. gates AFTER the bump ----
cargo test --quiet || { echo "TESTS FAILED at $VER — tree left dirty for inspection" >&2; exit 1; }
[ "$(cargo clippy --all-targets 2>&1 | grep -cE '^(error|warning)')" = "0" ] \
    || { echo "CLIPPY FAILED at $VER" >&2; exit 1; }

# ---- 4. site changelog + header, then commit + tag ----
# The about page went stale once already (header pinned at v0.10.42 while the
# changelog stopped at v0.10.21) precisely because updating it was a manual
# step nobody could remember. Do it here, before the commit, so it rides along
# in the release commit instead of being left dirty.
SITE="$ROOT/site/about.html"
if [ -f "$SITE" ]; then
    sed -i.bak -E "s|(about / )v[0-9]+\.[0-9]+\.[0-9]+|\1v$VER|" "$SITE" && rm -f "$SITE.bak"
    if ! grep -q "<span class=\"tag\">v$VER</span>" "$SITE"; then
        # Summarise the work since the previous tag. The release commit itself
        # does not exist yet and would be noise, so the range ends at HEAD.
        SITE_PREV=$(git describe --tags --abbrev=0 HEAD 2>/dev/null || true)
        python3 - "$SITE" "$VER" "$SITE_PREV" <<'PY'
import subprocess, sys
path, ver, prev = sys.argv[1], sys.argv[2], sys.argv[3]
rng = f"{prev}..HEAD" if prev else "HEAD"
lines = [l for l in subprocess.run(
    ["git", "log", "--no-merges", "--format=%s", rng],
    capture_output=True, text=True).stdout.splitlines()]
# Drop release commits: they are bookkeeping, not a change worth describing.
lines = [l for l in lines if not l.startswith("Release v")]
if not lines:
    # Never block a release over a cosmetic site update.
    print("site changelog: no substantive commits in range, skipping", file=sys.stderr)
    raise SystemExit(0)
text = "; ".join(lines)
text = (text.replace("&", "&amp;").replace("<", "&lt;")).rstrip(" .")
words, out, cur = text.split(), [], []
for w in words:
    cur.append(w)
    if len(" ".join(cur)) >= 72:
        out.append(" ".join(cur)); cur = []
if cur:
    out.append(" ".join(cur))
entry = ["        <li>", f'          <span class="tag">v{ver}</span>',
         f'          <span class="note">{out[0]}']
entry += [f"          {l}" for l in out[1:]]
entry[-1] += "."
entry.append("          </span>")
entry.append("        </li>")
html = open(path, encoding="utf-8").read()
marker = "        </li>\n"
idx = html.rindex(marker)
open(path, "w", encoding="utf-8").write(
    html[:idx + len(marker)] + "\n".join(entry) + "\n" + html[idx + len(marker):])
print(f"site changelog: added v{ver}")
PY
    fi
fi

git add Cargo.toml Cargo.lock README.md docs/api.md assets/static/openapi.json
[ -f "$SITE" ] && git add "$SITE"
git commit -m "Release v$VER" || exit 1
git push origin rust || exit 1
git tag "v$VER" || exit 1
git push origin "v$VER" || exit 1

# ---- 5. portable build + publish ----
cargo build --release --quiet || exit 1
PKGDIR=/tmp/rel-$VER
rm -rf "$PKGDIR" && mkdir -p "$PKGDIR/pkg"
cp target/release/glances-rs "$PKGDIR/pkg/glances-rs"
tar -czf "$PKGDIR/glances-rs-linux-x86_64.tar.gz" -C "$PKGDIR/pkg" glances-rs
(cd "$PKGDIR" && sha256sum glances-rs-linux-x86_64.tar.gz > SHA256SUMS)

# Release notes are generated from the commits this release actually contains,
# not a fixed string. Pure-std, zero crates, LGPL-3.0-only is a standing
# property of the project, not a per-release fact worth repeating.
NOTES="$PKGDIR/notes.md"
{
    echo "Pure-std, zero crates, LGPL-3.0-only."
    echo
    echo "## Changes in this release"
    echo
    # Previous tag found by walking history, NOT by sorting version numbers:
    # the `upstream` remote contributes v4.x Glances tags, and `v4.5.7` sorts
    # above every v0.10.x — a version sort would make it the "previous" tag
    # and dump the entire project history into every release note.
    PREV=$(git describe --tags --abbrev=0 "v$VER^" 2>/dev/null || true)
    if [ -n "$PREV" ]; then
        git log --no-merges --format='- %s' "$PREV..v$VER" | sed 's/ (v[0-9.]*)$//'
    else
        echo "_First tagged release._"
    fi
} > "$NOTES"

gh release create "v$VER" --target rust --title "glances-rs v$VER" \
    --notes-file "$NOTES" \
    "$PKGDIR/glances-rs-linux-x86_64.tar.gz" "$PKGDIR/SHA256SUMS" || exit 1
echo "published https://github.com/UberMetroid/glances-rs/releases/tag/v$VER"

# ---- 6. service cutover (optional) ----
if [ "$SERVICE" = 1 ]; then
    RUSTFLAGS='-C target-cpu=native' cargo build --release --quiet || exit 1
    # Stop BEFORE copy: the running service holds the binary (ETXTBSY).
    systemctl --user stop glances-rs
    sleep 2
    cp target/release/glances-rs "$HOME/.local/bin/glances-rs"
    systemctl --user start glances-rs
    sleep 5
    ./verify-deploy.sh || exit 1
fi
if [ "$CROSSCHECK" = 1 ]; then
    ./crosscheck.sh http://localhost:61208 || exit 1
fi
echo "release v$VER complete"
