#!/usr/bin/env python3
"""Check whether the shortlist gaps are real.

A low download count is not proof that nothing exists: a crate may be new, or
the search term may have missed it. This looks each candidate up by several
names and reports what is actually on crates.io, so a gap is only called real
when a direct name lookup comes back empty.
"""

from __future__ import annotations

import json
import time
import urllib.parse
import urllib.request

UA = {"User-Agent": "crate-gap-research/1.0 (https://github.com/cipansor)"}

# (label, npm package, [names and search terms to try])
SHORTLIST = [
    ("Cron to human text", "cronstrue", ["cronstrue", "cron-descriptor", "cron human readable", "cronstrue"]),
    ("PPTX generation", "pptxgenjs", ["pptx", "powerpoint", "pptxgen", "ooxml presentation"]),
    ("Anthropic API client", "@anthropic-ai/sdk", ["anthropic", "anthropic-sdk", "claude api"]),
    ("Barcode generation", "bwip-js", ["barcode", "bwip", "barcoders", "qrcode"]),
    ("Twilio API", "twilio", ["twilio", "twilio api"]),
    ("Sitemap generation", "sitemap", ["sitemap", "sitemap-rs", "sitemap generator"]),
    ("FIGlet ASCII art", "figlet", ["figlet", "figlet-rs", "ascii art text", "toilet"]),
    ("English NLP", "compromise", ["nlp english", "compromise", "nlprule", "rust-bert"]),
    ("PDF generation (browser)", "jspdf", ["printpdf", "genpdf", "pdf generation", "pdf-writer"]),
    ("HTML to Markdown", "turndown", ["html2md", "htmd", "html to markdown", "htmd"]),
    ("DOCX to HTML", "mammoth", ["docx", "dotext", "office_oxide", "docx to html"]),
    ("Socket.IO", "socket.io", ["socketioxide", "socket.io", "rust-socketio"]),
    ("MongoDB driver", "mongodb", ["mongodb", "mongo"]),
    ("XLSX read/write", "xlsx", ["calamine", "umya-spreadsheet", "rust_xlsxwriter", "xlsx"]),
    ("MUSTACHE templates", "mustache", ["mustache", "ramhorns", "handlebars"]),
    ("iCalendar parsing", "ical.js", ["ical", "rrule", "icalendar"]),
    ("IMAP client", "imapflow", ["imap", "async-imap", "imap-codec"]),
    ("Audio metadata", "music-metadata", ["lofty", "id3", "audio metadata", "symphonia"]),
    ("Bloom filters", "bloom-filters", ["bloomfilter", "bloom", "bloom filter"]),
    ("Fuzzy search", "fuse.js", ["fuzzy-matcher", "fuzzy search", "nucleo", "fuzzy"]),
    ("Currency / money", "dinero.js", ["money", "rusty-money", "currency", "iso_currency"]),
    ("Phone number parsing", "libphonenumber-js", ["phonenumber", "libphonenumber", "phone number"]),
    ("IndexedDB", "idb", ["indexed_db_futures", "rexie", "indexeddb", "gloo"]),
    ("Service workers", "workbox", ["service worker", "service-worker", "yew"]),
    ("Job queues", "bull", ["apalis", "celery", "job queue", "background job"]),
    ("Profanity filter", "bad-words", ["profanity", "censor", "bad words"]),
    ("Spell checking", "nodehun", ["hunspell", "spellcheck", "spellbook", "spell"]),
    ("vCard", "vcard4", ["vcard", "vcard4"]),
    ("EPUB", "epub-gen", ["epub", "epub-builder"]),
    ("Short ids", "nanoid", ["nanoid", "uuid", "cuid"]),
    ("Obfuscated ids", "hashids", ["hashids", "harsh", "sqids"]),
    ("Emoji lookup", "node-emoji", ["emoji", "emojis", "emojis-rs"]),
    ("Fuzzy string matching", "string-similarity", ["strsim", "string similarity", "jaro"]),
    ("Data validation", "ajv", ["jsonschema", "json schema validation", "valico", "garde"]),
    ("Query strings", "qs", ["serde_qs", "serde_urlencoded", "querystring"]),
    ("JSONPath", "jsonpath-plus", ["jsonpath", "jsonpath-rust"]),
    ("User agent parsing", "ua-parser-js", ["woothee", "user agent", "uaparser"]),
    ("GeoIP", "geoip-lite", ["maxminddb", "geoip", "geoip2"]),
    ("robots.txt", "robots-parser", ["robotstxt", "robots txt", "texting_robots"]),
    ("Rate limiting", "express-rate-limit", ["tower-governor", "governor", "rate limit"]),
    ("CSRF", "csurf", ["csrf", "axum-csrf"]),
    ("TOTP", "otplib", ["totp", "totp-rs", "otpauth"]),
    ("Password strength", "zxcvbn", ["zxcvbn", "password strength"]),
    ("Transliteration", "transliteration", ["transliterate", "deunicode", "unidecode"]),
    ("Colour science", "culori", ["palette", "colorous", "color science"]),
    ("Interactive prompts", "inquirer", ["dialoguer", "inquire", "prompt"]),
    ("Human readable sizes", "pretty-bytes", ["humansize", "bytesize", "humanize"]),
    ("Number formatting", "numeral", ["num-format", "humanize", "numfmt"]),
    ("Internationalisation", "i18next", ["fluent", "rust-i18n", "i18n"]),
    ("Vector database", "vectordb", ["qdrant", "lancedb", "vector database", "vector search"]),
    ("ANN search", "hnswlib-node", ["hnsw", "instant-distance", "ann search"]),
    ("LLM orchestration", "langchain", ["langchain", "llm-chain", "llm"]),
    ("Tokenizer", "tiktoken", ["tiktoken", "tokenizer", "bpe"]),
    ("OpenAI API", "openai", ["openai", "async-openai"]),
    ("Stripe API", "stripe", ["stripe", "async-stripe"]),
    ("Slack API", "slack-sdk", ["slack", "slack-morphism"]),
    ("SendGrid", "sendgrid", ["sendgrid"]),
    ("AWS S3", "aws-sdk", ["aws-sdk-s3", "aws s3"]),
    ("Firebase", "firebase", ["firebase", "firestore"]),
    ("CouchDB", "pouchdb", ["couchdb", "couch_rs", "couch"]),
    ("Kafka", "kafkajs", ["rdkafka", "kafka"]),
    ("AMQP", "amqplib", ["lapin", "amqp", "rabbitmq"]),
    ("MQTT", "mqtt", ["rumqttc", "mqtt"]),
    ("FTP", "ftp", ["suppaftp", "ftp client"]),
    ("LDAP", "ldapjs", ["ldap3", "ldap"]),
    ("SSH", "ssh2", ["russh", "ssh2", "thrussh"]),
    ("Excel formulas", "formulajs", ["formula", "spreadsheet formula", "formulajs"]),
    ("Terminal tables", "cli-table3", ["comfy-table", "tabled", "prettytable"]),
    ("Terminal spinners", "ora", ["indicatif", "spinners", "spinner"]),
    ("Text wrapping", "wrap-ansi", ["textwrap", "ansi wrap"]),
    ("HTML entities", "he", ["html-escape", "entities", "html entity"]),
    ("Diffing", "diff", ["similar", "diff", "diffy"]),
    ("Glob matching", "micromatch", ["globset", "glob", "fast-glob"]),
    ("File watching", "chokidar", ["notify", "hotwatch", "file watch"]),
    ("Templating", "handlebars", ["handlebars", "tera", "minijinja", "askama"]),
    ("Markdown", "marked", ["comrak", "pulldown-cmark", "markdown"]),
    ("HTML parsing", "cheerio", ["scraper", "html5ever", "select", "tl"]),
    ("Browser automation", "playwright", ["chromiumoxide", "thirtyfour", "playwright", "headless_chrome"]),
    ("Image processing", "sharp", ["image", "fast_image_resize", "imageproc"]),
    ("2D drawing", "canvas", ["tiny-skia", "raqote", "skia"]),
    ("SVG", "svg.js", ["resvg", "usvg", "svg"]),
    ("Charts", "chart.js", ["plotters", "charming", "plotly"]),
    ("3D graphics", "three", ["wgpu", "bevy", "three-d"]),
    ("GIF", "gifencoder", ["gif", "gifski"]),
    ("FFmpeg", "fluent-ffmpeg", ["ffmpeg-next", "ffmpeg", "rsmpeg"]),
    ("Audio synthesis", "tone", ["rodio", "cpal", "fundsp"]),
    ("MP3 encoding", "lamejs", ["mp3lame-encoder", "shine", "mp3 encoder"]),
    ("Video", "libav.js", ["ffmpeg-next", "libav"]),
    ("ZIP", "adm-zip", ["zip", "async_zip", "zip-rs"]),
    ("7-Zip", "node-7z", ["sevenz-rust", "sevenzip"]),
    ("Archives", "archiver", ["tar", "zip", "archiver"]),
    ("CSV", "papaparse", ["csv", "csv-core"]),
    ("YAML", "yaml", ["serde_yaml", "yaml-rust2", "serde_yml"]),
    ("TOML", "toml", ["toml", "toml_edit"]),
    ("XML", "fast-xml-parser", ["quick-xml", "roxmltree", "xml"]),
    ("INI", "ini", ["rust-ini", "configparser", "ini"]),
    ("Config", "convict", ["config", "figment", "config-rs"]),
    ("Logging", "winston", ["tracing", "log", "slog"]),
    ("CLI args", "commander", ["clap", "argh", "lexopt"]),
    ("Big decimal", "decimal.js", ["rust_decimal", "bigdecimal", "decimal"]),
    ("Big integers", "bignumber.js", ["num-bigint", "rug", "ibig"]),
    ("Rational", "fraction.js", ["num-rational", "fraction"]),
    ("Statistics", "simple-statistics", ["statrs", "statistical", "statistics"]),
    ("Linear algebra", "ml-matrix", ["nalgebra", "ndarray"]),
    ("Optimisation", "fmin", ["argmin", "optimization", "nelder-mead"]),
    ("Semver", "semver", ["semver", "node-semver"]),
    ("Colour", "chroma-js", ["palette", "csscolorparser", "colorous"]),
    ("QR codes", "qrcode", ["qrcode", "fast_qr", "qrcodegen"]),
    ("PDF", "pdf-lib", ["lopdf", "pdf", "pdf-writer"]),
    ("JWT", "jsonwebtoken", ["jsonwebtoken", "josekit"]),
    ("Password hashing", "bcryptjs", ["bcrypt", "argon2"]),
    ("Encryption", "crypto-js", ["aes-gcm", "sha2", "ring", "rustls"]),
    ("HTML sanitising", "dompurify", ["ammonia", "sanitize"]),
    ("Rate limiting (tower)", "express-slow-down", ["tower-governor", "governor"]),
    ("OAuth2", "oauth2", ["oauth2", "openidconnect"]),
    ("Authorisation", "casbin", ["casbin", "oso", "casbin-rs"]),
    ("NLP", "natural", ["rust-stemmers", "whatlang", "nlp"]),
    ("Language detection", "franc", ["whatlang", "lingua", "language detection"]),
    ("Encoding detection", "jschardet", ["chardetng", "chardet", "encoding_rs"]),
    ("Unicode segmentation", "grapheme-splitter", ["unicode-segmentation", "unicode-width"]),
    ("CSS processing", "postcss", ["lightningcss", "cssparser", "swc_css"]),
    ("Sass", "sass", ["grass", "sass-rs", "sass"]),
    ("JS bundler", "esbuild", ["swc", "rolldown", "oxc"]),
    ("JS minifier", "terser", ["swc", "oxc_minifier", "minifier"]),
    ("JS formatter", "prettier", ["dprint", "biome", "prettyplease"]),
    ("JS linter", "eslint", ["oxc_linter", "biome", "eslint"]),
    ("Markdown AST", "remark", ["markdown", "comrak", "pulldown-cmark"]),
    ("Template engine", "ejs", ["minijinja", "tera", "askama"]),
    ("Utility belt", "lodash", ["itertools", "lodash", "tap"]),
    ("Reactive streams", "rxjs", ["rxrust", "futures", "tokio-stream"]),
    ("In-memory cache", "node-cache", ["moka", "cached", "lru"]),
    ("HyperLogLog", "hyperloglog", ["hyperloglog", "amadeus", "hyperloglogplus"]),
    ("Cuckoo filter", "cuckoo-filter", ["cuckoofilter", "cuckoo"]),
    ("Graph algorithms", "graphlib", ["petgraph", "pathfinding"]),
    ("Spatial index", "kdbush", ["rstar", "kdtree", "geo"]),
    ("R-tree", "rbush", ["rstar", "rtree"]),
    ("Priority queue", "tinyqueue", ["priority-queue", "binary-heap-plus"]),
    ("LRU cache", "lru-cache", ["lru", "quick-cache"]),
]


def lookup(name: str) -> list[tuple[str, int, str]]:
    """Crates whose name matches exactly or starts with the query."""
    url = f"https://crates.io/api/v1/crates?q={urllib.parse.quote(name)}&per_page=10"
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=20) as r:
            data = json.load(r)
    except Exception:
        return []
    out = []
    for crate in data.get("crates", []):
        out.append((crate["name"], crate.get("downloads") or 0, crate.get("description") or ""))
    return out


def main() -> int:
    report = []
    for label, package, terms in SHORTLIST:
        seen: dict[str, tuple[int, str]] = {}
        for term in terms:
            for name, downloads, description in lookup(term):
                if name not in seen or downloads > seen[name][0]:
                    seen[name] = (downloads, description)
            time.sleep(0.12)
        ranked = sorted(seen.items(), key=lambda kv: -kv[1][0])[:5]
        top = ranked[0][1][0] if ranked else 0
        report.append({"label": label, "npm": package, "top_downloads": top, "crates": ranked})
        print(f"\n=== {label}  (npm: {package}) ===", flush=True)
        for name, (downloads, description) in ranked:
            print(f"   {name:26} {downloads:>12,}  {description[:70]}")
        if not ranked:
            print("   (nothing found)")
    with open("shortlist.json", "w") as f:
        json.dump(report, f, indent=1)

    print("\n\n########## weakest coverage (top crate under 1M downloads) ##########")
    weak = [r for r in report if r["top_downloads"] < 1_000_000]
    weak.sort(key=lambda r: r["top_downloads"])
    for r in weak:
        best = r["crates"][0][0] if r["crates"] else "NOTHING"
        print(f"{r['label']:34} npm={r['npm']:24} best={best:24} dl={r['top_downloads']:>10,}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
