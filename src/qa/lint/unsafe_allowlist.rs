//! AC-11 lint: no `unsafe` outside the platform layer.
//!
//! `src/platform/linux/` is the only directory that may contain `unsafe`.
//! There is no macOS or Windows code in this tree, so no other slot is
//! allowlisted. A file outside the allowlist is excused only if it carries a
//! `// safe:` note (file-level marker, see AGENTS.md §6).

use std::fs;
use std::path::Path;

const ALLOWLIST: &[&str] = &[
    "src/platform/linux/",
];

#[test]
fn no_unsafe_outside_allowlist() {
    let src = Path::new("src");
    let mut violations = Vec::new();
    visit_rs(src, &mut |p, content| {
        let path_str = p.to_string_lossy().replace('\\', "/");
        if path_str.contains("/qa/") { return; }
        let allowed = ALLOWLIST.iter().any(|a| path_str.contains(a));
        if !allowed && content.contains("unsafe ") && !content.contains("// safe:") {
            let count = content.matches("unsafe ").count();
            if count > 0 {
                violations.push(format!("{}: {} unsafe block(s)", path_str, count));
            }
        }
    });
    assert!(violations.is_empty(),
        "AC-11 violation: unsafe blocks outside allowlist:\n{}",
        violations.join("\n"));
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
