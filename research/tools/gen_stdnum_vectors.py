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

    pip install 'python-stdnum==2.2'
    python3 research/tools/gen_stdnum_vectors.py

**Pin the version.** The reference's own data moves between releases - a
country's registry entry, a check-digit routine, a doctest - so a fixture
regenerated against a different release would change the crate's expectations
without a crate change, and two maintainers on two releases would get two
fixtures from one checkout. The version is asserted below, so an environment
with another release stops the run instead of silently re-baselining it.

It rewrites the fixture in place and prints a per-country summary.
"""

from __future__ import annotations

import itertools
import random
import re
import sys
from pathlib import Path

import stdnum

# The reference release this fixture is baselined against. The docstring above
# explains why: the reference's data moves, and a regenerated fixture would
# otherwise differ between environments. Asserting it here means another
# release stops the run instead of silently rewriting the oracle.
PINNED = "2.2"
if stdnum.__version__ != PINNED:
    raise SystemExit(
        f"python-stdnum {stdnum.__version__} is installed, but this fixture is "
        f"baselined against {PINNED}. Install 'python-stdnum=={PINNED}' "
        f"(pip install 'python-stdnum=={PINNED}') or re-pin this file and "
        f"research/tools/README.md together."
    )

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
    # Added in 0.4.0: the identifiers python-stdnum covers that the crate still
    # lacked - the Polish, Nordic, Canadian, Australian, Indian, Mexican and
    # further-European tax and register numbers.
    "PolandNip": "pl.nip",
    "PolandRegon": "pl.regon",
    "FranceNif": "fr.nif",
    "NorwayAccountNumber": "no.kontonr",
    "NorwayMva": "no.mva",
    "DenmarkCvr": "dk.cvr",
    "CanadaBusinessNumber": "ca.bn",
    "AustraliaAcn": "au.acn",
    "IndiaPan": "in_.pan",
    "IndiaGstin": "in_.gstin",
    "MexicoCurp": "mx.curp",
    "EstoniaRegistrikood": "ee.registrikood",
    "EstoniaKmkr": "ee.kmkr",
    "LithuaniaPvm": "lt.pvm",
    "SlovakiaDph": "sk.dph",
    "PortugalCc": "pt.cc",
    "SouthAfricaTin": "za.tin",
    # Added in 0.5.0: further identifiers python-stdnum covers - the US routing
    # transit number, the Turkish business tax number, the Indian voter ID, the
    # Korean business number, the Italian drug code, the Dutch VAT number, the
    # Thai company number and the British Columbia health number.
    "UnitedStatesRtn": "us.rtn",
    "TurkeyVkn": "tr.vkn",
    "IndiaEpic": "in_.epic",
    "KoreaBrn": "kr.brn",
    "ItalyAic": "it.aic",
    "NetherlandsBtw": "nl.btw",
    "ThailandMoa": "th.moa",
    "CanadaBcPhn": "ca.bc_phn",
    # Added in 0.6.0: further identifiers python-stdnum covers - the French
    # social security number, the Italian VAT number, the Romanian company
    # identifier, the Belgian structured payment communication, the Finnish and
    # Swiss business and payment references, the Chinese unified social credit
    # code, the Slovenian and Austrian register numbers, the Peruvian and
    # Argentine identity numbers, the Indian virtual ID, the British pupil
    # number and the Dutch student number.
    "FranceNir": "fr.nir",
    "ItalyPiva": "it.iva",
    "RomaniaCui": "ro.cui",
    "BelgiumOgm": "be.ogm_vcs",
    "FinlandYtunnus": "fi.ytunnus",
    "SwitzerlandEsr": "ch.esr",
    "ChinaUscc": "cn.uscc",
    "SloveniaMaticna": "si.maticna",
    "AustriaVnr": "at.vnr",
    "AustriaFirmenbuch": "at.businessid",
    "PeruCui": "pe.cui",
    "ArgentinaDni": "ar.dni",
    "IndiaVid": "in_.vid",
    "UnitedKingdomUpn": "gb.upn",
    "NetherlandsOnderwijsnummer": "nl.onderwijsnummer",
    # Added in 0.6.0: the second national batch - the Spanish register and
    # property numbers, the German securities and routing codes, the Dutch
    # school and travel documents, the French trade register, the North
    # American tax identifiers, the Israeli company number, the Ukrainian
    # individual tax number, the Argentine bank account, the Costa Rican
    # natural-person number, the Finnish association and tax numbers, the
    # Slovenian citizen number and the Romanian trade register.
    "SpainNif": "es.nif",
    "SpainCif": "es.cif",
    "SpainCcc": "es.ccc",
    "SpainCups": "es.cups",
    "SpainCae": "es.cae",
    "SpainReferenciaCatastral": "es.referenciacatastral",
    "GermanyWkn": "de.wkn",
    "GermanyLeitweg": "de.leitweg",
    "NetherlandsBrin": "nl.brin",
    "NetherlandsIdentityCard": "nl.identiteitskaartnummer",
    "FranceRcs": "fr.rcs",
    "UnitedStatesEin": "us.ein",
    "UnitedStatesPtin": "us.ptin",
    "UnitedStatesAtin": "us.atin",
    "IsraelCompanyNumber": "il.hp",
    "UkraineRntrc": "ua.rntrc",
    "ArgentinaCbu": "ar.cbu",
    "CostaRicaCpf": "cr.cpf",
    "FinlandAssociationId": "fi.associationid",
    "FinlandVeronumero": "fi.veronumero",
    "SloveniaEmso": "si.emso",
    "RomaniaOnrc": "ro.onrc",
    # Added in 0.7.0: the identifiers the 0.6.0 review left behind - the
    # Spanish foreigner number and postal code, the Belgian BIS, identity-card
    # and VAT numbers, the German, Swiss and Bulgarian VAT numbers, the French
    # accise number, the Dutch and Swedish postal codes, the Costa Rican CR
    # number, the Dominican cédula and tax receipt, and the two aggregating
    # taxpayer numbers of the United States and Thailand.
    "SpainNie": "es.nie",
    "SpainPostalCode": "es.postal_code",
    "BelgiumBis": "be.bis",
    "BelgiumEid": "be.eid",
    "BelgiumVat": "be.vat",
    "GermanyVat": "de.vat",
    "GermanySteuernummer": "de.stnr",
    "FranceAccise": "fr.accise",
    "NetherlandsPostcode": "nl.postcode",
    "SwedenPostnummer": "se.postnummer",
    "SwitzerlandVat": "ch.vat",
    "BulgariaVat": "bg.vat",
    "CostaRicaCr": "cr.cr",
    "DominicanRepublicCedula": "do.cedula",
    "DominicanRepublicNcf": "do.ncf",
    "UnitedStatesTin": "us.tin",
    "ThailandTin": "th.tin",
    # The European Union and France VAT numbers added in 0.7.0.
    "EuExcise": "eu.excise",
    "EuOss": "eu.oss",
    "FranceTva": "fr.tva",
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
    # Added in 0.8.0: the four number types the crate's README opens with, which
    # the gate had omitted even though the crate implements them. Their absence
    # is why the IBAN structure and national checks, the ISBN nine-digit form
    # and the ISBN 978/979 prefix rule could drift from the reference unseen.
    "Ean": "ean",
    "Iban": "iban",
    "Isbn": "isbn",
    "Luhn": "luhn",
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

def rntrc_corpus(mod, rng) -> list[str]:
    """Valid Ukrainian RNTRC numbers: a random nine-digit payload closed with
    the reference's own check digit."""
    out: list[str] = []
    while len(out) < 15:
        payload = "".join(rng.choice("0123456789") for _ in range(9))
        out.append(payload + mod.calc_check_digit(payload))
    return out


def cbu_corpus(mod, rng) -> list[str]:
    """Valid Argentine CBU numbers.

    A CBU is three digits of bank, four of branch, one check digit over the
    first seven, then a thirteen-digit account and a check digit over the last
    thirteen. Both check digits come from the reference, so every row is
    accepted by construction.
    """
    out: list[str] = []
    while len(out) < 15:
        head = "".join(rng.choice("0123456789") for _ in range(7))
        head += mod.calc_check_digit(head)
        account = "".join(rng.choice("0123456789") for _ in range(13))
        account += mod.calc_check_digit(account)
        out.append(head + account)
    return out


def emso_corpus(mod, rng) -> list[str]:
    """Valid Slovenian EMŠO numbers: a seven-digit date block the reference
    accepts, five more digits, then its check digit. The reference reads a
    three-digit year by adding 2000 below 800 and 1000 above, so any 000-999
    is a real year and only the day and month need to be plausible."""
    out: list[str] = []
    while len(out) < 15:
        day = rng.randint(1, 28)
        month = rng.randint(1, 12)
        year = rng.randint(0, 999)
        body = f"{day:02d}{month:02d}{year:03d}" + "".join(
            rng.choice("0123456789") for _ in range(5)
        )
        out.append(body + mod.calc_check_digit(body))
    return out


def ccc_corpus(mod, rng) -> list[str]:
    """Valid Spanish CCC bank account codes: eight digits of bank and branch,
    then the reference's two check digits, then ten digits of account."""
    out: list[str] = []
    while len(out) < 15:
        head = "".join(rng.choice("0123456789") for _ in range(8))
        account = "".join(rng.choice("0123456789") for _ in range(10))
        check = mod.calc_check_digits(head + "00" + account)
        out.append(head + check + account)
    return out


def cups_corpus(mod, rng) -> list[str]:
    """Valid Spanish CUPS supply-point codes: `ES`, sixteen digits, the
    reference's two check letters, and sometimes the optional two-character
    suffix the 22-character form carries."""
    out: list[str] = []
    while len(out) < 15:
        body = "".join(rng.choice("0123456789") for _ in range(16))
        code = "ES" + body + mod.calc_check_digits("ES" + body)
        if len(out) % 2 == 1:
            code += rng.choice("0123456789") + rng.choice("FPRCXYZ")
        out.append(code)
    return out


def cadastral_corpus(mod, rng) -> list[str]:
    """Valid Spanish cadastral references: eighteen characters from the
    reference's alphabet (which includes Ñ), then its two check characters."""
    out: list[str] = []
    while len(out) < 15:
        head = "".join(rng.choice(mod.alphabet) for _ in range(18))
        code = head + mod.calc_check_digits(head)
        if len(code) == 20 and mod.is_valid(code):
            out.append(code)
    return out


def nif_corpus(mod, rng) -> list[str]:
    """Valid Spanish NIF numbers.

    Three shapes, each closed with the reference's own check digit: a DNI
    (eight digits and the modulo-23 letter), an NIE (`X`, `Y` or `Z` and seven
    digits), and a CIF (a leading organisation letter and seven digits, whose
    check is the Luhn digit or the letter that stands for it).
    """
    from stdnum import luhn
    from stdnum.es import dni

    letters = "ABCDEFGHJNPQRSUVW"
    out: list[str] = []
    while len(out) < 15:
        kind = len(out) % 3
        if kind == 0:
            body = "".join(rng.choice("0123456789") for _ in range(8))
            out.append(body + dni.calc_check_digit(body))
        elif kind == 1:
            body = "".join(rng.choice("0123456789") for _ in range(7))
            # The reference maps X, Y and Z to 0, 1 and 2 before the check.
            prefix = rng.choice("XYZ")
            value = str("XYZ".index(prefix)) + body
            out.append(prefix + body + dni.calc_check_digit(value))
        else:
            body = "".join(rng.choice("0123456789") for _ in range(7))
            check = luhn.calc_check_digit(body)
            out.append(
                rng.choice(letters) + body + rng.choice([check, "JABCDEFGHI"[int(check)]])
            )
    return out


def rcs_corpus(mod, rng) -> list[str]:
    """Valid French RCS numbers: `RCS`, a city, an `A` or `B`, and a SIREN
    whose last digit is the reference's own Luhn check digit."""
    from stdnum import luhn

    cities = ["Nancy", "Paris", "Lyon", "Marseille", "Toulouse", "Nice"]
    out: list[str] = []
    while len(out) < 15:
        body = "".join(rng.choice("0123456789") for _ in range(8))
        siren = body + luhn.calc_check_digit(body)
        out.append(f"RCS {rng.choice(cities)} {rng.choice('AB')} {siren}")
    return out


def leitweg_corpus(mod, rng) -> list[str]:
    """Valid German Leitweg-IDs.

    The number is a two-digit state code, an optional middle group and two
    check digits, closed so that the whole thing is one modulo 97. The
    reference's own `mod_97_10.calc_check_digits` supplies the closing pair.
    """
    from stdnum.iso7064 import mod_97_10

    states = ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11",
              "12", "13", "14", "15", "16", "99"]
    out: list[str] = []
    while len(out) < 15:
        state = rng.choice(states)
        middle = "".join(rng.choice("0123456789") for _ in range(rng.randint(2, 10)))
        # The reference pattern is digits, an optional dashed group, then a
        # dashed two-digit check, so the corpus carries the second dash.
        code = (
            state
            + "-"
            + middle
            + "-"
            + mod_97_10.calc_check_digits(state + middle)
        )
        if mod.is_valid(code):
            out.append(code)
    return out


def _iban_structure(mod, country: str) -> str:
    """The BBAN structure the reference holds for `country`."""
    info = mod._ibandb.info(country + "0" * 40)
    return info[0][1].get("bban", "")


def _iban_bban(structure: str, rng) -> str:
    """A BBAN that satisfies a structure, one character per class."""
    out: list[str] = []
    rest = structure
    while rest:
        count, tail = rest.split("!", 1)
        cls = tail[0]
        for _ in range(int(count)):
            if cls == "a":
                out.append(rng.choice("ABCDEFGHIJKLMNOPQRSTUVWXYZ"))
            elif cls == "n":
                out.append(rng.choice("0123456789"))
            else:
                out.append(rng.choice("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"))
        rest = tail[1:]
    return "".join(out)


def iban_corpus(mod, rng) -> list[str]:
    """IBANs that exercise the three checks the crate performs.

    Four kinds of row, each isolating one check from the MOD-97 check that
    would otherwise reject it first:

    * a valid IBAN, which both implementations accept;
    * for the countries with an arithmetic national check, a *valid* IBAN whose
      national check digits were solved rather than drawn, so the accepted side
      of the national check is covered as well as the rejected side. Without
      this the only ES/NO/ME rows were rejections, and a crate that rejected
      every ES/NO/ME IBAN would agree with the fixture;
    * a BBAN with a letter where the registry requires a digit, closed with the
      *correct* MOD-97 check digits, so only the structure check separates the
      two;
    * a structurally valid BBAN with wrong national check digits, again closed
      with correct MOD-97 check digits, so only the national check separates
      them.

    Belgium is left out of the national-check rows: the reference looks the bank
    code up in a register this crate deliberately does not ship, so a Belgian
    IBAN is only required to match on structure and MOD-97.
    """
    countries = [
        "AD", "AE", "AL", "AT", "AZ", "BA", "BG", "BH", "BI", "BR", "BY",
        "CH", "CR", "CY", "CZ", "DE", "DK", "DJ", "DO", "EE", "EG", "ES",
        "FI", "FK", "FO", "FR", "GB", "GE", "GI", "GL", "GR", "GT", "HN",
        "HR", "HU", "IE", "IL", "IQ", "IS", "IT", "JO", "KW", "KZ", "LB",
        "LC", "LI", "LT", "LU", "LV", "LY", "MC", "MD", "ME", "MK", "MN",
        "MR", "MT", "MU", "NI", "NL", "NO", "OM", "PK", "PL", "PS", "PT",
        "QA", "RO", "RS", "RU", "SA", "SC", "SD", "SE", "SI", "SK", "SM",
        "SO", "ST", "SV", "TL", "TN", "TR", "UA", "VA", "VG", "XK", "YE",
    ]
    out: list[str] = []
    for country in countries:
        structure = _iban_structure(mod, country)
        if not structure:
            continue
        bban = _iban_bban(structure, rng)
        if country in NATIONAL_CHECK_POSITIONS:
            # Solve the national check digits so the base row is accepted, not
            # only the wrong-check row derived from it. Norway's `0000` postgiro
            # prefix takes a different branch the position solver cannot fill,
            # so a draw that lands on it is retried.
            for _ in range(20):
                solved = _iban_solve_national_check(mod, country, bban)
                if solved is not None:
                    bban = solved
                    break
                bban = _iban_bban(structure, rng)
        out.append(country + mod.calc_check_digits(country + "00" + bban) + bban)

        # A letter where the structure requires a digit, with correct MOD-97.
        for index, cls in _structure_classes(structure):
            if cls != "n":
                continue
            broken = bban[:index] + rng.choice("ABCDEFGHIJKLMNOPQRSTUVWXYZ") + bban[index + 1 :]
            out.append(
                country + mod.calc_check_digits(country + "00" + broken) + broken
            )
            break

        # A valid structure and MOD-97 but a wrong national check digit, for
        # the countries whose national check is arithmetic. The reference
        # rejects these with InvalidChecksum; the crate must too.
        if country in NATIONAL_CHECK_POSITIONS:
            wrong = _break_national_check(country, bban, rng)
            out.append(country + mod.calc_check_digits(country + "00" + wrong) + wrong)
    return out


# The BBAN positions holding each country's national check digits. Negative
# indexes count from the end. The set is the countries whose national IBAN
# check is arithmetic; Belgium is absent because its check is a register
# lookup this crate does not ship.
NATIONAL_CHECK_POSITIONS: dict[str, tuple[int, ...]] = {
    "ES": (8, 9),
    "NO": (10,),
    "ME": (-2, -1),
}


def _iban_solve_national_check(mod, country: str, bban: str) -> str | None:
    """Fill in the national check digits so the reference accepts the IBAN.

    The check digits are solved by asking the reference's own `is_valid` for
    the one combination it accepts, rather than by reimplementing each
    country's arithmetic - the reference is the oracle, and a reimplementation
    here could agree with the crate while disagreeing with the reference.
    Returns `None` if no combination is accepted (Norway's postgiro branch).
    """
    indexes = [
        len(bban) + pos if pos < 0 else pos for pos in NATIONAL_CHECK_POSITIONS[country]
    ]
    chars = list(bban)
    for combo in itertools.product("0123456789", repeat=len(indexes)):
        for index, digit in zip(indexes, combo):
            chars[index] = digit
        candidate = "".join(chars)
        iban = country + mod.calc_check_digits(country + "00" + candidate) + candidate
        if mod.is_valid(iban):
            return candidate
    return None


def _break_national_check(country: str, bban: str, rng) -> str:
    """A BBAN identical to `bban` but with its national check digit made wrong.

    The check digits are moved by one (modulo ten) so the change is a real
    error rather than a re-normalisation that could land on the same value.
    """
    if country == "ES":
        index = 8 if rng.random() < 0.5 else 9
    elif country == "NO":
        index = 10
    else:  # ME: the last two digits close the whole BBAN to 1 modulo 97.
        index = len(bban) - 1
    digit = (int(bban[index]) + 1) % 10
    return bban[:index] + str(digit) + bban[index + 1 :]



def _structure_classes(structure: str) -> list[tuple[int, str]]:
    """Every BBAN position with the class the structure requires there."""
    out: list[tuple[int, str]] = []
    pos = 0
    rest = structure
    while rest:
        count, tail = rest.split("!", 1)
        cls = tail[0]
        for _ in range(int(count)):
            out.append((pos, cls))
            pos += 1
        rest = tail[1:]
    return out



def isbn_corpus(mod, rng) -> list[str]:
    """Valid ISBN-10 and ISBN-13 numbers, plus the nine-digit ISBN-10 form.

    A nine-digit ISBN-10 is a printed number whose leading zero was dropped;
    the reference pads it back before checking, and this crate must agree.
    """
    from stdnum import ean

    out: list[str] = []
    for _ in range(10):
        prefix = rng.choice(["978", "979"])
        body = "".join(rng.choice("0123456789") for _ in range(9))
        out.append(prefix + body + ean.calc_check_digit(prefix + body))
        # A valid ISBN-10 that starts with zero, and its nine-digit form.
        head = "0" + "".join(rng.choice("0123456789") for _ in range(8))
        ten = head + mod._calc_isbn10_check_digit(head)
        out.append(ten)
        out.append(ten[1:])
    return out


def ean_corpus(mod, rng) -> list[str]:
    """Valid EAN-8, UPC-A, EAN-13 and GTIN-14 barcodes, one per length."""
    out: list[str] = []
    for length in (8, 12, 13, 14):
        for _ in range(5):
            body = "".join(rng.choice("0123456789") for _ in range(length - 1))
            out.append(body + mod.calc_check_digit(body))
    return out


def luhn_corpus(mod, rng) -> list[str]:
    """Valid Luhn numbers over a range of lengths.

    Lengths start at two: a lone digit is a degenerate number the reference
    accepts only when it is `0` (its checksum of one digit is zero), and this
    crate deliberately requires a payload and a check digit.
    """
    out: list[str] = []
    for _ in range(30):
        length = rng.randint(2, 18)
        body = "".join(rng.choice("0123456789") for _ in range(length - 1))
        out.append(body + mod.calc_check_digit(body))
    return out


CUSTOM_CORPUS: dict[str, object] = {
    "eu.ecnumber": ecnumber_corpus,
    "eu.banknote": banknote_corpus,
    "eu.at_02": sepa_creditor_corpus,
    "eu.eic": eic_corpus,
    "ua.rntrc": rntrc_corpus,
    "ar.cbu": cbu_corpus,
    "si.emso": emso_corpus,
    "es.ccc": ccc_corpus,
    "es.cups": cups_corpus,
    "es.referenciacatastral": cadastral_corpus,
    "es.nif": nif_corpus,
    "fr.rcs": rcs_corpus,
    "de.leitweg": leitweg_corpus,
    "iban": iban_corpus,
    "isbn": isbn_corpus,
    "ean": ean_corpus,
    "luhn": luhn_corpus,
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
