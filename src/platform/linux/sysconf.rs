//! Kernel constants via `sysconf(3)` — direct libc, no crates.
//!
//! The port avoids hardcoding assumptions like HZ or page size where a
//! one-line syscall gives the truth (each with a documented fallback so
//! a failed call degrades instead of erroring).

// safe: declaration only — no code runs here. `sysconf(int) -> long` takes and
// returns plain integers by value; there is no pointer or length in the
// signature, so nothing in the ABI can be misused.
unsafe extern "C" {
    fn sysconf(name: i32) -> i64;
}

const SC_PAGESIZE: i32 = 30;
const SC_CLK_TCK: i32 = 2;

/// Memory page size in bytes (4096 fallback).
pub fn page_size() -> u64 {
    // safe: value-in/value-out call with no pointer argument — no memory
    // safety invariant to uphold. `SC_PAGESIZE` is the glibc constant 30.
    let v = unsafe { sysconf(SC_PAGESIZE) };
    if v > 0 {
        v as u64
    } else {
        4096
    }
}

/// Clock ticks per second (100 fallback — the Linux standard).
pub fn clock_ticks() -> u64 {
    // safe: value-in/value-out call with no pointer argument — no memory
    // safety invariant to uphold. `SC_CLK_TCK` is the glibc constant 2.
    let v = unsafe { sysconf(SC_CLK_TCK) };
    if v > 0 {
        v as u64
    } else {
        100
    }
}
