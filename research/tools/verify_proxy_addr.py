#!/usr/bin/env python3
"""Differential gate: `crates/proxy-addr` against the npm reference.

The crate replays the committed corpus in `tests/oracle.rs`, which is what runs
in CI. This script is the other half: it generates a *fresh* corpus with
`scripts/fuzz/proxy-addr/gen.js` (which drives npm `proxy-addr`), then builds a
small Rust driver against the crate and compares the two over every line at
once, so the port and the fixtures cannot drift apart. A disagreement is
reported with the failing input.

    python3 research/tools/verify_proxy_addr.py
    python3 research/tools/verify_proxy_addr.py --cases 200000

Needs `npm install proxy-addr` in the repository root and a Rust toolchain. The
driver is built in a temporary directory, so nothing is written into the crate.
`--seed` chooses a different corpus; the default is the one the committed
fixture was generated with.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "proxy-addr"
GENERATOR = ROOT / "scripts" / "fuzz" / "proxy-addr" / "gen.js"

DRIVER = r"""
use std::net::IpAddr;
use proxy_addr::{addr, all, forwarded, Error, Trust};

fn unescape(field: &str) -> String {
    if field == "\\N" {
        return String::new();
    }
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some('x') if chars.next() == Some('2') && chars.next() == Some('0') => out.push(' '),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Escape a value the way the generator does, so a value that itself contains a
/// tab is not split into two columns by the gate's line protocol.
fn escape(s: &str) -> String {
    let mut out = s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n");
    if out.ends_with(' ') {
        out.truncate(out.len() - 1);
        out.push_str("\\x20");
    }
    out
}

fn error_text(e: &Error) -> String {
    match e {
        Error::Missing => "argument is required".to_string(),
        Error::InvalidAddress(a) => format!("invalid IP address: {a}"),
        Error::InvalidRange(n) => format!("invalid range on address: {n}"),
        other => format!("{other}"),
    }
}

fn main() {
    for line in include_str!("oracle.tsv").lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        // `raw` keeps the fixture's own text for the identifying columns, so
        // the gate can match this output back to the reference's line; `fields`
        // is the unescaped form the crate is called with.
        let raw: Vec<&str> = line.split('\t').collect();
        let fields: Vec<String> = raw.iter().map(|f| unescape(f)).collect();
        let tag = raw[0];
        let peer: IpAddr = fields[1].parse().expect("peer parses");
        let header = if raw[2] == "\\N" { None } else { Some(fields[2].as_str()) };

        match tag {
            "FWD" => {
                let got: Vec<String> = forwarded(peer, header).iter().map(|a| escape(a)).collect();
                println!("FWD\t{}\t{}\t{}", raw[1], raw[2], got.join("\t"));
            }
            "ALL" | "ADDR" => {
                let specs: Vec<&str> = fields[3].split('|').collect();
                let tail = fields[4].as_str();
                let compiled = Trust::parse_list(specs.iter().copied());
                // Escape each value the way the generator does, so a value that
                // itself contains a tab cannot be split into two columns.
                let text = if tail.starts_with('!') {
                    match compiled {
                        Ok(_) => "!accepted".to_string(),
                        Err(e) => format!("!{}", escape(&error_text(&e))),
                    }
                } else {
                    let trust = compiled.expect("reference accepted this trust spec");
                    if tag == "ALL" {
                        all(peer, header, Some(&trust))
                            .iter()
                            .map(|a| escape(a))
                            .collect::<Vec<_>>()
                            .join("\t")
                    } else {
                        escape(&addr(peer, header, &trust))
                    }
                };
                println!("{tag}\t{}\t{}\t{}\t{}", raw[1], raw[2], raw[3], text);
            }
            other => panic!("unknown tag {other}"),
        }
    }
}
"""


def generate(cases: int, seed: int, out: Path) -> None:
    env = dict(os.environ)
    env["CASES"] = str(cases)
    env["SEED"] = str(seed)
    env["OUT"] = str(out)
    subprocess.run(["node", str(GENERATOR)], check=True, env=env, cwd=ROOT)


def run_driver(fixture: Path) -> list[list[str]]:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        (tmp / "oracle.tsv").write_text(fixture.read_text())
        (tmp / "main.rs").write_text(DRIVER)
        (tmp / "Cargo.toml").write_text(
            "[package]\n"
            'name = "verify-proxy-addr"\n'
            'version = "0.0.0"\n'
            'edition = "2021"\n\n'
            "[dependencies]\n"
            f'proxy-addr = {{ path = "{CRATE}" }}\n\n'
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=int, default=6000, help="random cases per tag")
    parser.add_argument("--seed", type=int, default=0x9E3779B9, help="PRNG seed")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as tmp:
        fixture = Path(tmp) / "oracle.tsv"
        generate(args.cases, args.seed, fixture)
        oracle = {}
        for line in fixture.read_text().splitlines():
            if line.startswith("#") or not line.strip():
                continue
            parts = line.split("\t")
            # Key by the tag and the input fields, so the driver's output can be
            # matched back to the reference's line.
            key = (parts[0], parts[1], parts[2]) if parts[0] == "FWD" \
                else (parts[0], parts[1], parts[2], parts[3])
            oracle[key] = parts
        lines = run_driver(fixture)

    problems: list[str] = []
    total = 0
    for parts in lines:
        tag = parts[0]
        total += 1
        if tag == "FWD":
            key = (tag, parts[1], parts[2])
            want = oracle.get(key)
            if want is None:
                problems.append(f"no oracle row for {key}")
                continue
            got = parts[3:]
            expected = want[3:]
            if got != expected:
                problems.append(f"FWD {parts[1]} {parts[2]!r}: {got} vs {expected}")
        else:
            key = (tag, parts[1], parts[2], parts[3])
            want = oracle.get(key)
            if want is None:
                problems.append(f"no oracle row for {key}")
                continue
            got = parts[4]
            expected = want[4]
            if got != expected:
                problems.append(
                    f"{tag} peer={parts[1]} xff={parts[2]!r} trust={parts[3]!r}: "
                    f"{got!r} vs {expected!r}"
                )

    if problems:
        for problem in problems[:40]:
            print(problem, file=sys.stderr)
        print(f"\n{len(problems)} of {total} cases disagree with npm proxy-addr", file=sys.stderr)
        return 1
    print(f"{total} cases agree with npm proxy-addr (seed {args.seed})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
