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

import random
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
    # Added in 0.1.7: the remaining countries python-stdnum covers.
    "UnitedStatesSsn": "us.ssn",
    "UnitedStatesItin": "us.itin",
    "UnitedKingdomNhs": "gb.nhs",
    "UnitedKingdomUtr": "gb.utr",
    "UnitedKingdomVat": "gb.vat",
    "UnitedKingdomSedol": "gb.sedol",
    "JapanCorporateNumber": "jp.cn",
    "JapanMyNumber": "jp.in_",
    "PakistanCnic": "pk.cnic",
    "TaiwanUbn": "tw.ubn",
    "LatviaPvn": "lv.pvn",
    "LiechtensteinPeid": "li.peid",
    "MonacoTva": "mc.tva",
    "AndorraNrt": "ad.nrt",
    "AlbaniaNipt": "al.nipt",
    "AzerbaijanVoen": "az.voen",
    "FaroeIslandsVn": "fo.vn",
    "MoldovaIdno": "md.idno",
    "SanMarinoCoe": "sm.coe",
    "AustriaUid": "at.uid",
    "AustriaTin": "at.tin",
    "ParaguayRuc": "py.ruc",
    "ElSalvadorNit": "sv.nit",
    # The national modules that had no counterpart when the crate was reviewed.
    "IcelandKennitala": "is_.kennitala",
    "IcelandVsk": "is_.vsk",
    "EcuadorCi": "ec.ci",
    "EcuadorRuc": "ec.ruc",
    "GuatemalaNit": "gt.nit",
    "CostaRicaCpj": "cr.cpj",
    "DominicanRepublicRnc": "do.rnc",
    "GuineaNifp": "gn.nifp",
    "CubaNi": "cu.ni",
    "MauritiusNid": "mu.nid",
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
    # Added in 0.1.7: the identifiers that need no registry.
    "Imo": "imo",
    "Meid": "meid",
    "Isan": "isan",
    "BitcoinAddress": "bitcoin",
    # Added in 0.1.8: the identifiers that need no registry table of their own.
    # MAC is validated without the manufacturer lookup, which needs the IEEE
    # OUI registry the crate deliberately does not ship.
    "Mac": "mac",
    "Isil": "isil",
    # Added in 0.1.9: the two table-driven check-digit algorithms, alongside
    # Luhn and the ISO 7064 family the crate already carried.
    "Damm": "damm",
    "Verhoeff": "verhoeff",
    # Added in 0.1.10: the two EU-domain identifiers.
    "EcNumber": "eu.ecnumber",
    "EuropeanBanknote": "eu.banknote",
    # Added in 0.2.0: the SEPA Creditor Identifier and the European Energy
    # Identification Code, the two remaining self-contained `eu` modules.
    "SepaCreditorIdentifier": "eu.at_02",
    "Eic": "eu.eic",
}


# Modules whose `validate` needs an argument other than the number itself, so
# the verdict matches the mode this crate implements. The MAC manufacturer
# lookup is the only one: the crate checks the shape and the address class bits,
# not the IEEE registry.
VALIDATE_KWARGS: dict[str, dict[str, object]] = {
    "mac": {"validate_manufacturer": False},
}


# Modules whose reference docstring carries no doctest, so the gate has no
# published value to seed mutations from. Each entry is a number the reference
# accepts, found by solving its own check-digit routine.
SEEDS: dict[str, list[str]] = {
    "mu.nid": ["A1501920000003", "B0203901234568"],
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


def verdict(mod, value: str, **kwargs) -> bool:
    try:
        mod.validate(value, **kwargs)
    except Exception:
        return False
    return True


# The check-digit algorithms have a one-line doctest each, so their generic
# corpus is thin. `calc_check_digit` lets the reference build valid numbers from
# random payloads, which is what turns "the two examples agree" into "the table
# agrees over thousands of inputs". A valid number and its single-digit
# mutations are both recorded, so the gate covers acceptance and rejection.
CHECK_DIGIT_ALGORITHMS: dict[str, str] = {
    "Damm": "damm",
    "Verhoeff": "verhoeff",
}


def ecnumber_corpus(mod, rng) -> list[str]:
    """Valid EC Numbers: a random six-digit payload plus the reference's own
    check digit, so the corpus is the reference's answer and not a hand-written
    guess."""
    out: list[str] = []
    for _ in range(15):
        payload = "".join(rng.choice("0123456789") for _ in range(6))
        out.append(payload + mod.calc_check_digit(payload))
    return out


def banknote_corpus(mod, rng) -> list[str]:
    """Valid euro banknote serials.

    Half use the older one-letter head (a country letter, ten digits, a check
    digit); half use the newer two-letter series, whose first two characters may
    both be letters. The last digit closes the reference's checksum to a multiple
    of nine, so the reference accepts every row.
    """
    letters = "BCDEFGHJLMNPRSTUVWXYZ"
    out: list[str] = []
    while len(out) < 15:
        if len(out) % 2 == 0:
            head = rng.choice(letters)
            digits = "".join(rng.choice("0123456789") for _ in range(10))
        else:
            # Two-letter series: the second letter is not restricted to the
            # issuing set, only the first is.
            head = rng.choice(letters) + rng.choice("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            digits = "".join(rng.choice("0123456789") for _ in range(9))
        fixed = sum(ord(c) if c.isalpha() else int(c) for c in head + digits)
        last = (9 - fixed % 9) % 9
        out.append(f"{head}{digits}{last}")
    return out


# Identifiers whose doctest is a single example, so their generic corpus would
# be too thin to catch a divergence. Each builder returns valid numbers, which
# the caller then mutates exactly like a doctest example.
def sepa_creditor_corpus(mod, rng) -> list[str]:
    """Valid SEPA Creditor Identifiers.

    A random ISO country code, a random three-character business code and a
    random national identifier, closed with the reference's own check digits.
    The reference has no length check, so the national identifier is kept long
    enough to stay inside the scheme's own range rather than accidentally
    exercising the crate's documented minimum.
    """
    letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    out: list[str] = []
    for _ in range(15):
        country = rng.choice(letters) + rng.choice(letters)
        business = "".join(rng.choice(letters + "0123456789") for _ in range(3))
        national = "".join(
            rng.choice(letters + "0123456789") for _ in range(rng.randint(3, 20))
        )
        body = country + "00" + business + national
        check = mod.calc_check_digits(body)
        out.append(country + check + business + national)
    return out


def eic_corpus(mod, rng) -> list[str]:
    """Valid Energy Identification Codes: a random fifteen-character payload
    from the reference's own alphabet plus the reference's check character."""
    alphabet = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-"
    out: list[str] = []
    while len(out) < 15:
        payload = "".join(rng.choice(alphabet) for _ in range(15))
        code = payload + mod.calc_check_digit(payload)
        # A check character of `-` is a truncated code the reference rejects, so
        # it would not be a valid seed.
        if code[-1] != "-":
            out.append(code)
    return out


CUSTOM_CORPUS: dict[str, object] = {
    "eu.ecnumber": ecnumber_corpus,
    "eu.banknote": banknote_corpus,
    "eu.at_02": sepa_creditor_corpus,
    "eu.eic": eic_corpus,
}

CORPUS_SIZE = 1500


def algorithm_corpus(rust_name: str, py_name: str, rng) -> list[str]:
    """Rows for a check-digit algorithm: valid numbers plus mutations."""
    mod = module(py_name)
    lines: list[str] = []
    seen: set[str] = set()
    while len(seen) < CORPUS_SIZE:
        length = rng.randint(1, 18)
        payload = "".join(rng.choice("0123456789") for _ in range(length))
        valid = payload + mod.calc_check_digit(payload)
        if valid in seen:
            continue
        seen.add(valid)
        lines.append(f"{rust_name}\t{valid}\t1")
        # One mutation per valid number: replace a random position with a random
        # digit. The reference decides the verdict, which is usually rejection
        # but need not be.
        i = rng.randrange(len(valid))
        d = rng.choice("0123456789")
        mutant = valid[:i] + d + valid[i + 1 :]
        if mutant not in seen:
            seen.add(mutant)
            lines.append(f"{rust_name}\t{mutant}\t{1 if verdict(mod, mutant) else 0}")
    return lines


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
    rng = random.Random(20261027)
    for rust_name, py_name in {**COUNTRIES, **INTERNATIONAL}.items():
        if rust_name in have:
            summary.append(f"{rust_name:22s} kept from the existing fixture")
            continue
        if py_name in CHECK_DIGIT_ALGORITHMS.values():
            corpus = algorithm_corpus(rust_name, py_name, rng)
            lines.extend(corpus)
            accepted = sum(1 for row in corpus if row.endswith("\t1"))
            summary.append(
                f"{rust_name:22s} {len(corpus):5d} vectors, {accepted:5d} accepted"
            )
            continue
        try:
            mod = module(py_name)
        except Exception as exc:  # noqa: BLE001 - report and continue
            print(f"!! {rust_name}: cannot import stdnum.{py_name}: {exc}")
            return 1
        accepted, rejected = examples(mod)
        accepted.extend(SEEDS.get(py_name, ()))
        if py_name in CUSTOM_CORPUS:
            accepted.extend(CUSTOM_CORPUS[py_name](mod, rng))
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
        kwargs = VALIDATE_KWARGS.get(py_name, {})
        for value in cases:
            if value in seen:
                continue
            seen.add(value)
            ok = verdict(mod, value, **kwargs)
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
