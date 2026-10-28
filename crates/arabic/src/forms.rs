//! The Arabic presentation forms.
//!
//! Each entry maps a letter to its isolated, initial, medial and final shapes.
//! A letter that cannot join on a side has no shape for that side, which is
//! exactly how the joining behaviour is recorded: a letter with no initial form
//! cannot join to the following letter, and one with no final form cannot join
//! to the preceding letter.
//!
//! There are three tables, matching the three `language` values the MIT-licensed
//! `arabic-reshaper` library offers. `Arabic` is the default, used for Arabic,
//! Urdu and Farsi. `ArabicV2` and `Kurdish` leave most letters unshaped when
//! they stand alone, which is what makes them render in fonts that lack the
//! presentation forms; they differ from each other only in how they shape a few
//! letters. The tables are generated from the reference by
//! `research/tools/gen_arabic_tables.py` and sit between the markers below, so a
//! regeneration replaces them without touching the code around them.
//!
//! The presentation forms themselves are defined by the Unicode standard's
//! Arabic Presentation Forms-A and -B blocks; the tables are an independent
//! transcription of them.

/// The four contextual shapes of a letter, in the order isolated, initial,
/// medial, final.
pub(crate) type Shapes = [Option<char>; 4];

/// The index of each shape inside a [`Shapes`] array.
pub(crate) const ISOLATED: usize = 0;
/// Index of the initial shape.
pub(crate) const INITIAL: usize = 1;
/// Index of the medial shape.
pub(crate) const MEDIAL: usize = 2;
/// Index of the final shape.
pub(crate) const FINAL: usize = 3;

// BEGIN GENERATED TABLES
const LETTERS_ARABIC: [(char, Shapes); 78] = [
    ('\u{621}', [Some('\u{fe80}'), None, None, None]),
    ('\u{622}', [Some('\u{fe81}'), None, None, Some('\u{fe82}')]),
    ('\u{623}', [Some('\u{fe83}'), None, None, Some('\u{fe84}')]),
    ('\u{624}', [Some('\u{fe85}'), None, None, Some('\u{fe86}')]),
    ('\u{625}', [Some('\u{fe87}'), None, None, Some('\u{fe88}')]),
    (
        '\u{626}',
        [
            Some('\u{fe89}'),
            Some('\u{fe8b}'),
            Some('\u{fe8c}'),
            Some('\u{fe8a}'),
        ],
    ),
    ('\u{627}', [Some('\u{fe8d}'), None, None, Some('\u{fe8e}')]),
    (
        '\u{628}',
        [
            Some('\u{fe8f}'),
            Some('\u{fe91}'),
            Some('\u{fe92}'),
            Some('\u{fe90}'),
        ],
    ),
    ('\u{629}', [Some('\u{fe93}'), None, None, Some('\u{fe94}')]),
    (
        '\u{62a}',
        [
            Some('\u{fe95}'),
            Some('\u{fe97}'),
            Some('\u{fe98}'),
            Some('\u{fe96}'),
        ],
    ),
    (
        '\u{62b}',
        [
            Some('\u{fe99}'),
            Some('\u{fe9b}'),
            Some('\u{fe9c}'),
            Some('\u{fe9a}'),
        ],
    ),
    (
        '\u{62c}',
        [
            Some('\u{fe9d}'),
            Some('\u{fe9f}'),
            Some('\u{fea0}'),
            Some('\u{fe9e}'),
        ],
    ),
    (
        '\u{62d}',
        [
            Some('\u{fea1}'),
            Some('\u{fea3}'),
            Some('\u{fea4}'),
            Some('\u{fea2}'),
        ],
    ),
    (
        '\u{62e}',
        [
            Some('\u{fea5}'),
            Some('\u{fea7}'),
            Some('\u{fea8}'),
            Some('\u{fea6}'),
        ],
    ),
    ('\u{62f}', [Some('\u{fea9}'), None, None, Some('\u{feaa}')]),
    ('\u{630}', [Some('\u{feab}'), None, None, Some('\u{feac}')]),
    ('\u{631}', [Some('\u{fead}'), None, None, Some('\u{feae}')]),
    ('\u{632}', [Some('\u{feaf}'), None, None, Some('\u{feb0}')]),
    (
        '\u{633}',
        [
            Some('\u{feb1}'),
            Some('\u{feb3}'),
            Some('\u{feb4}'),
            Some('\u{feb2}'),
        ],
    ),
    (
        '\u{634}',
        [
            Some('\u{feb5}'),
            Some('\u{feb7}'),
            Some('\u{feb8}'),
            Some('\u{feb6}'),
        ],
    ),
    (
        '\u{635}',
        [
            Some('\u{feb9}'),
            Some('\u{febb}'),
            Some('\u{febc}'),
            Some('\u{feba}'),
        ],
    ),
    (
        '\u{636}',
        [
            Some('\u{febd}'),
            Some('\u{febf}'),
            Some('\u{fec0}'),
            Some('\u{febe}'),
        ],
    ),
    (
        '\u{637}',
        [
            Some('\u{fec1}'),
            Some('\u{fec3}'),
            Some('\u{fec4}'),
            Some('\u{fec2}'),
        ],
    ),
    (
        '\u{638}',
        [
            Some('\u{fec5}'),
            Some('\u{fec7}'),
            Some('\u{fec8}'),
            Some('\u{fec6}'),
        ],
    ),
    (
        '\u{639}',
        [
            Some('\u{fec9}'),
            Some('\u{fecb}'),
            Some('\u{fecc}'),
            Some('\u{feca}'),
        ],
    ),
    (
        '\u{63a}',
        [
            Some('\u{fecd}'),
            Some('\u{fecf}'),
            Some('\u{fed0}'),
            Some('\u{fece}'),
        ],
    ),
    (
        '\u{640}',
        [
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
        ],
    ),
    (
        '\u{641}',
        [
            Some('\u{fed1}'),
            Some('\u{fed3}'),
            Some('\u{fed4}'),
            Some('\u{fed2}'),
        ],
    ),
    (
        '\u{642}',
        [
            Some('\u{fed5}'),
            Some('\u{fed7}'),
            Some('\u{fed8}'),
            Some('\u{fed6}'),
        ],
    ),
    (
        '\u{643}',
        [
            Some('\u{fed9}'),
            Some('\u{fedb}'),
            Some('\u{fedc}'),
            Some('\u{feda}'),
        ],
    ),
    (
        '\u{644}',
        [
            Some('\u{fedd}'),
            Some('\u{fedf}'),
            Some('\u{fee0}'),
            Some('\u{fede}'),
        ],
    ),
    (
        '\u{645}',
        [
            Some('\u{fee1}'),
            Some('\u{fee3}'),
            Some('\u{fee4}'),
            Some('\u{fee2}'),
        ],
    ),
    (
        '\u{646}',
        [
            Some('\u{fee5}'),
            Some('\u{fee7}'),
            Some('\u{fee8}'),
            Some('\u{fee6}'),
        ],
    ),
    (
        '\u{647}',
        [
            Some('\u{fee9}'),
            Some('\u{feeb}'),
            Some('\u{feec}'),
            Some('\u{feea}'),
        ],
    ),
    ('\u{648}', [Some('\u{feed}'), None, None, Some('\u{feee}')]),
    (
        '\u{649}',
        [
            Some('\u{feef}'),
            Some('\u{fbe8}'),
            Some('\u{fbe9}'),
            Some('\u{fef0}'),
        ],
    ),
    (
        '\u{64a}',
        [
            Some('\u{fef1}'),
            Some('\u{fef3}'),
            Some('\u{fef4}'),
            Some('\u{fef2}'),
        ],
    ),
    ('\u{671}', [Some('\u{fb50}'), None, None, Some('\u{fb51}')]),
    ('\u{677}', [Some('\u{fbdd}'), None, None, None]),
    (
        '\u{679}',
        [
            Some('\u{fb66}'),
            Some('\u{fb68}'),
            Some('\u{fb69}'),
            Some('\u{fb67}'),
        ],
    ),
    (
        '\u{67a}',
        [
            Some('\u{fb5e}'),
            Some('\u{fb60}'),
            Some('\u{fb61}'),
            Some('\u{fb5f}'),
        ],
    ),
    (
        '\u{67b}',
        [
            Some('\u{fb52}'),
            Some('\u{fb54}'),
            Some('\u{fb55}'),
            Some('\u{fb53}'),
        ],
    ),
    (
        '\u{67e}',
        [
            Some('\u{fb56}'),
            Some('\u{fb58}'),
            Some('\u{fb59}'),
            Some('\u{fb57}'),
        ],
    ),
    (
        '\u{67f}',
        [
            Some('\u{fb62}'),
            Some('\u{fb64}'),
            Some('\u{fb65}'),
            Some('\u{fb63}'),
        ],
    ),
    (
        '\u{680}',
        [
            Some('\u{fb5a}'),
            Some('\u{fb5c}'),
            Some('\u{fb5d}'),
            Some('\u{fb5b}'),
        ],
    ),
    (
        '\u{683}',
        [
            Some('\u{fb76}'),
            Some('\u{fb78}'),
            Some('\u{fb79}'),
            Some('\u{fb77}'),
        ],
    ),
    (
        '\u{684}',
        [
            Some('\u{fb72}'),
            Some('\u{fb74}'),
            Some('\u{fb75}'),
            Some('\u{fb73}'),
        ],
    ),
    (
        '\u{686}',
        [
            Some('\u{fb7a}'),
            Some('\u{fb7c}'),
            Some('\u{fb7d}'),
            Some('\u{fb7b}'),
        ],
    ),
    (
        '\u{687}',
        [
            Some('\u{fb7e}'),
            Some('\u{fb80}'),
            Some('\u{fb81}'),
            Some('\u{fb7f}'),
        ],
    ),
    ('\u{688}', [Some('\u{fb88}'), None, None, Some('\u{fb89}')]),
    ('\u{68c}', [Some('\u{fb84}'), None, None, Some('\u{fb85}')]),
    ('\u{68d}', [Some('\u{fb82}'), None, None, Some('\u{fb83}')]),
    ('\u{68e}', [Some('\u{fb86}'), None, None, Some('\u{fb87}')]),
    ('\u{691}', [Some('\u{fb8c}'), None, None, Some('\u{fb8d}')]),
    ('\u{698}', [Some('\u{fb8a}'), None, None, Some('\u{fb8b}')]),
    (
        '\u{6a4}',
        [
            Some('\u{fb6a}'),
            Some('\u{fb6c}'),
            Some('\u{fb6d}'),
            Some('\u{fb6b}'),
        ],
    ),
    (
        '\u{6a6}',
        [
            Some('\u{fb6e}'),
            Some('\u{fb70}'),
            Some('\u{fb71}'),
            Some('\u{fb6f}'),
        ],
    ),
    (
        '\u{6a9}',
        [
            Some('\u{fb8e}'),
            Some('\u{fb90}'),
            Some('\u{fb91}'),
            Some('\u{fb8f}'),
        ],
    ),
    (
        '\u{6ad}',
        [
            Some('\u{fbd3}'),
            Some('\u{fbd5}'),
            Some('\u{fbd6}'),
            Some('\u{fbd4}'),
        ],
    ),
    (
        '\u{6af}',
        [
            Some('\u{fb92}'),
            Some('\u{fb94}'),
            Some('\u{fb95}'),
            Some('\u{fb93}'),
        ],
    ),
    (
        '\u{6b1}',
        [
            Some('\u{fb9a}'),
            Some('\u{fb9c}'),
            Some('\u{fb9d}'),
            Some('\u{fb9b}'),
        ],
    ),
    (
        '\u{6b3}',
        [
            Some('\u{fb96}'),
            Some('\u{fb98}'),
            Some('\u{fb99}'),
            Some('\u{fb97}'),
        ],
    ),
    ('\u{6ba}', [Some('\u{fb9e}'), None, None, Some('\u{fb9f}')]),
    (
        '\u{6bb}',
        [
            Some('\u{fba0}'),
            Some('\u{fba2}'),
            Some('\u{fba3}'),
            Some('\u{fba1}'),
        ],
    ),
    (
        '\u{6be}',
        [
            Some('\u{fbaa}'),
            Some('\u{fbac}'),
            Some('\u{fbad}'),
            Some('\u{fbab}'),
        ],
    ),
    ('\u{6c0}', [Some('\u{fba4}'), None, None, Some('\u{fba5}')]),
    (
        '\u{6c1}',
        [
            Some('\u{fba6}'),
            Some('\u{fba8}'),
            Some('\u{fba9}'),
            Some('\u{fba7}'),
        ],
    ),
    ('\u{6c5}', [Some('\u{fbe0}'), None, None, Some('\u{fbe1}')]),
    ('\u{6c6}', [Some('\u{fbd9}'), None, None, Some('\u{fbda}')]),
    ('\u{6c7}', [Some('\u{fbd7}'), None, None, Some('\u{fbd8}')]),
    ('\u{6c8}', [Some('\u{fbdb}'), None, None, Some('\u{fbdc}')]),
    ('\u{6c9}', [Some('\u{fbe2}'), None, None, Some('\u{fbe3}')]),
    ('\u{6cb}', [Some('\u{fbde}'), None, None, Some('\u{fbdf}')]),
    (
        '\u{6cc}',
        [
            Some('\u{fbfc}'),
            Some('\u{fbfe}'),
            Some('\u{fbff}'),
            Some('\u{fbfd}'),
        ],
    ),
    (
        '\u{6d0}',
        [
            Some('\u{fbe4}'),
            Some('\u{fbe6}'),
            Some('\u{fbe7}'),
            Some('\u{fbe5}'),
        ],
    ),
    ('\u{6d2}', [Some('\u{fbae}'), None, None, Some('\u{fbaf}')]),
    ('\u{6d3}', [Some('\u{fbb0}'), None, None, Some('\u{fbb1}')]),
    (
        '\u{200d}',
        [
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
        ],
    ),
];
const LETTERS_ARABIC_V2: [(char, Shapes); 80] = [
    ('\u{621}', [Some('\u{fe80}'), None, None, None]),
    ('\u{622}', [Some('\u{622}'), None, None, Some('\u{fe82}')]),
    ('\u{623}', [Some('\u{623}'), None, None, Some('\u{fe84}')]),
    ('\u{624}', [Some('\u{624}'), None, None, Some('\u{fe86}')]),
    ('\u{625}', [Some('\u{625}'), None, None, Some('\u{fe88}')]),
    (
        '\u{626}',
        [
            Some('\u{626}'),
            Some('\u{fe8b}'),
            Some('\u{fe8c}'),
            Some('\u{fe8a}'),
        ],
    ),
    ('\u{627}', [Some('\u{627}'), None, None, Some('\u{fe8e}')]),
    (
        '\u{628}',
        [
            Some('\u{628}'),
            Some('\u{fe91}'),
            Some('\u{fe92}'),
            Some('\u{fe90}'),
        ],
    ),
    ('\u{629}', [Some('\u{629}'), None, None, Some('\u{fe94}')]),
    (
        '\u{62a}',
        [
            Some('\u{62a}'),
            Some('\u{fe97}'),
            Some('\u{fe98}'),
            Some('\u{fe96}'),
        ],
    ),
    (
        '\u{62b}',
        [
            Some('\u{62b}'),
            Some('\u{fe9b}'),
            Some('\u{fe9c}'),
            Some('\u{fe9a}'),
        ],
    ),
    (
        '\u{62c}',
        [
            Some('\u{62c}'),
            Some('\u{fe9f}'),
            Some('\u{fea0}'),
            Some('\u{fe9e}'),
        ],
    ),
    (
        '\u{62d}',
        [
            Some('\u{fea1}'),
            Some('\u{fea3}'),
            Some('\u{fea4}'),
            Some('\u{fea2}'),
        ],
    ),
    (
        '\u{62e}',
        [
            Some('\u{62e}'),
            Some('\u{fea7}'),
            Some('\u{fea8}'),
            Some('\u{fea6}'),
        ],
    ),
    ('\u{62f}', [Some('\u{62f}'), None, None, Some('\u{feaa}')]),
    ('\u{630}', [Some('\u{630}'), None, None, Some('\u{feac}')]),
    ('\u{631}', [Some('\u{631}'), None, None, Some('\u{feae}')]),
    ('\u{632}', [Some('\u{632}'), None, None, Some('\u{feb0}')]),
    (
        '\u{633}',
        [
            Some('\u{633}'),
            Some('\u{feb3}'),
            Some('\u{feb4}'),
            Some('\u{feb2}'),
        ],
    ),
    (
        '\u{634}',
        [
            Some('\u{634}'),
            Some('\u{feb7}'),
            Some('\u{feb8}'),
            Some('\u{feb6}'),
        ],
    ),
    (
        '\u{635}',
        [
            Some('\u{635}'),
            Some('\u{febb}'),
            Some('\u{febc}'),
            Some('\u{feba}'),
        ],
    ),
    (
        '\u{636}',
        [
            Some('\u{636}'),
            Some('\u{febf}'),
            Some('\u{fec0}'),
            Some('\u{febe}'),
        ],
    ),
    (
        '\u{637}',
        [
            Some('\u{637}'),
            Some('\u{fec3}'),
            Some('\u{fec4}'),
            Some('\u{fec2}'),
        ],
    ),
    (
        '\u{638}',
        [
            Some('\u{638}'),
            Some('\u{fec7}'),
            Some('\u{fec8}'),
            Some('\u{fec6}'),
        ],
    ),
    (
        '\u{639}',
        [
            Some('\u{639}'),
            Some('\u{fecb}'),
            Some('\u{fecc}'),
            Some('\u{feca}'),
        ],
    ),
    (
        '\u{63a}',
        [
            Some('\u{63a}'),
            Some('\u{fecf}'),
            Some('\u{fed0}'),
            Some('\u{fece}'),
        ],
    ),
    (
        '\u{640}',
        [
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
        ],
    ),
    (
        '\u{641}',
        [
            Some('\u{641}'),
            Some('\u{fed3}'),
            Some('\u{fed4}'),
            Some('\u{fed2}'),
        ],
    ),
    (
        '\u{642}',
        [
            Some('\u{642}'),
            Some('\u{fed7}'),
            Some('\u{fed8}'),
            Some('\u{fed6}'),
        ],
    ),
    (
        '\u{643}',
        [
            Some('\u{643}'),
            Some('\u{fedb}'),
            Some('\u{fedc}'),
            Some('\u{feda}'),
        ],
    ),
    (
        '\u{644}',
        [
            Some('\u{644}'),
            Some('\u{fedf}'),
            Some('\u{fee0}'),
            Some('\u{fede}'),
        ],
    ),
    (
        '\u{645}',
        [
            Some('\u{645}'),
            Some('\u{fee3}'),
            Some('\u{fee4}'),
            Some('\u{fee2}'),
        ],
    ),
    (
        '\u{646}',
        [
            Some('\u{646}'),
            Some('\u{fee7}'),
            Some('\u{fee8}'),
            Some('\u{fee6}'),
        ],
    ),
    (
        '\u{647}',
        [
            Some('\u{647}'),
            Some('\u{feeb}'),
            Some('\u{feec}'),
            Some('\u{feea}'),
        ],
    ),
    ('\u{648}', [Some('\u{648}'), None, None, Some('\u{feee}')]),
    (
        '\u{649}',
        [
            Some('\u{649}'),
            Some('\u{fbe8}'),
            Some('\u{fbe9}'),
            Some('\u{fef0}'),
        ],
    ),
    (
        '\u{64a}',
        [
            Some('\u{64a}'),
            Some('\u{fef3}'),
            Some('\u{fef4}'),
            Some('\u{fef2}'),
        ],
    ),
    ('\u{671}', [Some('\u{671}'), None, None, Some('\u{fb51}')]),
    ('\u{677}', [Some('\u{677}'), None, None, None]),
    (
        '\u{679}',
        [
            Some('\u{679}'),
            Some('\u{fb68}'),
            Some('\u{fb69}'),
            Some('\u{fb67}'),
        ],
    ),
    (
        '\u{67a}',
        [
            Some('\u{67a}'),
            Some('\u{fb60}'),
            Some('\u{fb61}'),
            Some('\u{fb5f}'),
        ],
    ),
    (
        '\u{67b}',
        [
            Some('\u{67b}'),
            Some('\u{fb54}'),
            Some('\u{fb55}'),
            Some('\u{fb53}'),
        ],
    ),
    (
        '\u{67e}',
        [
            Some('\u{67e}'),
            Some('\u{fb58}'),
            Some('\u{fb59}'),
            Some('\u{fb57}'),
        ],
    ),
    (
        '\u{67f}',
        [
            Some('\u{67f}'),
            Some('\u{fb64}'),
            Some('\u{fb65}'),
            Some('\u{fb63}'),
        ],
    ),
    (
        '\u{680}',
        [
            Some('\u{680}'),
            Some('\u{fb5c}'),
            Some('\u{fb5d}'),
            Some('\u{fb5b}'),
        ],
    ),
    (
        '\u{683}',
        [
            Some('\u{683}'),
            Some('\u{fb78}'),
            Some('\u{fb79}'),
            Some('\u{fb77}'),
        ],
    ),
    (
        '\u{684}',
        [
            Some('\u{684}'),
            Some('\u{fb74}'),
            Some('\u{fb75}'),
            Some('\u{fb73}'),
        ],
    ),
    (
        '\u{686}',
        [
            Some('\u{686}'),
            Some('\u{fb7c}'),
            Some('\u{fb7d}'),
            Some('\u{fb7b}'),
        ],
    ),
    (
        '\u{687}',
        [
            Some('\u{687}'),
            Some('\u{fb80}'),
            Some('\u{fb81}'),
            Some('\u{fb7f}'),
        ],
    ),
    ('\u{688}', [Some('\u{688}'), None, None, Some('\u{fb89}')]),
    ('\u{68c}', [Some('\u{68c}'), None, None, Some('\u{fb85}')]),
    ('\u{68d}', [Some('\u{68d}'), None, None, Some('\u{fb83}')]),
    ('\u{68e}', [Some('\u{68e}'), None, None, Some('\u{fb87}')]),
    ('\u{691}', [Some('\u{691}'), None, None, Some('\u{fb8d}')]),
    ('\u{698}', [Some('\u{698}'), None, None, Some('\u{fb8b}')]),
    (
        '\u{6a4}',
        [
            Some('\u{6a4}'),
            Some('\u{fb6c}'),
            Some('\u{fb6d}'),
            Some('\u{fb6b}'),
        ],
    ),
    (
        '\u{6a6}',
        [
            Some('\u{6a6}'),
            Some('\u{fb70}'),
            Some('\u{fb71}'),
            Some('\u{fb6f}'),
        ],
    ),
    (
        '\u{6a9}',
        [
            Some('\u{6a9}'),
            Some('\u{fb90}'),
            Some('\u{fb91}'),
            Some('\u{fb8f}'),
        ],
    ),
    (
        '\u{6ad}',
        [
            Some('\u{6ad}'),
            Some('\u{fbd5}'),
            Some('\u{fbd6}'),
            Some('\u{fbd4}'),
        ],
    ),
    (
        '\u{6af}',
        [
            Some('\u{6af}'),
            Some('\u{fb94}'),
            Some('\u{fb95}'),
            Some('\u{fb93}'),
        ],
    ),
    (
        '\u{6b1}',
        [
            Some('\u{6b1}'),
            Some('\u{fb9c}'),
            Some('\u{fb9d}'),
            Some('\u{fb9b}'),
        ],
    ),
    (
        '\u{6b3}',
        [
            Some('\u{6b3}'),
            Some('\u{fb98}'),
            Some('\u{fb99}'),
            Some('\u{fb97}'),
        ],
    ),
    ('\u{6ba}', [Some('\u{6ba}'), None, None, Some('\u{fb9f}')]),
    (
        '\u{6bb}',
        [
            Some('\u{6bb}'),
            Some('\u{fba2}'),
            Some('\u{fba3}'),
            Some('\u{fba1}'),
        ],
    ),
    (
        '\u{6be}',
        [
            Some('\u{6be}'),
            Some('\u{fbac}'),
            Some('\u{fbad}'),
            Some('\u{fbab}'),
        ],
    ),
    ('\u{6c0}', [Some('\u{6c0}'), None, None, Some('\u{fba5}')]),
    (
        '\u{6c1}',
        [
            Some('\u{6c1}'),
            Some('\u{fba8}'),
            Some('\u{fba9}'),
            Some('\u{fba7}'),
        ],
    ),
    ('\u{6c5}', [Some('\u{6c5}'), None, None, Some('\u{fbe1}')]),
    ('\u{6c6}', [Some('\u{6c6}'), None, None, Some('\u{fbda}')]),
    ('\u{6c7}', [Some('\u{6c7}'), None, None, Some('\u{fbd8}')]),
    ('\u{6c8}', [Some('\u{6c8}'), None, None, Some('\u{fbdc}')]),
    ('\u{6c9}', [Some('\u{6c9}'), None, None, Some('\u{fbe3}')]),
    ('\u{6cb}', [Some('\u{6cb}'), None, None, Some('\u{fbdf}')]),
    (
        '\u{6cc}',
        [
            Some('\u{6cc}'),
            Some('\u{fbfe}'),
            Some('\u{fbff}'),
            Some('\u{fbfd}'),
        ],
    ),
    (
        '\u{6ce}',
        [
            Some('\u{e004}'),
            Some('\u{e005}'),
            Some('\u{e006}'),
            Some('\u{e004}'),
        ],
    ),
    (
        '\u{6d0}',
        [
            Some('\u{6d0}'),
            Some('\u{fbe6}'),
            Some('\u{fbe7}'),
            Some('\u{fbe5}'),
        ],
    ),
    ('\u{6d2}', [Some('\u{6d2}'), None, None, Some('\u{fbaf}')]),
    ('\u{6d3}', [Some('\u{6d3}'), None, None, Some('\u{fbb1}')]),
    ('\u{6d5}', [Some('\u{6d5}'), None, None, Some('\u{e000}')]),
    (
        '\u{200d}',
        [
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
        ],
    ),
];
const LETTERS_KURDISH: [(char, Shapes); 80] = [
    ('\u{621}', [Some('\u{fe80}'), None, None, None]),
    ('\u{622}', [Some('\u{622}'), None, None, Some('\u{fe82}')]),
    ('\u{623}', [Some('\u{623}'), None, None, Some('\u{fe84}')]),
    ('\u{624}', [Some('\u{624}'), None, None, Some('\u{fe86}')]),
    ('\u{625}', [Some('\u{625}'), None, None, Some('\u{fe88}')]),
    (
        '\u{626}',
        [
            Some('\u{626}'),
            Some('\u{fe8b}'),
            Some('\u{fe8c}'),
            Some('\u{fe8a}'),
        ],
    ),
    ('\u{627}', [Some('\u{627}'), None, None, Some('\u{fe8e}')]),
    (
        '\u{628}',
        [
            Some('\u{628}'),
            Some('\u{fe91}'),
            Some('\u{fe92}'),
            Some('\u{fe90}'),
        ],
    ),
    ('\u{629}', [Some('\u{629}'), None, None, Some('\u{fe94}')]),
    (
        '\u{62a}',
        [
            Some('\u{62a}'),
            Some('\u{fe97}'),
            Some('\u{fe98}'),
            Some('\u{fe96}'),
        ],
    ),
    (
        '\u{62b}',
        [
            Some('\u{62b}'),
            Some('\u{fe9b}'),
            Some('\u{fe9c}'),
            Some('\u{fe9a}'),
        ],
    ),
    (
        '\u{62c}',
        [
            Some('\u{62c}'),
            Some('\u{fe9f}'),
            Some('\u{fea0}'),
            Some('\u{fe9e}'),
        ],
    ),
    (
        '\u{62d}',
        [
            Some('\u{fea1}'),
            Some('\u{fea3}'),
            Some('\u{fea4}'),
            Some('\u{fea2}'),
        ],
    ),
    (
        '\u{62e}',
        [
            Some('\u{62e}'),
            Some('\u{fea7}'),
            Some('\u{fea8}'),
            Some('\u{fea6}'),
        ],
    ),
    ('\u{62f}', [Some('\u{62f}'), None, None, Some('\u{feaa}')]),
    ('\u{630}', [Some('\u{630}'), None, None, Some('\u{feac}')]),
    ('\u{631}', [Some('\u{631}'), None, None, Some('\u{feae}')]),
    ('\u{632}', [Some('\u{632}'), None, None, Some('\u{feb0}')]),
    (
        '\u{633}',
        [
            Some('\u{633}'),
            Some('\u{feb3}'),
            Some('\u{feb4}'),
            Some('\u{feb2}'),
        ],
    ),
    (
        '\u{634}',
        [
            Some('\u{634}'),
            Some('\u{feb7}'),
            Some('\u{feb8}'),
            Some('\u{feb6}'),
        ],
    ),
    (
        '\u{635}',
        [
            Some('\u{635}'),
            Some('\u{febb}'),
            Some('\u{febc}'),
            Some('\u{feba}'),
        ],
    ),
    (
        '\u{636}',
        [
            Some('\u{636}'),
            Some('\u{febf}'),
            Some('\u{fec0}'),
            Some('\u{febe}'),
        ],
    ),
    (
        '\u{637}',
        [
            Some('\u{637}'),
            Some('\u{fec3}'),
            Some('\u{fec4}'),
            Some('\u{fec2}'),
        ],
    ),
    (
        '\u{638}',
        [
            Some('\u{638}'),
            Some('\u{fec7}'),
            Some('\u{fec8}'),
            Some('\u{fec6}'),
        ],
    ),
    (
        '\u{639}',
        [
            Some('\u{639}'),
            Some('\u{fecb}'),
            Some('\u{fecc}'),
            Some('\u{feca}'),
        ],
    ),
    (
        '\u{63a}',
        [
            Some('\u{63a}'),
            Some('\u{fecf}'),
            Some('\u{fed0}'),
            Some('\u{fece}'),
        ],
    ),
    (
        '\u{640}',
        [
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
            Some('\u{640}'),
        ],
    ),
    (
        '\u{641}',
        [
            Some('\u{641}'),
            Some('\u{fed3}'),
            Some('\u{fed4}'),
            Some('\u{fed2}'),
        ],
    ),
    (
        '\u{642}',
        [
            Some('\u{642}'),
            Some('\u{fed7}'),
            Some('\u{fed8}'),
            Some('\u{fed6}'),
        ],
    ),
    (
        '\u{643}',
        [
            Some('\u{643}'),
            Some('\u{fedb}'),
            Some('\u{fedc}'),
            Some('\u{feda}'),
        ],
    ),
    (
        '\u{644}',
        [
            Some('\u{644}'),
            Some('\u{fedf}'),
            Some('\u{fee0}'),
            Some('\u{fede}'),
        ],
    ),
    (
        '\u{645}',
        [
            Some('\u{645}'),
            Some('\u{fee3}'),
            Some('\u{fee4}'),
            Some('\u{fee2}'),
        ],
    ),
    (
        '\u{646}',
        [
            Some('\u{646}'),
            Some('\u{fee7}'),
            Some('\u{fee8}'),
            Some('\u{fee6}'),
        ],
    ),
    (
        '\u{647}',
        [
            Some('\u{fbab}'),
            Some('\u{fbab}'),
            Some('\u{fbab}'),
            Some('\u{fbab}'),
        ],
    ),
    ('\u{648}', [Some('\u{648}'), None, None, Some('\u{feee}')]),
    (
        '\u{649}',
        [
            Some('\u{649}'),
            Some('\u{fbe8}'),
            Some('\u{fbe9}'),
            Some('\u{fef0}'),
        ],
    ),
    (
        '\u{64a}',
        [
            Some('\u{64a}'),
            Some('\u{fef3}'),
            Some('\u{fef4}'),
            Some('\u{fef2}'),
        ],
    ),
    ('\u{671}', [Some('\u{671}'), None, None, Some('\u{fb51}')]),
    ('\u{677}', [Some('\u{677}'), None, None, None]),
    (
        '\u{679}',
        [
            Some('\u{679}'),
            Some('\u{fb68}'),
            Some('\u{fb69}'),
            Some('\u{fb67}'),
        ],
    ),
    (
        '\u{67a}',
        [
            Some('\u{67a}'),
            Some('\u{fb60}'),
            Some('\u{fb61}'),
            Some('\u{fb5f}'),
        ],
    ),
    (
        '\u{67b}',
        [
            Some('\u{67b}'),
            Some('\u{fb54}'),
            Some('\u{fb55}'),
            Some('\u{fb53}'),
        ],
    ),
    (
        '\u{67e}',
        [
            Some('\u{67e}'),
            Some('\u{fb58}'),
            Some('\u{fb59}'),
            Some('\u{fb57}'),
        ],
    ),
    (
        '\u{67f}',
        [
            Some('\u{67f}'),
            Some('\u{fb64}'),
            Some('\u{fb65}'),
            Some('\u{fb63}'),
        ],
    ),
    (
        '\u{680}',
        [
            Some('\u{680}'),
            Some('\u{fb5c}'),
            Some('\u{fb5d}'),
            Some('\u{fb5b}'),
        ],
    ),
    (
        '\u{683}',
        [
            Some('\u{683}'),
            Some('\u{fb78}'),
            Some('\u{fb79}'),
            Some('\u{fb77}'),
        ],
    ),
    (
        '\u{684}',
        [
            Some('\u{684}'),
            Some('\u{fb74}'),
            Some('\u{fb75}'),
            Some('\u{fb73}'),
        ],
    ),
    (
        '\u{686}',
        [
            Some('\u{686}'),
            Some('\u{fb7c}'),
            Some('\u{fb7d}'),
            Some('\u{fb7b}'),
        ],
    ),
    (
        '\u{687}',
        [
            Some('\u{687}'),
            Some('\u{fb80}'),
            Some('\u{fb81}'),
            Some('\u{fb7f}'),
        ],
    ),
    ('\u{688}', [Some('\u{688}'), None, None, Some('\u{fb89}')]),
    ('\u{68c}', [Some('\u{68c}'), None, None, Some('\u{fb85}')]),
    ('\u{68d}', [Some('\u{68d}'), None, None, Some('\u{fb83}')]),
    ('\u{68e}', [Some('\u{68e}'), None, None, Some('\u{fb87}')]),
    ('\u{691}', [Some('\u{691}'), None, None, Some('\u{fb8d}')]),
    ('\u{698}', [Some('\u{698}'), None, None, Some('\u{fb8b}')]),
    (
        '\u{6a4}',
        [
            Some('\u{6a4}'),
            Some('\u{fb6c}'),
            Some('\u{fb6d}'),
            Some('\u{fb6b}'),
        ],
    ),
    (
        '\u{6a6}',
        [
            Some('\u{6a6}'),
            Some('\u{fb70}'),
            Some('\u{fb71}'),
            Some('\u{fb6f}'),
        ],
    ),
    (
        '\u{6a9}',
        [
            Some('\u{6a9}'),
            Some('\u{fb90}'),
            Some('\u{fb91}'),
            Some('\u{fb8f}'),
        ],
    ),
    (
        '\u{6ad}',
        [
            Some('\u{6ad}'),
            Some('\u{fbd5}'),
            Some('\u{fbd6}'),
            Some('\u{fbd4}'),
        ],
    ),
    (
        '\u{6af}',
        [
            Some('\u{6af}'),
            Some('\u{fb94}'),
            Some('\u{fb95}'),
            Some('\u{fb93}'),
        ],
    ),
    (
        '\u{6b1}',
        [
            Some('\u{6b1}'),
            Some('\u{fb9c}'),
            Some('\u{fb9d}'),
            Some('\u{fb9b}'),
        ],
    ),
    (
        '\u{6b3}',
        [
            Some('\u{6b3}'),
            Some('\u{fb98}'),
            Some('\u{fb99}'),
            Some('\u{fb97}'),
        ],
    ),
    ('\u{6ba}', [Some('\u{6ba}'), None, None, Some('\u{fb9f}')]),
    (
        '\u{6bb}',
        [
            Some('\u{6bb}'),
            Some('\u{fba2}'),
            Some('\u{fba3}'),
            Some('\u{fba1}'),
        ],
    ),
    (
        '\u{6be}',
        [
            Some('\u{6be}'),
            Some('\u{fbac}'),
            Some('\u{fbad}'),
            Some('\u{fbab}'),
        ],
    ),
    ('\u{6c0}', [Some('\u{6c0}'), None, None, Some('\u{fba5}')]),
    (
        '\u{6c1}',
        [
            Some('\u{6c1}'),
            Some('\u{fba8}'),
            Some('\u{fba9}'),
            Some('\u{fba7}'),
        ],
    ),
    ('\u{6c5}', [Some('\u{6c5}'), None, None, Some('\u{fbe1}')]),
    ('\u{6c6}', [Some('\u{6c6}'), None, None, Some('\u{fbda}')]),
    ('\u{6c7}', [Some('\u{6c7}'), None, None, Some('\u{fbd8}')]),
    ('\u{6c8}', [Some('\u{6c8}'), None, None, Some('\u{fbdc}')]),
    ('\u{6c9}', [Some('\u{6c9}'), None, None, Some('\u{fbe3}')]),
    ('\u{6cb}', [Some('\u{6cb}'), None, None, Some('\u{fbdf}')]),
    (
        '\u{6cc}',
        [
            Some('\u{6cc}'),
            Some('\u{fbfe}'),
            Some('\u{fbff}'),
            Some('\u{fbfd}'),
        ],
    ),
    (
        '\u{6ce}',
        [
            Some('\u{e004}'),
            Some('\u{e005}'),
            Some('\u{e006}'),
            Some('\u{e004}'),
        ],
    ),
    (
        '\u{6d0}',
        [
            Some('\u{6d0}'),
            Some('\u{fbe6}'),
            Some('\u{fbe7}'),
            Some('\u{fbe5}'),
        ],
    ),
    ('\u{6d2}', [Some('\u{6d2}'), None, None, Some('\u{fbaf}')]),
    ('\u{6d3}', [Some('\u{6d3}'), None, None, Some('\u{fbb1}')]),
    ('\u{6d5}', [Some('\u{6d5}'), None, None, Some('\u{e000}')]),
    (
        '\u{200d}',
        [
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
        ],
    ),
];
// END GENERATED TABLES

/// The table a language shapes with.
fn table(language: Language) -> &'static [(char, Shapes)] {
    match language {
        Language::Arabic => &LETTERS_ARABIC,
        Language::ArabicV2 => &LETTERS_ARABIC_V2,
        Language::Kurdish => &LETTERS_KURDISH,
    }
}

/// The shapes of a letter, or `None` if the selected table does not list it.
pub(crate) fn lookup(c: char, language: Language) -> Option<Shapes> {
    let entries = table(language);
    entries
        .binary_search_by_key(&c, |(letter, _)| *letter)
        .ok()
        .map(|index| entries[index].1)
}

/// Whether `c` can join to the letter before it, which needs a final or medial
/// shape.
pub(crate) fn joins_before(c: char, language: Language) -> bool {
    lookup(c, language).is_some_and(|s| s[FINAL].is_some() || s[MEDIAL].is_some())
}

/// Whether `c` can join to the letter after it, which needs an initial or
/// medial shape.
pub(crate) fn joins_after(c: char, language: Language) -> bool {
    lookup(c, language).is_some_and(|s| s[INITIAL].is_some() || s[MEDIAL].is_some())
}

/// Whether any of the three tables lists `c`.
///
/// The tables overlap almost completely, and a letter absent from the default
/// one may still be shapeable when another language is selected, so the union is
/// the honest answer for a check that carries no language.
pub(crate) fn is_letter(c: char) -> bool {
    [Language::Arabic, Language::ArabicV2, Language::Kurdish]
        .into_iter()
        .any(|language| lookup(c, language).is_some())
}

use crate::Language;

#[cfg(test)]
mod tests {
    use super::*;

    /// The three tables paired with the language that selects them.
    const ALL: [(Language, &[(char, Shapes)]); 3] = [
        (Language::Arabic, &LETTERS_ARABIC),
        (Language::ArabicV2, &LETTERS_ARABIC_V2),
        (Language::Kurdish, &LETTERS_KURDISH),
    ];

    #[test]
    fn every_table_is_sorted_so_binary_search_works() {
        for (language, entries) in ALL {
            let mut previous = '\0';
            for (letter, _) in entries {
                assert!(
                    *letter > previous,
                    "{language:?} is out of order at {letter:?}"
                );
                previous = *letter;
            }
        }
    }

    #[test]
    fn alef_joins_only_to_the_letter_before_it() {
        // Alef never joins to what follows, which is why it has no initial or
        // medial shape, in every table.
        for (language, _) in ALL {
            assert!(joins_before('\u{0627}', language));
            assert!(!joins_after('\u{0627}', language));
        }
    }

    #[test]
    fn beh_joins_on_both_sides() {
        for (language, _) in ALL {
            assert!(joins_before('\u{0628}', language));
            assert!(joins_after('\u{0628}', language));
        }
    }

    #[test]
    fn hamza_joins_on_neither_side() {
        for (language, _) in ALL {
            assert!(!joins_before('\u{0621}', language));
            assert!(!joins_after('\u{0621}', language));
        }
    }

    #[test]
    fn every_entry_has_at_least_one_shape() {
        for (language, entries) in ALL {
            for (letter, shapes) in entries {
                assert!(
                    shapes.iter().any(Option::is_some),
                    "{letter:?} in {language:?} has no shape"
                );
            }
        }
    }

    #[test]
    fn v2_leaves_a_standalone_letter_unshaped() {
        // The point of ArabicV2: a letter that stands alone keeps its base form
        // rather than the presentation form, so it renders in a font with no
        // Arabic presentation block.
        assert_eq!(
            lookup('\u{0628}', Language::Arabic).unwrap()[ISOLATED],
            Some('\u{fe8f}')
        );
        assert_eq!(
            lookup('\u{0628}', Language::ArabicV2).unwrap()[ISOLATED],
            Some('\u{0628}')
        );
    }

    #[test]
    fn kurdish_shapes_heh_as_one_form() {
        // Kurdish is ArabicV2 with heh forced to a single shape; that is the
        // only entry the two tables disagree on.
        assert_eq!(
            lookup('\u{0647}', Language::Kurdish).unwrap(),
            [
                Some('\u{fbab}'),
                Some('\u{fbab}'),
                Some('\u{fbab}'),
                Some('\u{fbab}'),
            ]
        );
    }
}
