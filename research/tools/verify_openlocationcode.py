#!/usr/bin/env python3
"""Differential gate: `crates/openlocationcode` against the reference.

The crate replays the reference's own test data in `tests/reference.rs`, which
is what runs in CI. This script is the other half: it drives the crate's public
API and the Python reference over the same fixtures at once and diffs the two,
so the port and the fixtures cannot drift apart, and it can also regenerate the
fixtures from the reference.

    python3 research/tools/verify_openlocationcode.py
    python3 research/tools/verify_openlocationcode.py --update-fixtures

Needs `pip install openlocationcode` and a Rust toolchain. The Rust driver is
built in a temporary directory, so nothing is written into the crate.

The four fixtures are the reference's `test_data/*.csv`, copied unchanged. Two
comparisons are exact and two carry the tolerance the reference itself
documents:

* `encode_integers` is exact, so its column is compared exactly;
* `encode` from degrees converts floats to integers and the reference accepts a
  5% mismatch rate for it, so this script does too;
* `location_to_integers` may land one below the expected integer;
* decoding is compared to 1e-10, as the reference's own test is.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "openlocationcode"
FIXTURES = CRATE / "tests" / "fixtures"
UPSTREAM = "https://raw.githubusercontent.com/google/open-location-code/main/test_data"
FILES = ["encoding.csv", "decoding.csv", "validityTests.csv", "shortCodeTests.csv"]

DRIVER = r"""
use openlocationcode::{
    decode, encode, encode_integers, is_full, is_short, is_valid, location_to_integers,
    recover_nearest, shorten,
};

fn rows(csv: &str) -> Vec<Vec<String>> {
    csv.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect()
}

fn main() {
    for row in rows(include_str!("encoding.csv")) {
        let lat: f64 = row[0].parse().unwrap();
        let lng: f64 = row[1].parse().unwrap();
        let lati: i64 = row[2].parse().unwrap();
        let lngi: i64 = row[3].parse().unwrap();
        let len: usize = row[4].parse().unwrap();
        let (gli, gni) = location_to_integers(lat, lng);
        println!("INT\t{lat}\t{lng}\t{gli}\t{gni}\t{lati}\t{lngi}");
        println!("ENCINT\t{lati}\t{lngi}\t{len}\t{}", encode_integers(lati, lngi, len).unwrap());
        println!("ENC\t{lat}\t{lng}\t{len}\t{}", encode(lat, lng, len).unwrap());
    }
    for row in rows(include_str!("decoding.csv")) {
        let a = decode(&row[0]).unwrap();
        println!(
            "DEC\t{}\t{}\t{:.12}\t{:.12}\t{:.12}\t{:.12}",
            row[0], a.code_length, a.latitude_low, a.longitude_low,
            a.latitude_high, a.longitude_high
        );
    }
    for row in rows(include_str!("validityTests.csv")) {
        println!(
            "VAL\t{}\t{}\t{}\t{}",
            row[0], is_valid(&row[0]), is_short(&row[0]), is_full(&row[0])
        );
    }
    for row in rows(include_str!("shortCodeTests.csv")) {
        let lat: f64 = row[1].parse().unwrap();
        let lng: f64 = row[2].parse().unwrap();
        println!("SHORT\t{}\t{lat}\t{lng}\t{}", row[0], shorten(&row[0], lat, lng).unwrap());
        println!("RECOVER\t{}\t{lat}\t{lng}\t{}", row[3], recover_nearest(&row[3], lat, lng).unwrap());
    }
}
"""


def run_driver() -> list[list[str]]:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        # The driver reads the fixtures through include_str!, so it is built
        # beside copies of them.
        for name in FILES:
            shutil.copy(FIXTURES / name, tmp / name)
        (tmp / "main.rs").write_text(DRIVER)
        (tmp / "Cargo.toml").write_text(
            "[package]\n"
            'name = "verify-olc"\n'
            'version = "0.0.0"\n'
            'edition = "2021"\n\n'
            "[dependencies]\n"
            f'openlocationcode = {{ path = "{CRATE}" }}\n\n'
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
    return [line.split("\t") for line in out.splitlines() if line]


def update_fixtures() -> int:
    import urllib.request

    for name in FILES:
        url = f"{UPSTREAM}/{name}"
        req = urllib.request.Request(url, headers={"User-Agent": "crateworks-verify"})
        with urllib.request.urlopen(req, timeout=60) as r:
            (FIXTURES / name).write_bytes(r.read())
        print(f"updated {name}")
    return 0


def main() -> int:
    if "--update-fixtures" in sys.argv:
        return update_fixtures()

    try:
        from openlocationcode import openlocationcode as olc
    except ImportError:
        print("needs the reference: pip install openlocationcode", file=sys.stderr)
        return 1

    lines = run_driver()
    problems: list[str] = []
    enc_total = enc_bad = 0
    # The two test-only functions (location_to_integers, encode_integers) are
    # not in the published PyPI package, so their oracle is the fixture's own
    # column, which came from the reference's test data.

    for parts in lines:
        kind = parts[0]
        if kind == "INT":
            lat, lng = float(parts[1]), float(parts[2])
            gli, gni = int(parts[3]), int(parts[4])
            lati, lngi = int(parts[5]), int(parts[6])
            if not lati - 1 <= gli <= lati:
                problems.append(f"locationToIntegers lat {lat}: {gli}, fixture {lati}")
            if not lngi - 1 <= gni <= lngi:
                problems.append(f"locationToIntegers lng {lng}: {gni}, fixture {lngi}")
        elif kind == "ENCINT":
            lati, lngi, length = int(parts[1]), int(parts[2]), int(parts[3])
            got = parts[4]
            # Compare against the fixture column, which the reference produced.
            row = _find_encoding(lati, lngi, length)
            if row is not None and got != row[5]:
                problems.append(f"encodeIntegers({lati},{lngi},{length}): {got} vs {row[5]}")
        elif kind == "ENC":
            lat, lng, length = float(parts[1]), float(parts[2]), int(parts[3])
            got = parts[4]
            want = olc.encode(lat, lng, length)
            enc_total += 1
            if got != want:
                enc_bad += 1
        elif kind == "DEC":
            code = parts[1]
            got = [float(parts[3]), float(parts[4]), float(parts[5]), float(parts[6])]
            area = olc.decode(code)
            want = [area.latitudeLo, area.longitudeLo, area.latitudeHi, area.longitudeHi]
            for i, name in enumerate(["south", "west", "north", "east"]):
                if abs(got[i] - want[i]) > 1e-10:
                    problems.append(f"decode {code} {name}: {got[i]} vs {want[i]}")
            if int(parts[2]) != area.codeLength:
                problems.append(f"decode {code} length: {parts[2]} vs {area.codeLength}")
        elif kind == "VAL":
            code = parts[1]
            for i, fn in enumerate([olc.isValid, olc.isShort, olc.isFull], start=2):
                if (parts[i] == "true") != fn(code):
                    problems.append(f"{fn.__name__} {code}: crate {parts[i]}")
        elif kind == "SHORT":
            code, lat, lng, got = parts[1], float(parts[2]), float(parts[3]), parts[4]
            want = olc.shorten(code, lat, lng)
            if got != want:
                problems.append(f"shorten {code}: {got} vs {want}")
        elif kind == "RECOVER":
            short, lat, lng, got = parts[1], float(parts[2]), float(parts[3]), parts[4]
            want = olc.recoverNearest(short, lat, lng)
            if got != want:
                problems.append(f"recoverNearest {short}: {got} vs {want}")

    if enc_total and enc_bad / enc_total > 0.05:
        problems.append(f"encode mismatch rate {enc_bad}/{enc_total} is too high")

    if problems:
        for p in problems[:40]:
            print(p, file=sys.stderr)
        print(f"{len(problems)} disagreement(s)", file=sys.stderr)
        return 1
    print(f"openlocationcode agrees with the reference on {len(lines)} rows")
    return 0


def _encoding_rows():
    for line in (FIXTURES / "encoding.csv").read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        yield line.split(",")


def _find_encoding(lat_int: int, lng_int: int, length: int):
    for row in _encoding_rows():
        if int(row[2]) == lat_int and int(row[3]) == lng_int and int(row[4]) == length:
            return row
    return None


if __name__ == "__main__":
    sys.exit(main())
