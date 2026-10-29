#!/usr/bin/env python3
"""Builds the `factur-x` corpus from the official `factur-x` Python package.

The crate's four hand-written fixtures prove that the documents a person
thought of round-trip; this generator adds the ones a person would not write
down. It builds random EN 16931 invoices - random numbers, dates, parties with
or without a VAT identifier, one to three lines at one or two VAT rates, and
each optional field present or absent - asks the reference to render each one
in both syntaxes, and records the bytes it produced.

Nothing is hand-written: the expected document for every case is the
reference's own output, so a disagreement is a difference from the reference
and not from an opinion. The gate is `crates/factur-x/tests/oracle.rs`, which
parses each recorded document with the crate and checks that re-generating it
carries the same element paths with the same text.

The reference's CII output is not valid against its own XSD (its minimal
`ApplicableHeaderTradeDelivery` is emitted `xsi:nil`, which the schema forbids),
so the generator renders with `check_xsd=False`. That is the reference's normal
output - the committed hand-written fixtures fail the same check - and the
crate models what the reference writes rather than what its schema says.

Usage:
    pip install factur-x
    python3 scripts/fuzz/factur-x/gen.py [out-dir]

The output directory defaults to `crates/factur-x/tests/fixtures/oracle`. It is
cleared and rewritten; `oracle.tsv` names each case's file and syntax.
"""

import pathlib
import random
import shutil
import sys

from facturx import generate_cii_xml, generate_ubl_xml

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "crates" / "factur-x" / "tests" / "fixtures" / "oracle"

SEED = 0x16931
CASES = 250

GUIDELINE = "urn:cen.eu:en16931:2017"
COUNTRIES = ["DE", "FR", "NL", "ES", "IT", "SE"]
CURRENCIES = ["EUR", "USD", "GBP", "SEK"]
RATES = [0, 7, 19]
UNITS = ["HUR", "C62", "KGM", "MTR", "LTR"]


def money(cents):
    return f"{cents // 100}.{cents % 100:02d}"


def party(rng, name, with_extras):
    out = {
        "name": name,
        "country_code": rng.choice(COUNTRIES),
        "city": rng.choice(["Berlin", "Paris", "Amsterdam", "Madrid", "Rome"]),
        "postcode": f"{rng.randint(10000, 99999)}",
        "addr_l1": f"{rng.randint(1, 200)} {rng.choice(['Main', 'Station', 'Market'])} Street",
    }
    if with_extras:
        if rng.random() < 0.7:
            out["addr_l2"] = f"Unit {rng.randint(1, 40)}"
        if rng.random() < 0.5:
            out["country_subdivision"] = rng.choice(["BE", "BY", "MI", "13"])
        if rng.random() < 0.6:
            out["vat_identifier"] = f"DE{rng.randint(100000000, 999999999)}"
        if rng.random() < 0.4:
            out["legal_identifier"] = f"HRB{rng.randint(1, 99999)}"
            out["legal_identifier_schemeid"] = rng.choice(["FC", "0002"])
        if rng.random() < 0.5:
            contact = {}
            if rng.random() < 0.8:
                contact["name"] = rng.choice(["Anna", "Bob", "Chloe", "Dan"])
            if rng.random() < 0.8:
                contact["phone"] = f"+49{rng.randint(1000000, 9999999)}"
            if rng.random() < 0.8:
                contact["email"] = f"contact{rng.randint(1, 99)}@example.com"
            if contact:
                out["contacts"] = [contact]
    return out


def invoice(rng):
    # Lines are computed in cents so the arithmetic is exact; a float would make
    # the recorded amounts depend on the reference's rounding.
    line_count = rng.randint(1, 3)
    rates = rng.sample(RATES, rng.randint(1, 2))
    lines = []
    by_rate = {rate: {"net": 0, "tax": 0} for rate in rates}
    for index in range(line_count):
        rate = rng.choice(rates)
        quantity = rng.randint(1, 50)
        unit_price = rng.randint(1, 50000)
        net = unit_price * quantity
        tax = round(net * rate / 100)
        by_rate[rate]["net"] += net
        by_rate[rate]["tax"] += tax
        line = {
            "BT-126": str(index + 1),
            "BT-129": str(quantity),
            "BT-130": rng.choice(UNITS),
            "BT-131": money(net),
            "BT-146": money(unit_price),
            "BT-151": "S",
            "BT-153": f"Item {index + 1}",
        }
        if rate != 0:
            line["BT-152"] = money(rate * 100)
        if rng.random() < 0.4:
            line["BT-154"] = f"Description of item {index + 1}"
        lines.append(line)

    net_total = sum(entry["net"] for entry in by_rate.values())
    tax_total = sum(entry["tax"] for entry in by_rate.values())
    gross = net_total + tax_total

    data = {
        "BT-1": f"INV-{rng.randint(1, 999999)}",
        "BT-2": f"{rng.randint(2025, 2027)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}",
        "BT-3": rng.choice(["380", "381", "384"]),
        "BT-5": rng.choice(CURRENCIES),
        "BT-24": GUIDELINE,
        "BT-106": money(net_total),
        "BT-109": money(net_total),
        "BT-110": money(tax_total),
        "BT-112": money(gross),
        "BT-115": money(gross),
        "BG-4": party(rng, f"Seller {rng.randint(1, 999)}", True),
        "BG-7": party(rng, f"Buyer {rng.randint(1, 999)}", rng.random() < 0.5),
        "BG-25": lines,
        "BG-23": [
            {
                "BT-116": money(entry["net"]),
                "BT-117": money(entry["tax"]),
                "BT-118": "S",
                "BT-119": money(rate * 100),
            }
            for rate, entry in by_rate.items()
        ],
    }

    # Optional header fields, each present or absent at random.
    if rng.random() < 0.3:
        data["BT-6"] = "EUR"
    if rng.random() < 0.3:
        data["BT-23"] = rng.choice(
            ["A1", "A2", "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0"]
        )
    if rng.random() < 0.4:
        data["BT-9"] = (
            f"{rng.randint(2025, 2027)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}"
        )
    if rng.random() < 0.4:
        data["BT-10"] = f"PO-{rng.randint(1, 9999)}"
    if rng.random() < 0.3:
        data["BT-13"] = f"ORDER-{rng.randint(1, 9999)}"
    if rng.random() < 0.3:
        data["BT-12"] = f"CONTRACT-{rng.randint(1, 9999)}"
    if rng.random() < 0.4:
        data["BT-20"] = rng.choice(
            ["Payable within 30 days", "Due on receipt", "Net 14"]
        )
    if rng.random() < 0.3:
        note = {"BT-22": rng.choice(["Thanks for your business", "Delivered in full"])}
        if rng.random() < 0.5:
            note["BT-21"] = rng.choice(["AAI", "SUR", "REG"])
        data["BG-1"] = [note]
    return data


def main() -> int:
    out_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)

    rng = random.Random(SEED)
    rows = []
    written = 0
    for case in range(CASES):
        data = invoice(rng)
        for syntax, generate in (("cii", generate_cii_xml), ("ubl", generate_ubl_xml)):
            try:
                xml = generate(data, check_xsd=False, check_schematron=False)
            except Exception as error:  # noqa: BLE001 - report and skip
                print(
                    f"case {case} {syntax}: reference refused: {error}",
                    file=sys.stderr,
                )
                continue
            name = f"{case:04d}_{syntax}.xml"
            (out_dir / name).write_bytes(xml)
            rows.append(f"{case}\t{syntax}\t{name}")
            written += 1

    index = ROOT / "crates" / "factur-x" / "tests" / "fixtures" / "oracle.tsv"
    index.write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(f"{written} documents -> {out_dir}")
    print(f"index -> {index}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
