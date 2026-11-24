//! Peak memory, measured in a child process of its own.
//!
//! `/proc/self/status` reports `VmHWM` as a maximum for the whole process, so a
//! reading taken inside a test binary that has already run other tests is
//! contaminated by them, and `clear_refs` does not reset `VmPeak`. A test that
//! took a delta around one call would pass while measuring nothing. Re-executing
//! this binary with an environment variable set makes the child take one reading
//! from a fresh process, which is the only reading that means anything.
//!
//! The allocator cannot be counted instead: a global allocator needs `unsafe`,
//! and the workspace forbids it. `VmHWM` also only sees pages that were written,
//! which is why the case below is chosen so that its rows are written.
//!
//! Where `/proc` does not exist - macOS, Windows - there is no reading to take,
//! so the test skips instead of failing: a portable crate should not carry a
//! test that can only pass on Linux.

use std::process::Command;

const MODE: &str = "FUZZY_SCORE_MEMORY_CHILD";

fn peak_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// A long string over a two-character alphabet with one adjacent swap.
///
/// The alphabet is deliberately tiny. With two shared characters the snapshots
/// cost `2 * (n + 1)` cells, which is tens of kilobytes, while a full matrix
/// costs `n * n` cells, which is 128 MiB at this length. A wide alphabet cannot
/// make that distinction: `s == n` there makes the two costs the same size, so
/// the ceiling would have to allow both.
///
/// The swap is the only edit that needs the transposition term, so the answer
/// proves the snapshots exist and the peak proves they are not a matrix.
fn narrow_swapped(n: usize) -> (String, String) {
    let a: String = (0..n).map(|i| if i % 2 == 0 { 'a' } else { 'b' }).collect();
    let mut chars: Vec<char> = a.chars().collect();
    let mut i = chars.len() / 2;
    while chars[i] == chars[i + 1] {
        i += 1;
    }
    chars.swap(i, i + 1);
    (a, chars.into_iter().collect())
}

#[test]
fn damerau_peak_memory_stays_within_the_snapshot_bound() {
    let n = 4096;

    if std::env::var_os(MODE).is_some() {
        // Child: one measurement, from a process that has done nothing else.
        if peak_kib().is_none() {
            // No `/proc` here, so there is nothing to measure.
            return;
        }

        let (a, b) = narrow_swapped(n);
        assert_eq!(
            fuzzy_score::distance::damerau_levenshtein(&a, &b),
            1,
            "the input is not a transposition"
        );
        assert_eq!(fuzzy_score::distance::levenshtein(&a, &b), 2);

        // The documented bound is `s * min(m, n)` snapshot cells: two
        // characters over 4,096 columns is about 64 KiB, and the input copies add
        // 32 KiB. A full matrix would be 4,096^2 * 8 = 128 MiB, so this ceiling
        // is far above the documented bound and far below a matrix.
        let peak = peak_kib().expect("VmHWM readable in the child");
        assert!(
            peak < 16 * 1024,
            "peak RSS {peak} KiB, above the 16 MiB ceiling: two characters need \
             kilobytes of snapshots, so this measured a matrix"
        );
        return;
    }

    // Parent: re-exec this binary so the child starts with a clean high-water
    // mark, and run only this test there.
    let exe = std::env::current_exe().expect("current test executable");
    let output = Command::new(exe)
        .args([
            "--exact",
            "damerau_peak_memory_stays_within_the_snapshot_bound",
            "--nocapture",
        ])
        .env(MODE, "1")
        .output()
        .expect("spawn the measurement child");
    assert!(
        output.status.success(),
        "measurement child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    if peak_kib().is_none() {
        eprintln!("skipping the peak-memory assertion: no /proc/self/status here");
    }
}
