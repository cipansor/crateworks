//! The standard Arabic ligatures.
//!
//! Unicode defines hundreds of Arabic ligatures, but only a handful are in
//! ordinary use: the four lam-alef combinations and the word "Allah". Those are
//! the ones a renderer is expected to produce, and the ones this module
//! substitutes.
//!
//! Substitution happens on the *shaped* text, so a pattern is a sequence of
//! letters with the shape each must be in. Naming the shape rather than the
//! presentation form is what makes the pattern work in every table: the default
//! table renders a standalone alef as a presentation form while `ArabicV2` and
//! `Kurdish` leave it as the base letter, and a pattern written in presentation
//! forms would silently stop matching there. It is also what makes the context
//! fall out for free: an isolated alef followed by initial-lam, medial-lam and
//! final-heh is the word "Allah", whereas the same letters in another position
//! have different shapes and are correctly left alone.

use crate::forms::{FINAL, INITIAL, ISOLATED, MEDIAL};

/// One character of shaped output and, when it is a shaped letter, the letter it
/// came from and the index of the shape it took. The shape index, not the
/// character, is what the ligature patterns match on, because a table may give
/// several shapes the same character.
pub(crate) type Shaped = (char, Option<(char, usize)>);

/// A shaped sequence and the single character it collapses to.
///
/// The pattern is a sequence of `(base letter, shape)` pairs rather than a
/// sequence of presentation forms. That is what lets the same ligature be found
/// in every table: the default table shapes a standalone alef as a presentation
/// form, while `ArabicV2` and `Kurdish` leave it as the base letter, so a
/// pattern written in presentation forms would silently stop matching there.
struct Ligature {
    /// The letters to look for, each with the shape it must be in.
    pattern: &'static [(char, usize)],
    /// What they become.
    replacement: char,
}

/// The lam-alef ligatures and the word "Allah".
const LIGATURES: [Ligature; 9] = [
    // Lam followed by an alef. The first pair is the initial lam, which happens
    // when the lam starts a word; the second is the medial form, which happens
    // after a joining letter.
    Ligature {
        pattern: &[('\u{0644}', INITIAL), ('\u{0627}', FINAL)],
        replacement: '\u{fefb}',
    },
    Ligature {
        pattern: &[('\u{0644}', MEDIAL), ('\u{0627}', FINAL)],
        replacement: '\u{fefc}',
    },
    // Lam with alef-hamza-above.
    Ligature {
        pattern: &[('\u{0644}', INITIAL), ('\u{0623}', FINAL)],
        replacement: '\u{fef7}',
    },
    Ligature {
        pattern: &[('\u{0644}', MEDIAL), ('\u{0623}', FINAL)],
        replacement: '\u{fef8}',
    },
    // Lam with alef-hamza-below.
    Ligature {
        pattern: &[('\u{0644}', INITIAL), ('\u{0625}', FINAL)],
        replacement: '\u{fef9}',
    },
    Ligature {
        pattern: &[('\u{0644}', MEDIAL), ('\u{0625}', FINAL)],
        replacement: '\u{fefa}',
    },
    // Lam with alef-madda-above.
    Ligature {
        pattern: &[('\u{0644}', INITIAL), ('\u{0622}', FINAL)],
        replacement: '\u{fef5}',
    },
    Ligature {
        pattern: &[('\u{0644}', MEDIAL), ('\u{0622}', FINAL)],
        replacement: '\u{fef6}',
    },
    // The word "Allah": isolated alef, initial lam, medial lam, final heh.
    Ligature {
        pattern: &[
            ('\u{0627}', ISOLATED),
            ('\u{0644}', INITIAL),
            ('\u{0644}', MEDIAL),
            ('\u{0647}', FINAL),
        ],
        replacement: '\u{fdf2}',
    },
];

/// Replaces the standard ligatures in shaped text.
///
/// The input is the shaped text as characters, each tagged with the letter and
/// shape it came from when it is a shaped letter. Matching on the shape index
/// rather than the character is what makes the same pattern work in every table
/// and stay correct where a table gives several shapes the same character.
///
/// Combining marks are transparent to the match: they are skipped while the
/// pattern is compared against the letters, and the ones that fell inside the
/// span are re-emitted after the replacement. Without that, `اللّٰه` - the word
/// "Allah" with the shadda it is normally written with - would not collapse,
/// because the mark sits between the second lam and the heh.
///
/// The scan is left to right and non-overlapping, and a replacement is never
/// itself rescanned, so "lam alef alef" collapses only the first pair.
pub(crate) fn apply(shaped: &[Shaped]) -> String {
    let mut out = String::with_capacity(shaped.len());
    let mut i = 0;
    'outer: while i < shaped.len() {
        if crate::is_harakat(shaped[i].0) {
            out.push(shaped[i].0);
            i += 1;
            continue;
        }
        for ligature in &LIGATURES {
            let mut position = i;
            let mut marks = Vec::new();
            let mut matched = true;
            for (index, (base, shape)) in ligature.pattern.iter().enumerate() {
                if index > 0 {
                    while position < shaped.len() && crate::is_harakat(shaped[position].0) {
                        marks.push(shaped[position].0);
                        position += 1;
                    }
                }
                let expected = (*base, *shape);
                let Some((letter, actual)) = shaped.get(position).and_then(|entry| entry.1) else {
                    matched = false;
                    break;
                };
                if (letter, actual) == expected {
                    position += 1;
                } else {
                    matched = false;
                    break;
                }
            }
            if matched {
                out.push(ligature.replacement);
                out.extend(marks);
                i = position;
                continue 'outer;
            }
        }
        out.push(shaped[i].0);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    /// Builds the tagged input the ligature pass takes: each `(letter, shape)`
    /// becomes the character that letter has in that shape, tagged with the
    /// letter and the shape.
    fn tagged(language: Language, entries: &[(char, usize)]) -> Vec<Shaped> {
        entries
            .iter()
            .map(|&(letter, shape)| {
                let c = crate::forms::lookup(letter, language).unwrap()[shape].unwrap();
                (c, Some((letter, shape)))
            })
            .collect()
    }

    #[test]
    fn lam_alef_collapses_to_one_character() {
        // Shaped "la": lam initial then alef final.
        let input = tagged(
            Language::Arabic,
            &[('\u{0644}', INITIAL), ('\u{0627}', FINAL)],
        );
        assert_eq!(apply(&input), "\u{fefb}");
    }

    #[test]
    fn lam_alef_after_a_joining_letter_takes_the_final_form() {
        let input = tagged(
            Language::Arabic,
            &[('\u{0644}', MEDIAL), ('\u{0627}', FINAL)],
        );
        assert_eq!(apply(&input), "\u{fefc}");
    }

    #[test]
    fn the_word_allah_collapses() {
        let input = tagged(
            Language::Arabic,
            &[
                ('\u{0627}', ISOLATED),
                ('\u{0644}', INITIAL),
                ('\u{0644}', MEDIAL),
                ('\u{0647}', FINAL),
            ],
        );
        assert_eq!(apply(&input), "\u{fdf2}");
    }

    #[test]
    fn an_alef_in_another_position_does_not_collapse() {
        // The same letters, but the alef is final rather than isolated, which is
        // not the word "Allah".
        let input = tagged(
            Language::Arabic,
            &[
                ('\u{0627}', FINAL),
                ('\u{0644}', INITIAL),
                ('\u{0644}', MEDIAL),
                ('\u{0647}', FINAL),
            ],
        );
        assert_eq!(apply(&input), "\u{fe8e}\u{fedf}\u{fee0}\u{feea}");
    }

    #[test]
    fn text_without_ligatures_is_unchanged() {
        assert_eq!(apply(&[('h', None), ('i', None)]), "hi");
        assert_eq!(apply(&[]), "");
    }

    #[test]
    fn a_mark_inside_a_ligature_does_not_stop_it() {
        // Shaped "Allah" with a shadda and a superscript alef between the second
        // lam and the heh, which is how the word is normally written.
        let mut input = tagged(
            Language::Arabic,
            &[
                ('\u{0627}', ISOLATED),
                ('\u{0644}', INITIAL),
                ('\u{0644}', MEDIAL),
            ],
        );
        input.push(('\u{651}', None));
        input.push(('\u{670}', None));
        input.push(('\u{feea}', Some(('\u{0647}', FINAL))));
        assert_eq!(apply(&input), "\u{fdf2}\u{651}\u{670}");
    }

    #[test]
    fn a_mark_inside_lam_alef_does_not_stop_it() {
        // Lam, fatha, alef: the ligature forms and the mark follows it.
        let mut input = tagged(Language::Arabic, &[('\u{0644}', INITIAL)]);
        input.push(('\u{64e}', None));
        input.push(('\u{fe8e}', Some(('\u{0627}', FINAL))));
        assert_eq!(apply(&input), "\u{fefb}\u{64e}");
    }

    #[test]
    fn a_ligature_at_the_very_end_is_found() {
        let mut input = vec![('x', None)];
        input.extend(tagged(
            Language::Arabic,
            &[('\u{0644}', INITIAL), ('\u{0627}', FINAL)],
        ));
        assert_eq!(apply(&input), "x\u{fefb}");
    }

    #[test]
    fn a_mark_after_a_ligature_stays_after_it() {
        let mut input = tagged(
            Language::Arabic,
            &[('\u{0644}', INITIAL), ('\u{0627}', FINAL)],
        );
        input.push(('\u{64e}', None));
        assert_eq!(apply(&input), "\u{fefb}\u{64e}");
    }

    #[test]
    fn the_word_allah_collapses_under_v2_too() {
        // Under ArabicV2 a standalone alef keeps its base form; matching on the
        // shape is what still finds the ligature there.
        let input = tagged(
            Language::ArabicV2,
            &[
                ('\u{0627}', ISOLATED),
                ('\u{0644}', INITIAL),
                ('\u{0644}', MEDIAL),
                ('\u{0647}', FINAL),
            ],
        );
        assert_eq!(apply(&input), "\u{fdf2}");
    }

    #[test]
    fn the_word_allah_collapses_under_kurdish_with_its_own_heh() {
        // Kurdish gives heh the same character for every shape, so only the
        // shape tag distinguishes the final heh the pattern needs.
        let input = tagged(
            Language::Kurdish,
            &[
                ('\u{0627}', ISOLATED),
                ('\u{0644}', INITIAL),
                ('\u{0644}', MEDIAL),
                ('\u{0647}', FINAL),
            ],
        );
        assert_eq!(apply(&input), "\u{fdf2}");
    }
}
