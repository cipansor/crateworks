#!/usr/bin/env python3
"""Round 16: re-measure the open survivors, then sweep seven unswept domains.

The standing lesson of rounds 11-15 is that a gap written down in a document is
not a gap today: MRZ closed between rounds 10 and 11, round 12's YAML headline
had three successors by round 13, round 14's top pick was shipped inside this
workspace before round 15 ran, and round 15 closed two more archives. So this
round does the same two jobs: measure every survivor again, then sweep domains
no earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then seven *utility*
domains. This round deliberately picks seven **specialist protocol and data
format** domains, which those lists under-represent but that entire industries
depend on. They are chosen to fit this workspace's strength, which is
transcribing a real specification and checking the result against a reference:

    healthcare      HL7 v2, FHIR, DICOM
    energy / EV     OCPP charging, Modbus/SunSpec, DLMS/COSEM, IEC 61850
    field buses     CAN/DBC/J1939, ISO-TP, OPC UA
    threat intel    STIX/TAXII, YARA, Sigma, PCAP/Zeek
    supply chain    SPDX and CycloneDX SBOMs, in-toto, sigstore, TUF
    market data     XBRL, FIX, SWIFT MT, NACHA/ACH, SEPA
    identity        OpenPGP, JOSE/JWT, WebAuthn/FIDO2, PKCS#12

Usage:

    python3 round16.py candidates       # re-measure the open survivors
    python3 round16.py sweep            # the seven new domains
    python3 round16.py names <name>...  # is a proposed crate name free?
"""

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

# round15.py guards its main block, so importing it is safe and gives the
# measured helpers (crate, release_date, reverse_deps, pypi, npm, fmt).
from round15 import SLEEP, UA, crate, fmt, get, get_text, npm, pypi, release_date, reverse_deps


def github(repo):
    """Repository facts. Uses GITHUB_TOKEN when present, because the
    unauthenticated API allows only 60 requests an hour and this round makes
    far more than that."""
    if not repo or "github.com" not in repo:
        return None
    slug = repo.rstrip("/").split("github.com/")[-1]
    headers = dict(UA)
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    d = get(f"https://api.github.com/repos/{slug}", headers=headers)
    if not d:
        return None
    return {
        "stars": d.get("stargazers_count"),
        "archived": d.get("archived"),
        "pushed": (d.get("pushed_at") or "")[:10],
    }


def measure(name):
    c = crate(name)
    if c is None:
        return f"  {name:26s} FREE"
    ver, pub = release_date(name)
    rd = reverse_deps(name)
    gh = github(c["repo"])
    star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
    arch = " ARCHIVED" if gh and gh.get("archived") else ""
    return (
        f"  {name:26s} recent={fmt(c['recent']):>12s} v{ver or '?':9s} "
        f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
        f"stars={star}{arch}"
    )


# Every candidate rounds 14 and 15 left open, plus the crates this workspace
# shipped from them, so their own numbers stay visible.
CANDIDATES = {
    "OFX/QIF bank statements (open since round 13)": [
        "ofx-rs", "ofxy", "qif_parser", "ofx-parse", "ofxkit"],
    "subtitles SRT/VTT/ASS (open since round 13)": [
        "subtp", "subparse", "subtitler", "subkit"],
    "PDF table extraction (round 15's strongest)": [
        "pdf-extract", "pdfplumber", "tabula", "pdf_tables", "pdftable",
        "pdf-table-rs", "lopdf"],
    "readability / main content (round 15, a watch)": [
        "dom_smoothie", "readability", "trafilatura", "article-rs", "main-content"],
    "fuzzy matching (shipped here as fuzzy-score)": [
        "fuzzy-matcher", "nucleo-matcher", "fuzzy-score"],
    "immutable collections (a watch)": ["im", "rpds", "ordermap"],
}

# Seven specialist protocol/data domains no earlier round swept. Each group is
# (Rust crates to measure, source packages that show the demand).
SWEEP = {
    "healthcare: HL7 v2, FHIR, DICOM": {
        "rust": ["hl7", "hl7-parser", "hl7-mllp", "fhir", "fhirbolt", "rust-fhir",
                 "fhirpath", "dicom", "dicom-rs", "dicom_object", "dicom-core"],
        "pypi": ["hl7", "hl7apy", "fhir.resources", "fhirpathpy", "pydicom"],
        "npm": ["hl7", "fhir", "dicom-parser", "dcmjs"],
    },
    "energy and EV charging: OCPP, Modbus, DLMS, IEC 61850": {
        "rust": ["ocpp", "rust-ocpp", "ocpp-rs", "openadr", "dlms", "dlms-cosem",
                 "iec61850", "modbus", "tokio-modbus", "sunspec", "modbus-serde"],
        "pypi": ["ocpp", "pymodbus", "dlms-cosem", "pysunspec", "pymodbus3"],
        "npm": ["ocpp", "node-modbus", "modbus-serial", "node-opcua"],
    },
    "field buses: CAN/DBC/J1939, ISO-TP, OPC UA": {
        "rust": ["can", "socketcan", "can-dbc", "dbc", "j1939", "embedded-can",
                 "can-isotp", "opcua", "opcua-client"],
        "pypi": ["cantools", "python-can", "j1939", "opcua", "asyncua"],
        "npm": ["can", "socketcan", "can-dbc", "node-opcua"],
    },
    "threat intelligence: STIX/TAXII, YARA, Sigma, PCAP": {
        "rust": ["stix", "taxii", "yara", "yara-x", "sigma", "pcap", "pcap-parser",
                 "pcap-file", "pnet", "zeek", "cef"],
        "pypi": ["stix2", "taxii2-client", "yara-python", "pysigma", "scapy", "dpkt"],
        "npm": ["stix", "taxii", "yara", "pcap", "pcap-parser"],
    },
    "software supply chain: SBOM, in-toto, sigstore, TUF": {
        "rust": ["spdx", "spdx-rs", "cyclonedx-bom", "cyclonedx", "cargo-sbom",
                 "in-toto", "sigstore", "tough", "tuf", "sigstore-rs", "sbom"],
        "pypi": ["spdx-tools", "cyclonedx-python-lib", "in-toto", "sigstore", "tuf"],
        "npm": ["spdx", "cyclonedx", "sigstore", "@sigstore/tuf", "tuf-js"],
    },
    "market data: XBRL, FIX, SWIFT MT, NACHA/ACH": {
        "rust": ["xbrl", "xbrl-parser", "fix", "fix-rs", "quickfix-rs", "fefix",
                 "swift-mt", "swift", "nacha", "ach", "iso8583-msg"],
        "pypi": ["python-xbrl", "arelle", "quickfix", "swift", "iso8583", "nacha"],
        "npm": ["xbrl", "fixparser", "swift", "ach"],
    },
    "identity: OpenPGP, JOSE/JWT, WebAuthn/FIDO2, PKCS#12": {
        "rust": ["openpgp", "sequoia-openpgp", "pgp", "jwt", "jsonwebtoken", "josekit",
                 "jose", "webauthn", "webauthn-rs", "fido2", "pkcs12", "p12",
                 "openid", "openidconnect"],
        "pypi": ["python-jose", "authlib", "fido2", "webauthn", "python-pkcs11"],
        "npm": ["jose", "jsonwebtoken", "fido2-lib", "node-openpgp", "pkijs"],
    },
}


def show_candidates():
    for area, names in CANDIDATES.items():
        print(f"\n### {area}")
        for n in names:
            print(measure(n))
            time.sleep(SLEEP)


def show_sweep():
    for area, groups in SWEEP.items():
        print(f"\n### {area}")
        for n in groups["rust"]:
            print(measure(n))
            time.sleep(SLEEP)
        for n in groups["pypi"]:
            print(f"  [pypi] {n:20s} {fmt(pypi(n))}/month")
            time.sleep(0.3)
        for n in groups["npm"]:
            print(f"  [npm ] {n:20s} {fmt(npm(n))}/week")
            time.sleep(0.3)


def show_names():
    for n in sys.argv[2:]:
        c = crate(n)
        print(f"  {n:26s} {'FREE' if c is None else 'TAKEN recent=' + fmt(c['recent'])}")
        time.sleep(SLEEP)


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "candidates"
    if cmd == "candidates":
        show_candidates()
    elif cmd == "sweep":
        show_sweep()
    elif cmd == "names":
        show_names()
    else:
        raise SystemExit(f"unknown command {cmd!r}")
