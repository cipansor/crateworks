"""Round 15: re-measure the survivors, and sweep seven unswept domains.

The standing lesson of rounds 11-14 is that a gap recorded in a document is not a
gap today: MRZ closed between rounds 10 and 11, the YAML opening round 12 called
the largest in twelve rounds had three successors by round 13, and round 14's own
top pick (`fuzzy-matcher`) was shipped inside this workspace as `fuzzy-score`
before this round ran. So this round starts by measuring every survivor again,
then sweeps domains no earlier round touched.

Every earlier round searched the npm and PyPI popularity lists, which are
dominated by web frameworks and data-science packages. This round deliberately
picks seven *utility* domains that those lists under-represent but that real
applications depend on: human-readable formatting, natural-language dates,
typography, text repair, MIME sniffing, web feeds, and geo hashing.

Usage:

    python3 round15.py candidates     # re-measure the open survivors
    python3 round15.py sweep          # the seven new domains
    python3 round15.py names <name>...  # is a proposed crate name free?
"""

import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

UA = {"User-Agent": "gap-research (rust gap analysis)"}
SLEEP = 0.7  # crates.io rate-limits; a 429 comes back as an empty result


def get(url, headers=None):
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=headers or UA)
            return json.load(urllib.request.urlopen(req, timeout=30))
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code == 429:
                time.sleep(5 + attempt * 5)
                continue
            return None
        except Exception:
            time.sleep(2 + attempt * 2)
    return None


def get_text(url, headers=None):
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=headers or UA)
            return urllib.request.urlopen(req, timeout=30).read().decode("utf-8")
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code == 429:
                time.sleep(5 + attempt * 5)
                continue
            return None
        except Exception:
            time.sleep(2 + attempt * 2)
    return None


def crate(name):
    """Exact crate lookup. Returns None when the name is free."""
    d = get("https://crates.io/api/v1/crates/" + urllib.parse.quote(name))
    if not d or not d.get("crate"):
        return None
    c = d["crate"]
    return {
        "name": c["name"],
        "recent": c.get("recent_downloads") or 0,
        "version": c.get("max_stable_version") or c.get("newest_version"),
        "desc": (c.get("description") or "")[:70],
        "repo": c.get("repository"),
    }


def release_date(name):
    """Publish date of the newest non-yanked version, from the sparse index.

    The API's `updated_at` moves on a yank or an owner edit, so it is not a
    release date. The index is. The index path is 1/<n>, 2/<n>, 3/<c>/<n> or
    <ab>/<cd>/<n>, and the field is `pubtime`, not `pubdate`.
    """
    q = urllib.parse.quote(name.lower())
    if len(name) == 1:
        path = f"1/{q}"
    elif len(name) == 2:
        path = f"2/{q}"
    elif len(name) == 3:
        path = f"3/{q[0]}/{q}"
    else:
        path = f"{q[:2]}/{q[2:4]}/{q}"
    raw = get_text(f"https://index.crates.io/{path}")
    if raw is None:
        return None, None
    latest = None
    for line in raw.splitlines():
        try:
            v = json.loads(line)
        except Exception:
            continue
        if v.get("yanked"):
            continue
        latest = v
    return (latest or {}).get("vers"), ((latest or {}).get("pubtime") or "")[:10]


def reverse_deps(name):
    d = get(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}/reverse_dependencies")
    if not d:
        return None
    return d.get("meta", {}).get("total")


def github(repo):
    if not repo or "github.com" not in repo:
        return None
    slug = repo.rstrip("/").split("github.com/")[-1]
    d = get(f"https://api.github.com/repos/{slug}")
    if not d:
        return None
    return {"stars": d.get("stargazers_count"), "archived": d.get("archived"),
            "pushed": (d.get("pushed_at") or "")[:10]}


def pypi(name):
    d = get(f"https://pypistats.org/api/packages/{urllib.parse.quote(name)}/recent")
    if not d:
        return None
    return d.get("data", {}).get("last_month")


def npm(name):
    d = get(f"https://api.npmjs.org/downloads/point/last-week/{urllib.parse.quote(name)}")
    return (d or {}).get("downloads")


def fmt(n):
    return "-" if n is None else f"{n:,}"


# ---------------------------------------------------------------- candidates

# Every candidate rounds 12-14 left open, plus the crate this workspace shipped
# from round 14 so its own numbers stay visible.
CANDIDATES = {
    "immutable collections": ["im", "im-rc", "rpds", "ordermap"],
    "fuzzy matching (shipped here as fuzzy-score)": [
        "fuzzy-matcher", "nucleo-matcher", "simsearch", "fuzzy-score"],
    "small string": ["smartstring", "compact_str", "ecow"],
    "multipart": ["multipart", "multer", "formdata"],
    "pem": ["rustls-pemfile", "pem", "pem-rfc7468"],
    "OFX/QIF bank statements": ["ofx-rs", "ofxy", "qif_parser", "ofx-parse", "ofxkit"],
    "subtitles": ["subtp", "subparse", "subtitler", "subkit"],
}

# Seven utility domains no earlier round swept. Each pair is
# (Rust crate to measure, source package that shows the demand).
SWEEP = {
    "human-readable formatting": {
        "rust": ["humanize", "num-format", "num2words", "num2en", "pretty-bytes",
                 "bytesize", "humansize", "human_bytes", "filesize", "humanize-duration",
                 "duration-str", "parse-size"],
        "pypi": ["num2words", "humanize", "humanfriendly"],
        "npm": ["pretty-bytes", "humanize-duration", "filesize", "bytes"],
    },
    "natural-language dates": {
        "rust": ["chrono-english", "dateparser", "dateparser-rs", "natural-date",
                 "human-date-parser", "parse_datetime"],
        "pypi": ["dateparser", "parsedatetime", "dateutil"],
        "npm": ["chrono-node", "date-fns", "luxon"],
    },
    "typography and hyphenation": {
        "rust": ["hyphenation", "hypher", "textwrap", "smartypants", "typst"],
        "pypi": ["pyphen", "textwrap3", "smartypants"],
        "npm": ["hypher", "smartypants"],
    },
    "text repair and transliteration": {
        "rust": ["ftfy", "unidecode", "deunicode", "slug", "slugify", "ammonia",
                 "unicode-normalization", "encoding_rs", "chardetng", "charset"],
        "pypi": ["ftfy", "Unidecode", "python-slugify", "chardet", "charset-normalizer",
                 "bleach"],
        "npm": ["slugify", "unidecode", "sanitize-html", "dompurify", "iconv-lite"],
    },
    "MIME and file-type sniffing": {
        "rust": ["infer", "mime", "mime_guess", "tree_magic_mini", "file-format",
                 "magic", "content_inspector"],
        "pypi": ["python-magic", "filetype", "mimetypes"],
        "npm": ["file-type", "mime-types", "mime"],
    },
    "web feeds and crawler policy": {
        "rust": ["feed-rs", "rss", "atom_syndication", "quick-xml", "sitemap",
                 "robotstxt", "texting_robots", "robots-txt"],
        "pypi": ["feedparser", "atoma", "ultimate-sitemap-parser", "reppy", "robotexclusionrulesparser"],
        "npm": ["rss-parser", "feedparser", "sitemap", "robots-parser"],
    },
    "geo hashing and spatial indexing": {
        "rust": ["geohash", "geohash2", "h3o", "s2", "s2-geometry", "geo", "h3ron"],
        "pypi": ["python-geohash", "h3", "s2sphere", "geohash2"],
        "npm": ["ngeohash", "h3-js", "s2-geometry"],
    },
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
    return (f"  {name:26s} recent={fmt(c['recent']):>12s} v{ver or '?':9s} "
            f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
            f"stars={star}{arch}")


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
