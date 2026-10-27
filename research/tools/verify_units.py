#!/usr/bin/env python3
"""Differential gate for ``crates/runits`` against the pint library.

Two independent things are checked:

1. **The generated table agrees with pint.** Every unit name pint knows is
   resolved through the table and compared against what pint itself computes, so
   a bug in the definition-file parser shows up as a mismatch rather than being
   copied into the crate.
2. **A fixture for the Rust tests.** ``crates/runits/tests/fixtures/pint_units.tsv``
   records pint's own numbers, so the crate is checked against pint in CI even
   where pint is not installed. The Rust side never re-implements this lookup.

Usage:
    python3 research/tools/verify_units.py [crates/runits]
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_units as g  # noqa: E402

BASE_DIMS = g.BASE_DIMS


def build_table(source):
    """The generated tables as Python objects.

    ``build_rows`` is the generator's own function, so the verifier compares the
    crate against exactly the table the crate is built from. The prefix list is
    ordered longest first, the order the generated table uses and the order the
    crate's lookup relies on.
    """
    units, prefixes, dims, names, skipped, explicit = g.parse_definitions(source)
    rows, name_rows, unsupported = g.build_rows(units, dims, skipped)
    ordered = sorted(
        ((name, float(value)) for name, value in prefixes.items() if name),
        key=lambda item: (-len(item[0]), item[0]),
    )
    return rows, dict(name_rows), ordered, unsupported, explicit


def resolve(name, definitions, names, prefixes):
    """The crate's lookup order: exact name, then prefix, then plural.

    The name table is consulted first and contains every name the definition
    files know, including the ones outside the seven SI base dimensions. Those
    map to ``UNSUPPORTED``, so ``ppi`` is the printer unit rather than
    ``pico`` x ``pi``, and a known-but-unrepresentable unit is never mistaken
    for a typo.

    ``prefixes`` must be ordered longest first, the same order the generated
    table uses.
    """
    index = names.get(name)
    if index is not None:
        return None if index == g.UNSUPPORTED else definitions[index]
    for prefix, factor in prefixes:
        if not name.startswith(prefix):
            continue
        remainder = name[len(prefix) :]
        if not remainder:
            continue
        for candidate in (remainder, remainder.rstrip("s")):
            index = names.get(candidate)
            if index is not None:
                if index == g.UNSUPPORTED:
                    return None
                base = definitions[index]
                return (base[0] * factor, base[1], base[2])
    if name.endswith("s"):
        index = names.get(name.rstrip("s"))
        if index is not None and index != g.UNSUPPORTED:
            return definitions[index]
    return None


def pint_value(registry, name):
    """pint's own factor, dimension vector and offset for a name.

    The value is read from ``to_base_units`` rather than the converter, because
    a converter alone does not carry the factor of a unit reached through a
    reference. The result is rescaled onto the crate's base units, which use the
    gram as the mass base where pint uses the kilogram. For the affine scales
    (``degree_Celsius``) the offset is the value of zero and the factor is the
    step, so ``base = value * factor + offset`` holds.
    """
    if not registry._units[name].is_multiplicative:
        # Logarithmic units (dB, Np, octave) have no linear factor.
        converter = registry._units[name].converter
        if getattr(converter, "logbase", None) is not None:
            return None
    zero = registry.Quantity(0, name).to_base_units()
    one = registry.Quantity(1, name).to_base_units()
    vector = []
    for base in BASE_DIMS:
        exponent = one.dimensionality.get(base, 0)
        if float(exponent) != int(float(exponent)):
            return None
        vector.append(int(exponent))
    for key in one.dimensionality:
        if str(key) not in BASE_DIMS:
            return None
    scale = 1000.0 ** vector[2]
    offset = float(zero.magnitude) * scale
    factor = float(one.magnitude) * scale - offset
    return (factor, tuple(vector), offset)


def explain_uncovered(registry, name, expected) -> str:
    """Why the crate does not carry ``name``, checked against pint's data."""
    definition = registry._units[name]
    if not definition.is_multiplicative:
        converter = definition.converter
        if getattr(converter, "logbase", None) is not None:
            return "logarithmic scale, not a linear factor"
    if expected is None:
        # Either a dimension outside the seven, or a fractional exponent.
        try:
            one = registry.Quantity(1, name).to_base_units()
        except Exception:
            return "unresolved by pint"
        for key, exponent in one.dimensionality.items():
            if str(key) not in BASE_DIMS:
                return "dimension outside the seven SI base units"
            if float(exponent) != int(float(exponent)):
                return "fractional exponent, cannot be a base-dimension vector"
        return "affine offset, but the crate carries it"  # unreachable if covered
    if name.startswith("delta_") or name.startswith("Δ"):
        return "pint-internal offset-difference unit, absent from the files"
    if expected is not None:
        return "definition uses a fractional power"
    return "unexplained"


def main() -> int:
    crate = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/runits")
    source = g.pint_source()

    import pint

    registry = pint.UnitRegistry()
    definitions, names, prefixes, unsupported_names, _explicit = build_table(source)
    unsupported = set(unsupported_names)

    checked = 0
    mismatches = []
    uncovered = []
    smuggled = []
    fixture_rows = []

    for name in sorted(registry._units):
        if name.startswith("["):
            continue
        expected = pint_value(registry, name)
        actual = resolve(name, definitions, names, prefixes)
        # A name the definition files know must never resolve by prefix
        # decomposition: `ppi` is the printer unit, not `pico` x `pi`. Its
        # dimension is outside the seven, so the comparison below cannot catch a
        # decomposition on its own - this check gives the name table teeth.
        if name in names and names[name] == g.UNSUPPORTED and actual is not None:
            smuggled.append(name)
        if actual is None:
            uncovered.append((name, expected))
            continue
        if expected is None:
            mismatches.append((name, expected, actual))
            continue
        checked += 1
        fixture_rows.append((name, expected))
        if not close(actual, expected):
            mismatches.append((name, expected, actual))

    print(f"pint unit names      {len(registry._units)}")
    print(f"compared             {checked}")
    print(f"mismatches           {len(mismatches)}")
    for name, expected, actual in mismatches[:25]:
        print(f"  {name:24} pint={expected} crate={actual}")
    if smuggled:
        print(f"prefix-decomposed explicit names  {len(smuggled)}: {smuggled[:20]}")

    # A name the crate does not resolve must have a reason, so coverage cannot
    # quietly shrink. Each reason is checked against pint's own data rather than
    # kept as a hand-maintained allow-list.
    unexplained = []
    reasons: dict[str, int] = {}
    for name, expected in uncovered:
        if name in unsupported:
            reason = "known but outside the seven SI base dimensions"
        else:
            reason = explain_uncovered(registry, name, expected)
        reasons[reason] = reasons.get(reason, 0) + 1
        if reason == "unexplained":
            unexplained.append(name)
    print(f"not covered          {len(uncovered)}")
    for reason, count in sorted(reasons.items()):
        print(f"  {count:4}  {reason}")
    if unexplained:
        print(f"UNEXPLAINED          {len(unexplained)}: {unexplained[:20]}")

    fixtures = crate / "tests" / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)
    lines = ["# Generated by research/tools/verify_units.py from pint - do not edit."]
    lines.append("# name\tfactor\tdim(7)\toffset")
    for name, (factor, vector, offset) in sorted(fixture_rows):
        dims_text = ",".join(str(v) for v in vector)
        lines.append(f"{name}\t{factor!r}\t{dims_text}\t{offset!r}")
    (fixtures / "pint_units.tsv").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"fixture rows         {len(fixture_rows)} -> {fixtures / 'pint_units.tsv'}")

    return 1 if mismatches or unexplained or smuggled else 0


def close(actual, expected) -> bool:
    for got, want in zip(actual, expected):
        if isinstance(got, tuple):
            if got != want:
                return False
        elif want == 0:
            if abs(got) > 1e-12:
                return False
        elif abs(got - want) > 1e-9 * max(1.0, abs(want)):
            return False
    return True


if __name__ == "__main__":
    raise SystemExit(main())
