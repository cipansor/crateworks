#!/usr/bin/env python3
"""Generate the unit table for ``crates/runits`` from pint's definition files.

This does NOT ask pint what a unit is worth. It parses the definition text
(``default_en.txt`` and ``constants_en.txt``) with its own expression evaluator
and resolves each unit from the raw relations the file states, such as
``inch = yard / 36``. The differential gate ``verify_units.py`` then compares the
generated table against the pint *library*, so a bug in this parser shows up as
a mismatch rather than being copied into the crate.

Usage:
    python3 research/tools/gen_units.py crates/runits
"""

from __future__ import annotations

UNSUPPORTED = 65535  # u16::MAX, marks a name the crate cannot represent

import pathlib
import re
import sys
from fractions import Fraction

# The seven SI base dimensions, in the order the Rust table stores them.
BASE_DIMS = [
    "[length]",
    "[time]",
    "[mass]",
    "[current]",
    "[temperature]",
    "[substance]",
    "[luminosity]",
]

TOKEN = re.compile(
    r"\s*(\*\*|[*/()]|\^|[0-9]+(?:\.[0-9]*)?(?:[eE][-+]?[0-9]+)?|[^\s*/()^]+)"
)


def lex(text: str) -> list[str]:
    """Split an expression into tokens, handling juxtaposition as multiplication."""
    tokens: list[str] = []
    i = 0
    while i < len(text):
        match = TOKEN.match(text, i)
        if not match:
            raise ValueError(f"cannot tokenise {text[i:]!r} in {text!r}")
        token = match.group(1)
        # `a b` is `a * b` in this format (e.g. `2150.42 cubic_inch`).
        if tokens and _juxtaposes(tokens[-1], token):
            tokens.append("*")
        tokens.append(token)
        i = match.end()
    return tokens


def _juxtaposes(left: str, right: str) -> bool:
    operators = ("*", "/", "(", ")", "**", "^", "+", "-")
    return left not in operators and right not in operators


class _Environment(dict):
    """Symbol table that resolves ``prefix+unit`` names on lookup.

    A definition may name a prefixed unit directly, as in
    ``newton = kilogram * meter / second ** 2`` where only ``gram`` is defined.
    Resolving on demand keeps that O(1) instead of pre-building every
    combination.

    A name that the definition files define explicitly always wins over a
    prefix decomposition, so ``m_e`` (electron mass) is never read as
    ``milli`` + ``e`` (elementary charge). Until such a name is resolved the
    lookup raises ``KeyError`` so the caller retries it in a later pass.
    """

    def __init__(self, prefixes: dict[str, Fraction], explicit: set[str]):
        super().__init__()
        self._prefixes = sorted(prefixes.items(), key=lambda kv: -len(kv[0]))
        self._explicit = explicit

    def __missing__(self, name: str) -> Quantity:
        if name in self._explicit:
            raise KeyError(name)
        for prefix, factor in self._prefixes:
            if not prefix or not name.startswith(prefix) or name == prefix:
                continue
            remainder = name[len(prefix) :]
            base = dict.get(self, remainder)
            if base is None:
                base = dict.get(self, remainder.rstrip("s"))
            if base is not None:
                value = Quantity(base.factor * factor, base.dims)
                self[name] = value
                return value
        raise KeyError(name)


class Quantity:
    """A scalar factor together with a dimension vector."""

    def __init__(self, factor: Fraction, dims: dict[str, int]):
        self.factor = factor
        self.dims = {k: v for k, v in dims.items() if v}

    def __mul__(self, other: "Quantity") -> "Quantity":
        dims = dict(self.dims)
        for key, exp in other.dims.items():
            dims[key] = dims.get(key, 0) + exp
        return Quantity(self.factor * other.factor, dims)

    def __truediv__(self, other: "Quantity") -> "Quantity":
        dims = dict(self.dims)
        for key, exp in other.dims.items():
            dims[key] = dims.get(key, 0) - exp
        return Quantity(self.factor / other.factor, dims)

    def __pow__(self, other: "Quantity") -> "Quantity":
        if other.dims:
            raise ValueError("dimensionful exponent")
        exp = other.factor
        if self.dims:
            if exp.denominator != 1:
                raise ValueError("fractional power of a dimensioned quantity")
            return Quantity(
                self.factor**exp, {k: v * exp.numerator for k, v in self.dims.items()}
            )
        return Quantity(self.factor**exp, {})


class Parser:
    def __init__(self, tokens: list[str], env: dict[str, Quantity]):
        self.tokens = tokens
        self.pos = 0
        self.env = env

    def peek(self) -> str | None:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def eat(self, expected: str | None = None) -> str:
        token = self.peek()
        if expected is not None and token != expected:
            raise ValueError(f"expected {expected!r}, got {token!r}")
        if token is None:
            raise ValueError("unexpected end of expression")
        self.pos += 1
        return token

    def expression(self) -> Quantity:
        value = self.term()
        while self.peek() in ("+", "-"):
            operator = self.eat()
            right = self.term()
            if value.dims and right.dims:
                raise ValueError("cannot add two dimensioned quantities")
            if operator == "+":
                value = Quantity(value.factor + right.factor, value.dims or right.dims)
            else:
                value = Quantity(value.factor - right.factor, value.dims or right.dims)
        return value

    def term(self) -> Quantity:
        value = self.power()
        while self.peek() in ("*", "/"):
            operator = self.eat()
            right = self.power()
            value = value * right if operator == "*" else value / right
        return value

    def power(self) -> Quantity:
        base = self.atom()
        token = self.peek()
        if token in ("**", "^"):
            self.eat()
            return base ** self.power()
        return base

    def atom(self) -> Quantity:
        token = self.peek()
        if token == "(":
            self.eat("(")
            value = self.expression()
            self.eat(")")
            return value
        if token in ("-", "+"):
            self.eat()
            inner = self.power()
            return Quantity(-inner.factor, inner.dims) if token == "-" else inner
        self.eat()
        if re.match(r"^[+-]?[0-9]", token):
            return Quantity(Fraction(token), {})
        try:
            return self.env[token]
        except KeyError:
            raise KeyError(token) from None


def evaluate(text: str, env: dict[str, Quantity]) -> Quantity:
    return Parser(lex(text), env).expression()


def pint_source() -> pathlib.Path:
    import pint

    return pathlib.Path(pint.__file__).parent


def parse_definitions(source: pathlib.Path):
    """Parse pint's definition files into raw unit/prefix tables.

    ``constants_en.txt`` is imported before the units it refers to are defined
    (``speed_of_light = 299792458 m/s``), so resolution runs to a fixpoint: each
    pass resolves what it can and the next pass retries the rest.
    """
    records: list[tuple] = []
    dims: dict[str, Quantity] = {}

    def collect(path: pathlib.Path, block_stack: list[str]):
        text = path.read_text(encoding="utf-8")
        for number, raw in enumerate(text.splitlines(), start=1):
            line = raw.split("#", 1)[0].rstrip()
            if not line.strip():
                continue
            stripped = line.strip()
            if stripped.startswith("@import"):
                collect(source / stripped.split(None, 1)[1].strip(), block_stack)
                continue
            if stripped.startswith("@"):
                directive = stripped[1:].split(None, 1)[0].split("(", 1)[0]
                directive = "@" + directive
                if directive in ("@group", "@context", "@system", "@defaults"):
                    block_stack.append(directive)
                elif directive == "@end":
                    if block_stack:
                        block_stack.pop()
                elif directive == "@alias":
                    body = stripped[len("@alias") :].strip()
                    parts = [p.strip() for p in body.split("=")]
                    records.append(("alias", parts[0], parts[1:], None))
                continue
            if block_stack and block_stack[-1] in ("@context", "@system", "@defaults"):
                continue

            parts = [p.strip() for p in stripped.split("=")]
            name = parts[0]
            value_text, *directives = [p.strip() for p in parts[1].split(";")]
            symbol_and_aliases = parts[2:]
            if name.endswith("-"):
                kind = "prefix"
            elif name.startswith("[") and name.endswith("]"):
                kind = "dim"
            else:
                kind = "unit"
            records.append(
                (kind, name, value_text, directives, symbol_and_aliases, number)
            )

    collect(source / "default_en.txt", [])

    units: dict[str, tuple[Quantity, float | None]] = {}
    prefixes: dict[str, Fraction] = {}

    explicit: set[str] = set()
    for record in records:
        kind, name, *_rest = record
        if kind == "unit":
            explicit.add(name)
            symbol_and_aliases = record[4] or []
            for alias in symbol_and_aliases:
                if alias and alias != "_":
                    explicit.add(alias)
        elif kind == "alias":
            explicit.add(name)
            for alias in record[2]:
                explicit.add(alias)

    def environment() -> dict[str, Quantity]:
        env = _Environment(prefixes, explicit)
        env.update({name: Quantity(Fraction(1), {name: 1}) for name in BASE_DIMS})
        env["[]"] = Quantity(Fraction(1), {})
        env.update(dims)
        env.update({name: entry[0] for name, entry in units.items()})
        return env

    pending = list(records)
    unsupported: list[tuple] = []
    while pending:
        remaining = []
        progress = False
        for record in pending:
            kind, name, value_text, directives, symbol_and_aliases, number = record
            try:
                if kind == "alias":
                    target = units[name][0]
                    for alias in value_text:
                        units[alias] = (target, None)
                    progress = True
                    continue
                value = evaluate(value_text, environment())
                if kind == "prefix":
                    if value.dims:
                        raise ValueError(f"prefix {name!r} is dimensioned")
                    symbol = symbol_and_aliases[0] if symbol_and_aliases else ""
                    aliases = symbol_and_aliases[1:] if symbol_and_aliases else []
                    for candidate in [symbol, *aliases, name]:
                        candidate = candidate.rstrip("-")
                        if candidate and candidate != "_":
                            prefixes[candidate] = value.factor
                elif kind == "dim":
                    dims[name] = value
                else:
                    offset = None
                    logarithmic = False
                    for directive in directives:
                        if directive.startswith("offset:"):
                            offset = evaluate(
                                directive[len("offset:") :].strip(), environment()
                            )
                            offset = float(offset.factor)
                        elif directive.startswith(("logbase:", "logfactor:")):
                            logarithmic = True
                    if logarithmic:
                        # A logarithmic unit (dB, Np, octave) has no linear
                        # factor, so representing it as one would be wrong.
                        raise ValueError(f"logarithmic unit {name!r}")
                    entry = (value, offset)
                    units[name] = entry
                    symbol = symbol_and_aliases[0] if symbol_and_aliases else None
                    aliases = symbol_and_aliases[1:] if symbol_and_aliases else []
                    for alias in [symbol, *aliases]:
                        if alias and alias != "_":
                            units[alias] = entry
                progress = True
            except KeyError:
                # A name used before it is defined: retry after the next pass.
                remaining.append(record)
            except ValueError:
                # Fractional dimensions (the CGS-EM units) are outside the seven
                # SI base dimensions this crate stores, so they are skipped.
                unsupported.append(record)
        if not progress:
            # Everything left depends on a unit that was skipped, so it is out of
            # scope for the same reason. Report rather than fail.
            unsupported.extend(remaining)
            break
        pending = remaining

    names = sorted(units)
    unsupported_names: set[str] = set()
    for record in unsupported:
        kind, name = record[0], record[1]
        if kind == "dim":
            # A dimension name such as `[esu_charge]` is not a unit name, so it
            # must not enter the name table.
            continue
        unsupported_names.add(name)
        symbol_and_aliases = record[4] or []
        for alias in [symbol_and_aliases[0] if symbol_and_aliases else None,
                      *(symbol_and_aliases[1:] if symbol_and_aliases else [])]:
            if alias and alias != "_":
                unsupported_names.add(alias)
    for name in units:
        unit, offset = units[name]
        if not supported(unit, offset, dims):
            unsupported_names.add(name)
    return units, prefixes, dims, names, sorted(unsupported_names), explicit


def build_rows(units, dims, unsupported_names=()):
    """Group units into distinct definitions and name -> row entries.

    A name the crate cannot represent still gets an entry, pointing at
    ``UNSUPPORTED``, so that a known unit is never mistaken for a typo and is
    never read as a prefix decomposition. Both the generator and the verifier
    call this, so the table they compare is literally the same one.
    """
    rows: list[tuple[float, tuple[int, ...], float]] = []
    row_index: dict[tuple[float, tuple[int, ...], float], int] = {}
    name_rows: list[tuple[str, int]] = []
    unsupported: list[str] = []

    for name in sorted(units):
        unit, offset = units[name]
        if not supported(unit, offset, dims):
            name_rows.append((name, UNSUPPORTED))
            unsupported.append(name)
            continue
        base = to_base(unit, dims)
        vector = tuple(int(base.get(d, 0)) for d in BASE_DIMS)
        factor = float(unit.factor)
        key = (factor, vector, offset or 0.0)
        if key not in row_index:
            row_index[key] = len(rows)
            rows.append(key)
        name_rows.append((name, row_index[key]))

    known = {name for name, _ in name_rows}
    for name in unsupported_names:
        if name not in known:
            name_rows.append((name, UNSUPPORTED))
            unsupported.append(name)
    name_rows.sort()
    return rows, name_rows, unsupported


def to_base(unit: Quantity, dims: dict[str, Quantity]) -> dict[str, int]:
    """Reduce a unit's dimension vector to the seven SI base dimensions.

    Derived dimensions (``[area] = [length] ** 2``) are expanded recursively, so
    a unit defined against ``[area]`` still lands on ``[length] ** 2``.
    """

    def expand(name: str, seen: frozenset[str]) -> dict[str, int]:
        if name in BASE_DIMS:
            return {name: 1}
        if name in seen:
            raise ValueError(f"cyclic dimension definition at {name!r}")
        if name not in dims:
            raise KeyError(name)
        result: dict[str, int] = {}
        for inner, exponent in dims[name].dims.items():
            for base, count in expand(inner, seen | {name}).items():
                result[base] = result.get(base, 0) + count * exponent
        return result

    resolved: dict[str, int] = {}
    for name, exponent in unit.dims.items():
        for base, count in expand(name, frozenset()).items():
            resolved[base] = resolved.get(base, 0) + count * exponent
    return {k: v for k, v in resolved.items() if v}


def supported(unit: Quantity, offset: float | None, dims) -> bool:
    """Whether the crate can represent this unit exactly."""
    try:
        base = to_base(unit, dims)
    except (KeyError, ValueError):
        return False
    return all(name in BASE_DIMS for name in base)


def rust_float(value: float) -> str:
    # `repr` prints an exponent the way Python does (`7.7e-08`, `1.1e+18`),
    # but clippy 1.88 reads that as excessive precision: it wants the
    # canonical Rust spelling, a bare `e` with no `+` and no leading zero.
    # The two forms are the same `f64`, so only the spelling changes.
    text = repr(float(value))
    if "e" in text:
        mantissa, _, exponent = text.partition("e")
        return f"{mantissa}e{int(exponent)}"
    if "." in text or "inf" in text:
        return text
    return text + ".0"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip())
        return 2
    crate = pathlib.Path(sys.argv[1])
    source = pint_source()
    units, prefixes, dims, names, skipped_names, _explicit = parse_definitions(source)

    # Collect one row per distinct definition, then a name -> row index list.
    rows, name_rows, skipped = build_rows(units, dims, skipped_names)

    # Longest first, so a lookup can stop at the first prefix that matches
    # without `d` shadowing `da`.
    usable_prefixes = sorted(
        (
            (name, float(value))
            for name, value in prefixes.items()
            if name and value.denominator != 0
        ),
        key=lambda item: (-len(item[0]), item[0]),
    )

    lines: list[str] = []
    lines.append("// Generated by research/tools/gen_units.py - do not edit by hand.")
    lines.append("//")
    lines.append("// Source: pint's `default_en.txt` and `constants_en.txt`, parsed from the")
    lines.append("// definition text rather than queried from the library. See NOTICE.")
    lines.append("")
    lines.append("/// A distinct unit definition: a factor to the SI base units, the base-dimension")
    lines.append("/// exponents, and an additive offset for the affine temperature scales.")
    lines.append("pub(crate) struct Definition {")
    lines.append("    pub(crate) factor: f64,")
    lines.append("    pub(crate) dim: [i8; 7],")
    lines.append("    pub(crate) offset: f64,")
    lines.append("}")
    lines.append("")
    lines.append("/// The row index used for a name that is known but not representable.")
    lines.append("pub(crate) const UNSUPPORTED: u16 = u16::MAX;")
    lines.append("")
    # The factors are data, not approximations of the maths constants they
    # happen to equal: `turn` really is 6.283185307179586 radians.
    lines.append("#[allow(clippy::approx_constant)]")
    # One row per line is the readable form here, and rustfmt would explode each
    # row into four, so the generated file opts out of formatting.
    lines.append("#[rustfmt::skip]")
    lines.append("pub(crate) static DEFINITIONS: &[Definition] = &[")
    for factor, vector, offset in rows:
        dims_text = ", ".join(str(v) for v in vector)
        lines.append(
            f"    Definition {{ factor: {rust_float(factor)}, "
            f"dim: [{dims_text}], offset: {rust_float(offset)} }},"
        )
    lines.append("];")
    lines.append("")
    lines.append("/// Every name the crate resolves, including symbols and aliases.")
    lines.append("#[rustfmt::skip]")
    lines.append("pub(crate) static NAMES: &[(&str, u16)] = &[")
    for name, index in name_rows:
        lines.append(f'    ({name!r}, {index}),'.replace("'", '"'))
    lines.append("];")
    lines.append("")
    lines.append("/// SI prefixes: the name or symbol, and its factor.")
    lines.append("#[rustfmt::skip]")
    lines.append("pub(crate) static PREFIXES: &[(&str, f64)] = &[")
    for name, value in usable_prefixes:
        lines.append(f'    ({name!r}, {rust_float(value)}),'.replace("'", '"'))
    lines.append("];")

    (crate / "src").mkdir(parents=True, exist_ok=True)
    # No trailing blank line: `cargo fmt` strips one, so emitting it would
    # leave the tree dirty after every regeneration.
    (crate / "src" / "table.rs").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"definitions   {len(rows)}")
    print(f"names         {len(name_rows)}")
    print(f"prefixes      {len(usable_prefixes)}")
    print(f"skipped       {len(skipped)} (dimensions outside the seven SI base units)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
