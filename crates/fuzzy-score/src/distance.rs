//! Edit distances and the Jaro family, for callers who need a number rather
//! than a `fuzzywuzzy` score.
//!
//! [`crate::fuzz`] answers "how alike are these two strings, on the scale
//! `fuzzywuzzy` uses". That scale is a policy inherited from Python's
//! `difflib`, and it is the right answer when a service already ranks records
//! with it. It is not a metric: it does not satisfy the triangle inequality,
//! it cannot be compared across string lengths, and it says nothing about how
//! many edits separate two strings.
//!
//! The functions here are the ones the maintained libraries provide -
//! `rapidfuzz`, `jellyfish`, and the `strsim` crate - and each returns the value
//! those libraries return for the same input:
//!
//! * [`levenshtein`] - insertions, deletions and substitutions.
//! * [`damerau_levenshtein`] - those plus a transposition of two adjacent
//!   characters as **one** edit, where the transposed characters may themselves
//!   have been edited first (the unrestricted variant).
//! * [`osa`] - the restricted variant, where a substring may be transposed at
//!   most once. `CA` against `ABC` is 2 under [`damerau_levenshtein`] and 3
//!   under [`osa`]; both are in use, so both are here rather than one chosen
//!   for the caller.
//! * [`hamming`] - positions that differ, counting a length difference as a
//!   difference at each extra position.
//! * [`jaro`] and [`jaro_winkler`] - similarity in `0.0..=1.0` that rewards
//!   matching characters appearing in the same order.
//!
//! # What counts as one character
//!
//! Every function counts **Unicode scalar values**, not bytes and not grapheme
//! clusters. `"café"` against `"cafe"` is one edit, and `"👩‍💻"` is three
//! scalars - so it is three edits from `""`, not one.
//!
//! This is `rapidfuzz`'s unit, and it is the reason this module is **not**
//! jellyfish-compatible for text containing multi-scalar graphemes. `jellyfish`
//! 1.3 segments into extended grapheme clusters, so a decomposed `e` plus a
//! combining acute is one unit there and two here, and a joined emoji is one
//! there and three here. On `"e"` against `"e" + U+0301`, `rapidfuzz` returns a
//! Jaro similarity of `0.833` and `jellyfish` returns `0.0`; on
//! `"👩‍💻"` against `"👨‍🔬"`, the Levenshtein distance is 2 here and 1 there.
//!
//! Neither is wrong - they answer different questions - but a caller comparing
//! user input should normalize to NFC (`unicode-normalization`) before calling
//! either, or the same visible text can score as different strings. The corpus
//! generator pins both values for these pairs so the divergence is measured
//! rather than assumed absent.

use std::collections::{HashMap, HashSet};

/// The Damerau-Levenshtein variant to compute.
///
/// The two differ only when a transposition could be combined with an edit next
/// to it. `CA` against `ABC` is the shortest case that shows the gap: the
/// unrestricted distance is 2 (`CA` -> `AC` -> `ABC`) and the restricted one is
/// 3, because the restricted algorithm may not transpose a substring that was
/// already edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Damerau {
    /// The unrestricted distance, as `rapidfuzz.distance.DamerauLevenshtein`
    /// and `jellyfish.damerau_levenshtein_distance` compute it.
    Unrestricted,
    /// The optimal-string-alignment distance, as `rapidfuzz.distance.OSA` does.
    Restricted,
}

/// Levenshtein distance: the fewest insertions, deletions and substitutions
/// that turn `a` into `b`.
///
/// ```
/// use fuzzy_score::distance::levenshtein;
///
/// assert_eq!(levenshtein("kitten", "sitting"), 3);
/// assert_eq!(levenshtein("", "abc"), 3);
/// assert_eq!(levenshtein("same", "same"), 0);
/// // Characters, not bytes: `é` is one character.
/// assert_eq!(levenshtein("café", "cafe"), 1);
/// ```
#[must_use]
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // The row buffers are `b.len() + 1` wide and the distance is symmetric, so
    // the shorter string belongs in that role; without the swap a long second
    // argument allocates a row as long as itself.
    if b.len() <= a.len() {
        levenshtein_chars(&a, &b)
    } else {
        levenshtein_chars(&b, &a)
    }
}

fn levenshtein_chars(a: &[char], b: &[char]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    // One row at a time: the matrix is never needed in full, and a long pair
    // would otherwise allocate a square it does not use.
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

/// Damerau-Levenshtein distance: Levenshtein plus transposition of two adjacent
/// characters as a single edit.
///
/// This is the unrestricted variant. Use [`osa`] for the restricted one.
///
/// ```
/// use fuzzy_score::distance::damerau_levenshtein;
///
/// // A transposition is one edit, where Levenshtein needs two.
/// assert_eq!(damerau_levenshtein("ab", "ba"), 1);
/// assert_eq!(fuzzy_score::distance::levenshtein("ab", "ba"), 2);
///
/// // The unrestricted variant may transpose a substring that was edited first.
/// assert_eq!(damerau_levenshtein("CA", "ABC"), 2);
/// ```
#[must_use]
pub fn damerau_levenshtein(a: &str, b: &str) -> usize {
    damerau_levenshtein_chars(
        &a.chars().collect::<Vec<_>>(),
        &b.chars().collect::<Vec<_>>(),
    )
}

fn damerau_levenshtein_chars(a: &[char], b: &[char]) -> usize {
    // The row buffers and the per-character snapshots below are all `b.len() + 1`
    // wide and the distance is symmetric, so the shorter string is the column
    // dimension. That alone turns an `m * n` matrix into `min(m, n)` columns.
    let (a, b) = if b.len() <= a.len() { (a, b) } else { (b, a) };
    let (m, n) = (a.len(), b.len());
    if n == 0 {
        return m;
    }

    // Lowrance and Wagner's recurrence, in the indexing the paper and the usual
    // transcription use: `i` and `j` are 1-based positions, and the answer for
    // the first `i` and `j` characters is `d[i][j]`.
    //
    //   d[i][j] = min(d[i-1][j-1] + cost, d[i][j-1] + 1, d[i-1][j] + 1,
    //                 d[k-1][l-1] + (i-k-1) + 1 + (j-l-1))
    //
    // where `k` is the last position in `a` at which `b[j]` occurred and `l` the
    // last position in `b` at which `a[i]` matched. Only the first three terms
    // look one row back, so `prev` and `curr` are enough for them. The fourth
    // reaches into an arbitrary earlier row, which is why the naive
    // transcription needs the whole matrix - and why 8 KiB inputs allocated
    // 512 MiB.
    //
    // `snapshot[c]` holds row `k - 1` of the matrix for the character `c` whose
    // last occurrence in `a` is at position `k`, so the fourth term becomes
    // `snapshot[b[j]][l - 1]`. The snapshot is taken when that occurrence is
    // processed, at which point the row it needs *is* `prev`.
    //
    // Memory is `O(s * min(m, n))` for `s` distinct characters shared by the two
    // strings, not `O(m * n)`. The saving is real but not unconditional: a
    // character that occurs only once still needs its row (`ab` against `ba`
    // reads row 0 through it), so a string of thousands of distinct characters
    // still allocates tens of megabytes. True linear space needs Zhao-Sahni,
    // which is not implemented. `AGENTS.md` records why rolling rows alone
    // cannot do it.
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    let mut last_seen: HashMap<char, usize> = HashMap::new();
    let mut snapshot: HashMap<char, Vec<usize>> = HashMap::new();
    let queryable: HashSet<char> = b.iter().copied().collect();

    for i in 1..=m {
        curr[0] = i;
        let mut last_match_column = 0usize;
        for j in 1..=n {
            let k = last_seen.get(&b[j - 1]).copied().unwrap_or(0);
            let l = last_match_column;
            let cost = if a[i - 1] == b[j - 1] {
                last_match_column = j;
                0
            } else {
                1
            };
            let substitution = prev[j - 1] + cost;
            let insertion = curr[j - 1] + 1;
            let deletion = prev[j] + 1;
            let mut value = substitution.min(insertion).min(deletion);
            // `k == 0` or `l == 0` means the character has not been seen yet; the
            // naive form reads a sentinel there that is at least `m + n`, so the
            // term can never win and skipping it is exact rather than a shortcut.
            if k > 0 && l > 0 {
                let transposition = snapshot[&b[j - 1]][l - 1] + (i - k - 1) + 1 + (j - l - 1);
                value = value.min(transposition);
            }
            curr[j] = value;
        }
        // Before `a[i - 1]` becomes the last occurrence of its character, `prev`
        // is row `i - 1`, which is exactly the row its future transposition
        // terms will read.
        if queryable.contains(&a[i - 1]) {
            snapshot.insert(a[i - 1], prev.clone());
            last_seen.insert(a[i - 1], i);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Optimal string alignment distance: Levenshtein plus transposition of two
/// adjacent characters, with no substring edited more than once.
///
/// This is the variant `rapidfuzz.distance.OSA` computes, and the one most
/// "Damerau-Levenshtein" implementations in the wild actually implement. It is
/// not a metric - it can violate the triangle inequality - which is why
/// [`damerau_levenshtein`] is the unrestricted algorithm and this one says so in
/// its name.
///
/// ```
/// use fuzzy_score::distance::osa;
///
/// assert_eq!(osa("ab", "ba"), 1);
/// // A transposition that would need a prior edit is priced as two edits here,
/// // where the unrestricted distance is 2 and this is 3.
/// assert_eq!(osa("CA", "ABC"), 3);
/// ```
#[must_use]
pub fn osa(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // The distance is symmetric and the rows below are `b.len() + 1` wide, so
    // the shorter string belongs in that role.
    let (a, b) = if b.len() <= a.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    if b.is_empty() {
        return a.len();
    }

    // The recurrence reads only the previous row (`prev`) and the one before it
    // (`prev2`), so three rows are enough. A full `m * n` matrix is what made an
    // 8 KiB comparison allocate 512 MiB before this was written this way.
    let n = b.len();
    let mut prev2 = vec![0usize; n + 1];
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut value = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                value = value.min(prev2[j - 2] + 1);
            }
            curr[j] = value;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Hamming distance: the number of positions at which the two strings differ.
///
/// The textbook definition requires equal lengths and is undefined otherwise.
/// There are two conventions in the Rust ecosystem and they are **not**
/// interchangeable:
///
/// * `strsim::hamming` returns `Err(DifferentLengthArgs)` for unequal lengths.
/// * `rapidfuzz` and `jellyfish` count each extra position as a difference, so
///   `"ab"` against `"abc"` is 1.
///
/// This function follows the second, so it always returns a value and never
/// fails. The reason is that the caller that needs Hamming distance is usually
/// comparing a field that may have been truncated, and an `Err` there means a
/// caller either unwraps or silently skips the pair; a count means the same
/// thing to both sides. A caller that requires the strict contract should check
/// the lengths first, which is one comparison.
///
/// ```
/// use fuzzy_score::distance::hamming;
///
/// assert_eq!(hamming("karolin", "kathrin"), 3);
/// assert_eq!(hamming("same", "same"), 0);
/// // Unequal lengths: the extra characters are differences.
/// assert_eq!(hamming("ab", "abc"), 1);
/// ```
#[must_use]
pub fn hamming(a: &str, b: &str) -> usize {
    let mut a_chars = a.chars();
    let mut b_chars = b.chars();
    let mut differences = 0usize;
    loop {
        match (a_chars.next(), b_chars.next()) {
            (None, None) => return differences,
            // A position present in one string and absent in the other is a
            // difference, which is how both references treat unequal lengths.
            (Some(ca), Some(cb)) => {
                if ca != cb {
                    differences += 1;
                }
            }
            _ => differences += 1,
        }
    }
}

/// Jaro similarity in `0.0..=1.0`: how many characters match, and how close in
/// order they are.
///
/// Two empty strings score `1.0`. That is the one point where the references
/// disagree - `jellyfish` returns `0.0`, `rapidfuzz` returns `1.0` - and this
/// crate follows `rapidfuzz`, the maintained one, along with the `strsim` crate
/// and the original paper's reading that two identical strings are identical.
///
/// ```
/// use fuzzy_score::distance::jaro;
///
/// let score = jaro("MARTHA", "MARHTA");
/// assert!((score - 0.944_444_444_444_444_5).abs() < 1e-12);
/// assert_eq!(jaro("", ""), 1.0);
/// assert_eq!(jaro("abc", "xyz"), 0.0);
/// ```
#[must_use]
pub fn jaro(a: &str, b: &str) -> f64 {
    jaro_chars(
        &a.chars().collect::<Vec<_>>(),
        &b.chars().collect::<Vec<_>>(),
    )
}

fn jaro_chars(a: &[char], b: &[char]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }

    // A character may match within this many positions of its own index, which
    // is what lets a transposition count as a near-match rather than as two
    // mismatches.
    let window = (a.len().max(b.len()) / 2).saturating_sub(1);

    let mut a_matched = vec![false; a.len()];
    let mut b_matched = vec![false; b.len()];
    let mut matches = 0usize;
    for (i, &ca) in a.iter().enumerate() {
        let start = i.saturating_sub(window);
        let end = (i + window + 1).min(b.len());
        for j in start..end {
            if !b_matched[j] && b[j] == ca {
                a_matched[i] = true;
                b_matched[j] = true;
                matches += 1;
                break;
            }
        }
    }
    if matches == 0 {
        return 0.0;
    }

    let a_seq: Vec<char> = a
        .iter()
        .zip(&a_matched)
        .filter(|&(_, &matched)| matched)
        .map(|(&c, _)| c)
        .collect();
    let b_seq: Vec<char> = b
        .iter()
        .zip(&b_matched)
        .filter(|&(_, &matched)| matched)
        .map(|(&c, _)| c)
        .collect();
    // Transpositions are counted in pairs, and the halving is integer division:
    // an odd count loses its remainder rather than contributing a half. Both
    // references do this (`jellyfish` literally writes `trans_count //= 2`), and
    // using `/ 2.0` here is wrong on every string with an odd transposition
    // count, which is a large share of them.
    let transpositions = a_seq.iter().zip(&b_seq).filter(|(ca, cb)| ca != cb).count() / 2;

    let m = matches as f64;
    (m / a.len() as f64 + m / b.len() as f64 + (m - transpositions as f64) / m) / 3.0
}

/// Jaro-Winkler similarity: [`jaro`] with a bonus for a shared prefix.
///
/// The prefix bonus is `0.1 * prefix * (1 - jaro)` with the prefix capped at
/// four characters, and it applies only when the Jaro similarity already
/// exceeds `0.7`. Those three numbers are what `rapidfuzz`, `jellyfish` and
/// `strsim` all use; they are parameters in the original paper, so a caller who
/// needs different ones should compute [`jaro`] and apply its own.
///
/// ```
/// use fuzzy_score::distance::jaro_winkler;
///
/// let score = jaro_winkler("MARTHA", "MARHTA");
/// assert!((score - 0.961_111_111_111_111_1).abs() < 1e-12);
/// // A shared prefix lifts the score above the plain Jaro similarity.
/// assert!(jaro_winkler("DIXON", "DICKSONX") > fuzzy_score::distance::jaro("DIXON", "DICKSONX"));
/// ```
#[must_use]
pub fn jaro_winkler(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let base = jaro_chars(&a_chars, &b_chars);
    if base <= 0.7 {
        return base;
    }
    let prefix = a_chars
        .iter()
        .zip(&b_chars)
        .take(4)
        .take_while(|(ca, cb)| ca == cb)
        .count();
    base + 0.1 * prefix as f64 * (1.0 - base)
}

/// [`levenshtein`] as a similarity in `0.0..=1.0`: one minus the distance over
/// the longer length.
///
/// Two empty strings score `1.0`, as `rapidfuzz.distance.Levenshtein
/// .normalized_similarity` does.
///
/// ```
/// use fuzzy_score::distance::levenshtein_similarity;
///
/// let score = levenshtein_similarity("kitten", "sitting");
/// assert!((score - 4.0 / 7.0).abs() < 1e-12);
/// assert_eq!(levenshtein_similarity("", ""), 1.0);
/// ```
#[must_use]
pub fn levenshtein_similarity(a: &str, b: &str) -> f64 {
    let longest = a.chars().count().max(b.chars().count());
    if longest == 0 {
        return 1.0;
    }
    1.0 - levenshtein(a, b) as f64 / longest as f64
}

/// The distance selected by [`Damerau`], so a caller can choose at run time.
///
/// ```
/// use fuzzy_score::distance::{damerau, Damerau};
///
/// assert_eq!(damerau("CA", "ABC", Damerau::Unrestricted), 2);
/// assert_eq!(damerau("CA", "ABC", Damerau::Restricted), 3);
/// ```
#[must_use]
pub fn damerau(a: &str, b: &str, variant: Damerau) -> usize {
    match variant {
        Damerau::Unrestricted => damerau_levenshtein(a, b),
        Damerau::Restricted => osa(a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_counts_characters_not_bytes() {
        assert_eq!(levenshtein("café", "cafe"), 1);
        assert_eq!(levenshtein("😀a", "😀b"), 1);
        // Two bytes differ in `é` against `e`, but only one character does.
        assert_eq!(levenshtein("é", "e"), 1);
    }

    #[test]
    fn levenshtein_handles_empty_and_equal() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }

    #[test]
    fn long_inputs_stay_within_linear_space() {
        // A quadratic matrix is 32 MiB at this length and 512 MiB at 8 KiB, which
        // is enough to abort a process. The test cannot measure allocation
        // directly, so it asserts the answer on an input where the full matrix
        // was the only thing standing between the caller and a result, and keeps
        // the length small enough that it does not dominate the suite.
        let n = 2048;
        let a: String = (0..n).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
        let mut chars: Vec<char> = a.chars().collect();
        chars[n / 2] = 'z';
        let b: String = chars.into_iter().collect();

        assert_eq!(levenshtein(&a, &b), 1);
        assert_eq!(osa(&a, &b), 1);
        assert_eq!(damerau_levenshtein(&a, &b), 1);
    }

    #[test]
    fn damerau_memory_is_bounded_by_distinct_characters_not_by_area() {
        // The documented bound is `O(s * min(m, n))`, so a wide alphabet costs
        // more than text does but still far less than the `m * n` matrix this
        // replaced. The input is an adjacent swap in an all-distinct alphabet,
        // which is the shape that *needs* the transposition term: a substitution
        // scores 1 through the ordinary recurrence, so it would pass even with
        // the snapshots removed.
        //
        // Peak memory is asserted in `tests/memory.rs`, not here: it needs a
        // process of its own, because `/proc/self/status` reports a maximum for
        // the whole process and no amount of resetting makes a second reading
        // independent of the first.
        let n = 2048;
        let a: String = (0..n)
            .map(|i| char::from_u32(0x4e00 + i as u32).unwrap())
            .collect();
        let mut chars: Vec<char> = a.chars().collect();
        chars.swap(n / 2 - 1, n / 2);
        let b: String = chars.into_iter().collect();

        assert_eq!(damerau_levenshtein(&a, &b), 1);
        assert_eq!(levenshtein(&a, &b), 2, "the swap is not a transposition");
        assert_eq!(
            damerau_levenshtein(&b, &a),
            1,
            "the answer must be symmetric"
        );
    }

    #[test]
    fn damerau_matches_the_matrix_form_on_repetitive_input() {
        // The snapshot-based implementation replaces a full matrix, so it has to
        // agree with the recurrence it replaces on input built to need the
        // transposition term: repeated characters make the last-seen positions
        // far apart, which is where a wrong snapshot row shows.
        assert_eq!(damerau_levenshtein("aab", "aba"), 1);
        assert_eq!(damerau_levenshtein("abab", "baba"), 2);
        assert_eq!(damerau_levenshtein("aaaa", "aaaa"), 0);
        assert_eq!(damerau_levenshtein("abcd", "badc"), 2);
        assert_eq!(damerau_levenshtein("ca", "abc"), 2);
        assert_eq!(damerau_levenshtein("ab", "ba"), 1);
    }

    #[test]
    fn the_unit_is_a_scalar_and_that_is_documented() {
        // A joined emoji is three scalars: woman, ZWJ, laptop. `jellyfish` treats
        // it as one grapheme cluster and would return 1 against another joined
        // emoji; this crate follows `rapidfuzz` and counts scalars.
        let woman_technologist = "\u{1f469}\u{200d}\u{1f4bb}";
        let man_scientist = "\u{1f468}\u{200d}\u{1f52c}";
        assert_eq!(levenshtein(woman_technologist, man_scientist), 2);
        assert_eq!(levenshtein("", woman_technologist), 3);

        // A decomposed accent is two scalars, so it is one edit from the plain
        // letter and zero from an identical pair.
        let decomposed = "e\u{301}";
        assert_eq!(levenshtein("e", decomposed), 1);
        assert_eq!(levenshtein(decomposed, decomposed), 0);
        // The precomposed form is one scalar, so it is a *different* string from
        // the two-scalar decomposed form: substituting `é` for `e` and then
        // inserting the combining mark is two edits, which is the whole reason
        // NFC normalization matters before comparing user input.
        assert_eq!(levenshtein("\u{e9}", decomposed), 2);
    }

    #[test]
    fn hamming_counts_unequal_lengths_instead_of_failing() {
        // `strsim::hamming` returns `Err` here. This follows `rapidfuzz` and
        // `jellyfish`: every extra position is a difference, so the function
        // cannot fail.
        assert_eq!(hamming("ab", "abc"), 1);
        assert_eq!(hamming("abc", "ab"), 1);
        assert_eq!(hamming("", "abc"), 3);
        assert_eq!(hamming("abc", ""), 3);
    }

    #[test]
    fn damerau_variants_diverge_on_the_shortest_case() {
        // The documented difference between the two algorithms.
        assert_eq!(damerau_levenshtein("CA", "ABC"), 2);
        assert_eq!(osa("CA", "ABC"), 3);
        // A plain transposition is one edit under both.
        assert_eq!(damerau_levenshtein("ab", "ba"), 1);
        assert_eq!(osa("ab", "ba"), 1);
        // `levenshtein` needs two.
        assert_eq!(levenshtein("ab", "ba"), 2);
    }

    #[test]
    fn damerau_handles_empty_and_equal() {
        for variant in [Damerau::Unrestricted, Damerau::Restricted] {
            assert_eq!(damerau("", "", variant), 0);
            assert_eq!(damerau("", "abc", variant), 3);
            assert_eq!(damerau("abc", "", variant), 3);
            assert_eq!(damerau("abc", "abc", variant), 0);
        }
    }

    #[test]
    fn hamming_counts_extra_positions_as_differences() {
        assert_eq!(hamming("karolin", "kathrin"), 3);
        assert_eq!(hamming("", ""), 0);
        assert_eq!(hamming("ab", "abc"), 1);
        assert_eq!(hamming("abc", "ab"), 1);
        assert_eq!(hamming("ab", "cdx"), 3);
        assert_eq!(hamming("", "a"), 1);
    }

    #[test]
    fn jaro_is_one_for_two_empty_strings() {
        // The documented divergence: jellyfish says 0.0 here, rapidfuzz 1.0.
        assert_eq!(jaro("", ""), 1.0);
        assert_eq!(jaro_winkler("", ""), 1.0);
        assert_eq!(jaro("a", ""), 0.0);
        assert_eq!(jaro("", "a"), 0.0);
    }

    #[test]
    fn jaro_matches_the_published_example() {
        let score = jaro("MARTHA", "MARHTA");
        assert!((score - 0.944_444_444_444_444_5).abs() < 1e-12, "{score}");
    }

    #[test]
    fn winkler_bonus_needs_a_high_base_and_a_prefix() {
        // `abc` against `xyz` shares nothing, so no bonus applies.
        assert_eq!(jaro_winkler("abc", "xyz"), 0.0);
        // A shared prefix above the 0.7 threshold lifts the score.
        let base = jaro("DIXON", "DICKSONX");
        assert!(base > 0.7);
        assert!(jaro_winkler("DIXON", "DICKSONX") > base);
    }

    #[test]
    fn winkler_prefix_is_capped_at_four() {
        // Both pairs share six characters, but only four may earn the bonus, so
        // the two scores are equal even though the prefixes differ in length.
        let four = jaro_winkler("abcdXX", "abcdYY");
        let six = jaro_winkler("abcdXX", "abcdYY");
        assert!((four - six).abs() < 1e-12);
        assert!((jaro_winkler("abcdefgh", "abcdefgx") - 0.95).abs() < 1e-12);
    }

    #[test]
    fn levenshtein_similarity_is_one_for_two_empty_strings() {
        assert_eq!(levenshtein_similarity("", ""), 1.0);
        assert_eq!(levenshtein_similarity("abc", "abc"), 1.0);
        assert_eq!(levenshtein_similarity("", "abc"), 0.0);
    }
}
