#!/usr/bin/env python3
"""Regenerate `crates/stdnum-rs/tests/fixtures/stdnum_vectors.tsv`.

Every line is `<RustCountryName>\t<value>\t<1|0>`, where the verdict is what
python-stdnum (the reference implementation) says. The crate's integration test
replays the file and fails on any disagreement, so this script is the oracle
gate for `stdnum-rs`.

Numbers come from two places:

* the reference module's own doctests, which are the values its authors
  published, and
* mutations of those: every single-digit substitution, the last character
  dropped, and a character appended. A mutation is what a typo produces, and a
  check-digit routine that accepts one is not doing its job.

Run from the repository root:

    python3 research/tools/gen_stdnum_vectors.py

It rewrites the fixture in place and prints a per-country summary.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Rust `Country` variant name -> dotted python-stdnum module path.
# Add a row here and a matching `Country` arm in the crate, and the gate covers
# the new country automatically.
COUNTRIES: dict[str, str] = {
    # Existing coverage (kept so the fixture stays one file).
    "IndonesiaNpwp": "id.npwp",
    "IndonesiaNik": "id.nik",
    "BrazilCpf": "br.cpf",
    "BrazilCnpj": "br.cnpj",
    "SpainDni": "es.dni",
    "GermanyTaxId": "de.idnr",
    "PolandPesel": "pl.pesel",
    "IndiaAadhaar": "in_.aadhaar",
    "NetherlandsBsn": "nl.bsn",
    "BelgiumNationalNumber": "be.nn",
    "SwedenPersonnummer": "se.personnummer",
    "NorwayBirthNumber": "no.fodselsnummer",
    "NorwayOrgNumber": "no.orgnr",
    "DenmarkCpr": "dk.cpr",
    "FinlandHetu": "fi.hetu",
    "PortugalNif": "pt.nif",
    "FranceSiren": "fr.siren",
    "FranceSiret": "fr.siret",
    "ItalyCodiceFiscale": "it.codicefiscale",
    "RomaniaCnp": "ro.cnp",
    "TurkeyTckn": "tr.tckimlik",
    "CanadaSin": "ca.sin",
    "AustraliaAbn": "au.abn",
    "AustraliaTfn": "au.tfn",
    "NewZealandIrd": "nz.ird",
    "ChinaRic": "cn.ric",
    "KoreaRrn": "kr.rrn",
    "MexicoRfc": "mx.rfc",
    # New coverage added in 0.1.6.
    "ChileRut": "cl.rut",
    "ColombiaNit": "co.nit",
    "PeruRuc": "pe.ruc",
    "UruguayRut": "uy.rut",
    "ArgentinaCuit": "ar.cuit",
    "IrelandVat": "ie.vat",
    "IrelandPps": "ie.pps",
    "HungaryAnum": "hu.anum",
    "CroatiaOib": "hr.oib",
    "BulgariaEgn": "bg.egn",
    "BulgariaPnf": "bg.pnf",
    "CzechRc": "cz.rc",
    "SlovakiaRc": "sk.rc",
    "EstoniaIk": "ee.ik",
    "LithuaniaAsmens": "lt.asmens",
    "LuxembourgTva": "lu.tva",
    "SwitzerlandSsn": "ch.ssn",
    "SwitzerlandUid": "ch.uid",
    "RussiaInn": "ru.inn",
    "RussiaOgrn": "ru.ogrn",
    "UkraineEdrpou": "ua.edrpou",
    "IsraelIdnr": "il.idnr",
    "ThailandPin": "th.pin",
    "SingaporeUen": "sg.uen",
    "MalaysiaNric": "my.nric",
    "SouthAfricaIdnr": "za.idnr",
    "VenezuelaRif": "ve.rif",
    "GreeceAmka": "gr.amka",
    # Added in 0.1.6: the Balkan, further-European and VAT set.
    "SerbiaPib": "rs.pib",
    "MontenegroPib": "me.pib",
    "NorthMacedoniaEdb": "mk.edb",
    "SloveniaDdv": "si.ddv",
    "RomaniaCf": "ro.cf",
    "BelarusUnp": "by.unp",
    "CyprusVat": "cy.vat",
    "MaltaVat": "mt.vat",
    "GreeceVat": "gr.vat",
    "FinlandAlv": "fi.alv",
    "SwedenOrgnr": "se.orgnr",
    "SwedenVat": "se.vat",
    "CzechDic": "cz.dic",
    # Added in 0.1.6: the African and further-Middle-Eastern set.
    "AlgeriaNif": "dz.nif",
    "EgyptTn": "eg.tn",
    "GhanaTin": "gh.tin",
    "KenyaPin": "ke.pin",
    "MoroccoIce": "ma.ice",
    "MozambiqueNuit": "mz.nuit",
    "SenegalNinea": "sn.ninea",
    "TunisiaMf": "tn.mf",
    "VietnamMst": "vn.mst",
}

EXAMPLE_RE = re.compile(r">>> validate\('([^']*)'\)")
FAIL_RE = re.compile(r">>> validate\('([^']*)'\)\nTraceback")

# Rust `NumberType` variant name -> dotted python-stdnum module path, for the
# international identifiers that are not tied to a country. The fixture format
# is the same, and the crate's test dispatches a name to either a `Country` or a
# `NumberType`.
INTERNATIONAL: dict[str, str] = {
    "Isin": "isin",
    "Cusip": "cusip",
    "Figi": "figi",
    "Lei": "lei",
    "Bic": "bic",
    "Grid": "grid",
    "Issn": "issn",
    "Isrc": "isrc",
    "Imei": "imei",
    "Ismn": "ismn",
    "Isni": "isni",
    "CasNumber": "casrn",
    "CreditorReference": "iso11649",
}


def module(name: str):
    return __import__("stdnum." + name, fromlist=["x"])


def examples(mod) -> tuple[list[str], list[str]]:
    """Return (accepted, rejected) literal arguments from the module doctests."""
    doc = mod.__doc__ or ""
    accepted: list[str] = []
    rejected: list[str] = []
    for match in EXAMPLE_RE.finditer(doc):
        value = match.group(1)
        tail = doc[match.end() : match.end() + 40]
        if tail.lstrip().startswith("Traceback"):
            rejected.append(value)
        else:
            accepted.append(value)
    return accepted, rejected


def mutations(value: str) -> list[str]:
    """Every single-position digit substitution, the last character dropped, and
    one appended.

    Every position is substituted, not only the ones already holding a digit: a
    letter swapped for a digit is just as likely a typo, and the reference says
    what the verdict is either way.
    """
    out: list[str] = []
    for i in range(len(value)):
        for d in "0123456789":
            if d != value[i]:
                out.append(value[:i] + d + value[i + 1 :])
    if len(value) > 1:
        out.append(value[:-1])
    out.append(value + "0")
    return out


def verdict(mod, value: str) -> bool:
    try:
        mod.validate(value)
    except Exception:
        return False
    return True


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    dest = root / "crates/stdnum-rs/tests/fixtures/stdnum_vectors.tsv"

    # Keep every line already in the fixture: those labels came from the same
    # oracle and were reviewed when they were added. Only add countries that
    # are not yet present, so regenerating never loses coverage.
    existing: list[str] = []
    have: set[str] = set()
    if dest.exists():
        for line in dest.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            existing.append(line)
            have.add(line.split("\t", 1)[0])

    lines: list[str] = []
    summary: list[str] = []
    for rust_name, py_name in {**COUNTRIES, **INTERNATIONAL}.items():
        if rust_name in have:
            summary.append(f"{rust_name:22s} kept from the existing fixture")
            continue
        try:
            mod = module(py_name)
        except Exception as exc:  # noqa: BLE001 - report and continue
            print(f"!! {rust_name}: cannot import stdnum.{py_name}: {exc}")
            return 1
        accepted, rejected = examples(mod)
        if not accepted:
            print(f"!! {rust_name}: no accepted doctest example in stdnum.{py_name}")
            return 1
        cases: list[str] = []
        for value in accepted:
            cases.append(value)
            cases.extend(mutations(value))
        for value in rejected:
            cases.append(value)
        seen: set[str] = set()
        count_ok = 0
        for value in cases:
            if value in seen:
                continue
            seen.add(value)
            ok = verdict(mod, value)
            count_ok += ok
            lines.append(f"{rust_name}\t{value}\t{1 if ok else 0}")
        summary.append(
            f"{rust_name:22s} {len(seen):5d} vectors, {count_ok:5d} accepted"
        )

    dest.write_text("\n".join(existing + lines) + "\n", encoding="utf-8")
    print("\n".join(summary))
    print(f"\n{len(existing)} kept, {len(lines)} added -> {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
