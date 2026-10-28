//! The `CONVERT` function: a value in one unit expressed in another.
//!
//! The unit table is generated from `@formulajs/formulajs` by
//! `research/tools/gen_convert_units.py` and lives in [`convert_table`]. A unit
//! may be written with a decimal SI prefix (`k`, `m`, `M`) or a binary prefix
//! (`ki`, `Mi`), and the prefix is applied only when the bare name is not a unit
//! in its own right, so `min` stays a minute and `mmHg` stays a millimetre of
//! mercury.
//!
//! The reference is Excel's `CONVERT`. One deliberate difference: a conversion
//! between two different categories, or an unknown unit, is reported as an
//! error, which is how a spreadsheet reports `#N/A`, rather than the reference's
//! `{}`.

use crate::Error;
use crate::convert_table::UNITS;

/// The decimal SI prefixes, longest first so `da` is not read as `d`.
const DECIMAL_PREFIXES: &[(&str, f64)] = &[
    ("da", 10.0),
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 100.0),
    ("e", 10.0),
    ("d", 0.1),
    ("c", 0.01),
    ("m", 0.001),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
    ("a", 1e-18),
    ("z", 1e-21),
    ("y", 1e-24),
];

/// The binary prefixes, spelled with two letters, as in `kibyte`.
const BINARY_PREFIXES: &[(&str, f64)] = &[
    ("Yi", 1_208_925_819_614_629_174_706_176.0),
    ("Zi", 1_180_591_620_717_411_303_424.0),
    ("Ei", 1_152_921_504_606_846_976.0),
    ("Pi", 1_125_899_906_842_624.0),
    ("Ti", 1_099_511_627_776.0),
    ("Gi", 1_073_741_824.0),
    ("Mi", 1_048_576.0),
    ("ki", 1024.0),
];

/// A unit resolved from the text written in the formula.
struct Resolved {
    /// The category the unit belongs to.
    category: &'static str,
    /// The unit's factor to the category's base unit.
    factor: f64,
    /// The multiplier contributed by a prefix, or 1.0 when there was none.
    prefix: f64,
}

/// Looks a unit up by symbol or alias.
fn find_unit(text: &str) -> Option<&'static crate::convert_table::Unit> {
    UNITS
        .iter()
        .find(|u| u.symbol == text || u.aliases.contains(&text))
}

/// Resolves a unit string, applying a prefix when the bare name is not a unit.
fn resolve(text: &str) -> Option<Resolved> {
    if let Some(unit) = find_unit(text) {
        return Some(Resolved {
            category: unit.category,
            factor: unit.factor,
            prefix: 1.0,
        });
    }
    // A binary prefix is tried first, because its two-letter spelling cannot be
    // mistaken for a one-letter decimal prefix.
    for (prefix, multiplier) in BINARY_PREFIXES {
        if let Some(unit) = text.strip_prefix(prefix).and_then(find_unit) {
            return Some(Resolved {
                category: unit.category,
                factor: unit.factor,
                prefix: *multiplier,
            });
        }
    }
    for (prefix, multiplier) in DECIMAL_PREFIXES {
        if let Some(unit) = text.strip_prefix(prefix).and_then(find_unit) {
            return Some(Resolved {
                category: unit.category,
                factor: unit.factor,
                prefix: *multiplier,
            });
        }
    }
    None
}

/// The `CONVERT` function: `number` in `from` units expressed in `to` units.
///
/// # Errors
///
/// Returns [`Error::Arithmetic`] with [`ValueError::NotANumber`] when either
/// unit is unknown, or when the two units measure different things.
///
/// [`ValueError::NotANumber`]: crate::ValueError::NotANumber
pub(crate) fn convert(number: f64, from: &str, to: &str) -> Result<f64, Error> {
    let (Some(from), Some(to)) = (resolve(from), resolve(to)) else {
        return Err(Error::Arithmetic(crate::ValueError::NotANumber));
    };
    if from.category != to.category {
        return Err(Error::Arithmetic(crate::ValueError::NotANumber));
    }
    Ok(number * from.factor * from.prefix / (to.factor * to.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_within_a_category() {
        assert!((convert(1.0, "m", "cm").unwrap() - 100.0).abs() < 1e-9);
        assert!((convert(1.0, "kg", "lbm").unwrap() - 2.204_622_621_848_775_7).abs() < 1e-12);
        assert!((convert(1.0, "day", "hr").unwrap() - 24.0).abs() < 1e-9);
        assert!((convert(1.0, "ft", "m").unwrap() - 0.3048).abs() < 1e-12);
        assert!((convert(1.0, "mi", "km").unwrap() - 1.609_344).abs() < 1e-9);
        assert!((convert(1.0, "Wh", "J").unwrap() - 3600.0).abs() < 1e-9);
    }

    #[test]
    fn applies_a_prefix() {
        assert!((convert(1.0, "km", "m").unwrap() - 1000.0).abs() < 1e-9);
        assert!((convert(1.0, "kibyte", "bit").unwrap() - 8192.0).abs() < 1e-9);
        assert!((convert(1.0, "Mibit", "bit").unwrap() - 1_048_576.0).abs() < 1e-9);
        // `mmHg` is a unit of its own, not `m` applied to `mHg`.
        assert!((convert(1.0, "Pa", "mmHg").unwrap() - 0.007_500_637_554_192_106).abs() < 1e-15);
    }

    #[test]
    fn an_alias_works_like_its_symbol() {
        let alias = convert(1.0, "lt", "gal").unwrap();
        let symbol = convert(1.0, "L", "gal").unwrap();
        assert!((alias - symbol).abs() < 1e-12);
    }

    #[test]
    fn an_unknown_unit_is_an_error() {
        assert!(convert(1.0, "furlong", "m").is_err());
        assert!(convert(1.0, "m", "smoot").is_err());
    }

    #[test]
    fn mixing_categories_is_an_error() {
        assert!(convert(1.0, "m", "kg").is_err());
    }
}
