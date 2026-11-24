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
//! and the workspace forbids it.

use std::process::Command;

const MODE: &str = "FUZZY_SCORE_MEMORY_CHILD";

fn peak_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// An adjacent swap in an all-distinct alphabet: the only shape the
/// transposition term can score as one edit, so a missing snapshot shows up.
fn swapped(n: usize) -> (String, String) {
    let a: String = (0..n)
        .map(|i| char::from_u32(0x4e00 + i as u32).unwrap())
        .collect();
    let mut chars: Vec<char> = a.chars().collect();
    chars.swap(n / 2 - 1, n / 2);
    (a, chars.into_iter().collect())
}

#[test]
fn damerau_peak_memory_stays_within_the_snapshot_bound() {
    let n = 2048;

    if std::env::var_os(MODE).is_some() {
        // Child: one measurement, from a process that has done nothing else.
        let (a, b) = swapped(n);
        assert_eq!(fuzzy_score::distance::damerau_levenshtein(&a, &b), 1);
        assert_eq!(fuzzy_score::distance::levenshtein(&a, &b), 2);

        // The documented bound is `s * min(m, n)` snapshot cells. Here
        // `s == m == n == 2048`, so that is about 33 MiB. A full matrix would be
        // another 33 MiB on top, so a 48 MiB ceiling separates them: above the
        // documented bound, below an implementation that keeps both.
        let peak = peak_kib().expect("VmHWM readable in the child");
        assert!(
            peak < 48 * 1024,
            "peak RSS {peak} KiB, above the 48 MiB ceiling that the snapshot \
             bound allows and a full matrix exceeds"
        );
        // A floor, so a reading of zero cannot pass: the snapshots have to exist
        // for the answer above to be 1, and they cost about 33 MiB.
        assert!(
            peak > 16 * 1024,
            "peak RSS {peak} KiB is below the 16 MiB the snapshots need, so \
             this measured nothing"
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
}
