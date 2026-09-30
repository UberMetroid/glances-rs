//! Naming lint: no drawer-named files (AGENTS.md §3).
//!
//! A file called `util.rs` is an admission that nobody could describe its job.
//! The tree should read as a map of the running system. The tree is currently
//! clean, so this is a ratchet: it keeps it that way.

use std::fs;
use std::path::Path;

/// Names that describe a drawer rather than a job.
const BANNED: &[&str] = &[
    "util", "utils", "helper", "helpers", "common", "misc", "shared", "base", "core",
];

#[test]
fn no_drawer_named_rs_files() {
    let mut violations = Vec::new();
    visit_rs(Path::new("src"), &mut |p| {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if BANNED.contains(&stem) {
            violations.push(format!("{} — name the function, not the drawer", p.display()));
        }
    });
    assert!(violations.is_empty(),
        "drawer-named file(s) under src/:\n{}", violations.join("\n"));
}

fn visit_rs(dir: &Path, cb: &mut dyn FnMut(&Path)) {
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() { visit_rs(&p, cb); }
            else if p.extension().and_then(|s| s.to_str()) == Some("rs") { cb(&p); }
        }
    }
}
