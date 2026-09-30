//! Lint suite — enforces the hard constraints from plan §3.5 and AGENTS.md.
//!
//! These tests read the source tree at test time and fail loudly if any rule
//! is broken. They are ordinary `#[test]`s, NOT `#[ignore]`d: they run in a
//! plain `cargo test`, in `cargo test --all-targets`, and in CI.

pub mod no_crates;
pub mod line_cap;
pub mod ownership;
pub mod unsafe_allowlist;
pub mod no_shell;
pub mod naming;
