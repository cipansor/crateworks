#!/usr/bin/env python3
"""Differential gate: the aviation-data table against the upstream OurAirports CSV.

The crate ships a compiled table. The only way to know it is right is to rebuild
the expected table from the upstream CSV and compare, field by field, against
what the crate actually returns. This script does both sides:

  1. it downloads the upstream CSV and applies the generator's filter;
  2. it builds a small Rust driver that reads the crate's public API and prints
     every airport it knows about, and every code lookup;
  3. it diffs the two.

A row that the crate has and upstream does not, a field that differs, a code
that resolves to the wrong airport - each is reported. The script exits non-zero
when anything disagrees, so it can be a CI gate.

    python3 research/tools/verify_aviation.py

Needs a Rust toolchain on PATH and network access for the CSV. The Rust driver
is compiled in a temporary directory, so nothing is written into the crate.
"""

import csv
import io
import json
import os
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "aviation-data"
CSV_URL = "https://davidmegginson.github.io/ourairports-data/airports.csv"

FIELDS = [
    "ident", "kind", "name", "lat", "lon", "elevation_ft", "continent",
    "country", "region", "municipality", "scheduled", "iata", "icao", "gps", "local",
]


def clean(value: str) -> str:
    return value.replace("\t", " ").replace("\r", " ").replace("\n", " ").strip()


def expected_table() -> list[list[str]]:
    req = urllib.request.Request(CSV_URL, headers={"User-Agent": "crateworks-verify"})
    with urllib.request.urlopen(req, timeout=120) as r:
        rows = list(csv.DictReader(io.StringIO(r.read().decode("utf-8"))))
    out = []
    for r in rows:
        if clean(r.get("type", "")) == "closed":
            continue
        if not clean(r.get("iata_code", "")) and not clean(r.get("icao_code", "")):
            continue
        out.append([
            clean(r.get("ident", "")), clean(r.get("type", "")), clean(r.get("name", "")),
            clean(r.get("latitude_deg", "")), clean(r.get("longitude_deg", "")),
            clean(r.get("elevation_ft", "")), clean(r.get("continent", "")),
            clean(r.get("iso_country", "")), clean(r.get("iso_region", "")),
            clean(r.get("municipality", "")), clean(r.get("scheduled_service", "")),
            clean(r.get("iata_code", "")), clean(r.get("icao_code", "")),
            clean(r.get("gps_code", "")), clean(r.get("local_code", "")),
        ])
    out.sort(key=lambda rec: rec[0])
    return out


DRIVER = r"""
use aviation_data::{all, find};

fn main() {
    for a in all() {
        let e = a.elevation_ft().map(|v| v.to_string()).unwrap_or_default();
        println!(
            "ROW\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            a.ident(), a.kind().as_str(), a.name(),
            a.latitude(), a.longitude(), e,
            a.continent(), a.country(), a.region(),
            a.municipality().unwrap_or(""), if a.has_scheduled_service() {"yes"} else {"no"},
            a.iata().unwrap_or(""), a.icao().unwrap_or(""),
            a.gps().unwrap_or(""), a.local().unwrap_or(""),
        );
    }
    // Every code in every space must resolve back to the airport that owns it.
    for a in all() {
        for code in [
            Some(a.ident()), a.iata(), a.icao(), a.gps(), a.local(),
        ] {
            if let Some(code) = code {
                let hit = find(code).expect("code must resolve");
                println!("LOOKUP\t{}\t{}", code, hit.ident());
            }
        }
    }
}
"""


def run_driver() -> tuple[list[list[str]], list[tuple[str, str]]]:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        (tmp / "main.rs").write_text(DRIVER)
        exe = tmp / "driver"
        # Link the crate as a dependency through a throwaway manifest, so the
        # driver uses the real public API rather than a copy of the logic.
        (tmp / "Cargo.toml").write_text(
            "[package]\n"
            'name = "verify-aviation"\n'
            'version = "0.0.0"\n'
            'edition = "2021"\n\n'
            "[dependencies]\n"
            f'aviation-data = {{ path = "{CRATE}" }}\n\n'
            "[[bin]]\n"
            'name = "driver"\n'
            'path = "main.rs"\n'
        )
        env = dict(os.environ)
        env.setdefault("CARGO_TARGET_DIR", str(tmp / "target"))
        build = subprocess.run(
            ["cargo", "build", "--quiet", "--manifest-path", str(tmp / "Cargo.toml")],
            capture_output=True, text=True, env=env,
        )
        if build.returncode != 0:
            sys.stderr.write(build.stderr)
            raise SystemExit("the driver failed to build")
        out = subprocess.run(
            [str(tmp / "target" / "debug" / "driver")],
            capture_output=True, text=True, check=True,
        ).stdout

    rows, lookups = [], []
    for line in out.splitlines():
        parts = line.split("\t")
        if parts[0] == "ROW":
            rows.append(parts[1:])
        elif parts[0] == "LOOKUP":
            lookups.append((parts[1], parts[2]))
    return rows, lookups


def main() -> int:
    want = expected_table()
    got, lookups = run_driver()
    problems = []

    if len(got) != len(want):
        problems.append(f"row count: crate {len(got)}, upstream {len(want)}")

    # Compare as dictionaries keyed by ident, so a reordering is not a defect.
    want_by = {rec[0]: rec for rec in want}
    got_by = {rec[0]: rec for rec in got}
    for ident in sorted(set(want_by) | set(got_by)):
        if ident not in got_by:
            problems.append(f"{ident}: in upstream, missing from the crate")
            continue
        if ident not in want_by:
            problems.append(f"{ident}: in the crate, not in upstream")
            continue
        for i, name in enumerate(FIELDS):
            g, w = got_by[ident][i], want_by[ident][i]
            if name in ("lat", "lon"):
                # Rust and Python both print the shortest string that round-trips
                # to the double, but they can disagree on the last digit while
                # producing the same double. Compare the parsed value, which is
                # exact, rather than the text.
                if float(g) != float(w):
                    problems.append(f"{ident}.{name}: crate {g}, upstream {w}")
            elif g != w:
                problems.append(f"{ident}.{name}: crate {g!r}, upstream {w!r}")

    # Code lookup: the crate must resolve every code the way the documented
    # precedence says - IATA, then ICAO, then GPS, then local, then `ident`,
    # with the first airport in ident order winning within a space. The order is
    # not arbitrary: 35 rows carry an `ident` that is another row's code, so a
    # wrong precedence returns the wrong airport for codes like `WABK`.
    spaces = ["iata", "icao", "gps", "local", "ident"]
    want_lookup: dict[str, str] = {}
    for space in spaces:
        for rec in want:
            code = rec[FIELDS.index(space)]
            if code and code not in want_lookup:
                want_lookup[code] = rec[0]
    got_lookup = dict(lookups)
    for code in sorted(want_lookup):
        if code not in got_lookup:
            problems.append(f"lookup {code!r}: expected {want_lookup[code]!r}, crate found nothing")
        elif got_lookup[code] != want_lookup[code]:
            problems.append(
                f"lookup {code!r}: crate {got_lookup[code]!r}, expected {want_lookup[code]!r}"
            )
    # Whatever a code resolves to must actually carry that code. This holds
    # regardless of which precedence is right, so it catches a lookup that
    # matches on a field the airport does not publish.
    for code, ident in got_lookup.items():
        rec = want_by.get(ident)
        if rec is None:
            problems.append(f"lookup {code!r} resolved to unknown airport {ident!r}")
        elif code not in (rec[0], rec[11], rec[12], rec[13], rec[14]):
            problems.append(f"lookup {code!r} -> {ident!r}, which does not carry that code")

    print(f"crate rows      {len(got)}")
    print(f"upstream rows   {len(want)}")
    print(f"code lookups    {len(lookups)} (of {len(want_lookup)} codes in the table)")
    if problems:
        print(f"\n{len(problems)} disagreement(s):")
        for p in problems[:40]:
            print(f"  {p}")
        if len(problems) > 40:
            print(f"  ... and {len(problems) - 40} more")
        return 1
    print("\nall rows and lookups agree with the upstream CSV")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
