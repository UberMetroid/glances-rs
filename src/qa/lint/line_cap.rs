//! File-size lint: every .rs file is 16–256 lines (comments and blank
//! lines count). The floor is waived for module-wiring shims — `mod.rs` /
//! `lib.rs` files whose every code line declares a module or re-export.
//! Anything else under 16 lines folds into its closest functionally-aligned
//! sibling. See AGENTS.md §2.

use std::fs;
use std::path::Path;

const FLOOR: usize = 16;
const CAP: usize = 256;

#[test]
fn every_rs_file_within_16_to_256_lines() {
    let src = Path::new("src");
    let mut violations = Vec::new();
    visit_rs(src, &mut |p, content| {
        let n = content.lines().count();
        if n > CAP {
            violations.push(format!("{}: {} lines (cap {})", p.display(), n, CAP));
        } else if n < FLOOR && !is_shim(p, content) {
            violations.push(format!("{}: {} lines (floor {})", p.display(), n, FLOOR));
        }
    });
    assert!(violations.is_empty(),
        "file-size violation, want 16–256 lines:\n{}", violations.join("\n"));
}

/// True for a `mod.rs`/`lib.rs` that only wires modules together.
///
/// The test is on content, not filename: a short `mod.rs` carrying real
/// logic is not a shim and must still clear the floor on its own merit.
fn is_shim(path: &Path, content: &str) -> bool {
    let wiring_name = path.file_name().and_then(|s| s.to_str())
        .is_some_and(|f| f == "mod.rs" || f == "lib.rs");
    if !wiring_name {
        return false;
    }
    let mut saw_declaration = false;
    for line in content.lines() {
        let t = line.trim();
        // blank, doc/line comment, or attribute — not code
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        if t.starts_with("mod ") || t.starts_with("use ") || t.starts_with("pub ") {
            saw_declaration = true;
            continue;
        }
        // the closing brace of a `pub const`/`const` block is still wiring
        if t == "}" || t == "};" {
            continue;
        }
        return false; // real code in a wiring file
    }
    saw_declaration
}

fn visit_rs(dir: &Path, cb: &mut dyn FnMut(&Path, &str)) {
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() { visit_rs(&p, cb); }
            else if p.extension().and_then(|s| s.to_str()) == Some("rs")
                && let Ok(t) = fs::read_to_string(&p) { cb(&p, &t); }
        }
    }
}
