#!/usr/bin/env python3
"""Measure npm demand against crates.io supply.

For each candidate, report the npm weekly downloads and the best Rust crate
found for the same job, so the gap between "many people need this" and "Rust
has it" is visible as a number rather than a guess.

The candidate list pairs each npm package with the search terms that would
find a Rust equivalent. A candidate counts as a gap when npm demand is high
and every search term comes back with either nothing or a crate that has
almost no traction.
"""

from __future__ import annotations

import json
import sys
import time
import urllib.parse
import urllib.request

UA = {"User-Agent": "crate-gap-research/1.0 (https://github.com/cipansor)"}

# (npm package, [crates.io search terms], what it does)
CANDIDATES: list[tuple[str, list[str], str]] = [
    # --- parsers and formats ------------------------------------------------
    ("cheerio", ["scraper", "html5ever", "select"], "HTML parsing with a CSS selector API"),
    ("jsdom", ["scraper", "html5ever"], "DOM implementation in memory"),
    ("marked", ["comrak", "pulldown-cmark"], "Markdown to HTML"),
    ("remark", ["comrak", "markdown"], "Markdown AST processing"),
    ("dompurify", ["ammonia", "sanitize"], "HTML sanitisation"),
    ("papaparse", ["csv", "csv-core"], "CSV parsing in the browser"),
    ("xlsx", ["calamine", "umya-spreadsheet"], "Spreadsheet read/write"),
    ("pdf-lib", ["lopdf", "pdf"], "PDF creation and editing"),
    ("pdfjs-dist", ["pdf", "lopdf"], "PDF rendering"),
    ("mammoth", ["docx", "dotext"], "DOCX to HTML"),
    ("handlebars", ["handlebars", "tera", "askama"], "Template engine"),
    ("mustache", ["mustache", "ramhorns"], "Logic-less templates"),
    ("ejs", ["minijinja", "tera"], "Embedded JS templates"),
    ("pug", ["minijinja", "askama"], "Indentation-based templates"),
    ("yaml", ["serde_yaml", "serde_yml", "yaml-rust2"], "YAML parsing"),
    ("ini", ["rust-ini", "configparser"], "INI files"),
    ("toml", ["toml", "toml_edit"], "TOML parsing"),
    ("xml2js", ["quick-xml", "roxmltree"], "XML to object"),
    ("fast-xml-parser", ["quick-xml", "serde-xml-rs"], "XML parsing"),
    ("json5", ["json5", "serde_json"], "JSON5"),
    ("jsonpath-plus", ["jsonpath-rust", "jsonpath"], "JSONPath queries"),
    ("ajv", ["jsonschema", "valico"], "JSON Schema validation"),
    ("protobufjs", ["prost", "protobuf"], "Protocol Buffers"),
    ("msgpack-lite", ["rmp-serde", "rmp"], "MessagePack"),
    ("bson", ["bson"], "BSON"),
    ("ioredis", ["redis"], "Redis client"),
    ("pg", ["tokio-postgres", "sqlx"], "PostgreSQL client"),
    ("mysql2", ["mysql_async", "sqlx"], "MySQL client"),
    ("better-sqlite3", ["rusqlite", "sqlx"], "SQLite"),
    ("mongodb", ["mongodb"], "MongoDB driver"),
    ("kafkajs", ["rdkafka", "kafka"], "Kafka client"),
    ("amqplib", ["lapin", "amiquip"], "AMQP / RabbitMQ"),
    ("mqtt", ["rumqttc", "paho-mqtt"], "MQTT client"),
    ("ws", ["tokio-tungstenite", "tungstenite"], "WebSocket"),
    ("socket.io", ["socketioxide", "rust-socketio"], "Socket.IO"),
    ("nodemailer", ["lettre", "mail-send"], "Email sending"),
    ("imapflow", ["imap", "async-imap"], "IMAP client"),
    ("ldapjs", ["ldap3"], "LDAP client"),
    ("ssh2", ["russh", "ssh2"], "SSH client"),
    ("ftp", ["suppaftp", "ftp"], "FTP client"),
    # --- dates and time -----------------------------------------------------
    ("dayjs", ["chrono", "time", "jiff"], "Date library"),
    ("moment", ["chrono", "jiff"], "Date library"),
    ("date-fns", ["chrono", "time"], "Date utilities"),
    ("luxon", ["chrono-tz", "jiff"], "Time zones"),
    ("cron-parser", ["cron", "croner", "cron_clock"], "Cron expression parsing"),
    ("cronstrue", ["cron-descriptor", "cronstrue"], "Cron to human text"),
    ("rrule", ["rrule", "recurring"], "Recurrence rules"),
    ("humanize-duration", ["humantime", "humanize-duration"], "Human-readable durations"),
    # --- numbers and math ---------------------------------------------------
    ("mathjs", ["nalgebra", "ndarray", "num"], "Math library"),
    ("decimal.js", ["rust_decimal", "bigdecimal"], "Arbitrary-precision decimal"),
    ("big.js", ["rust_decimal", "bigdecimal"], "Decimal arithmetic"),
    ("bignumber.js", ["num-bigint", "rug"], "Big numbers"),
    ("fraction.js", ["num-rational", "fraction"], "Rational numbers"),
    ("currency.js", ["rust_decimal", "money"], "Currency arithmetic"),
    ("dinero.js", ["money", "rusty-money"], "Money and currency"),
    ("accounting", ["rust_decimal"], "Number formatting for money"),
    ("simple-statistics", ["statrs", "statistical"], "Statistics"),
    ("ml-matrix", ["nalgebra", "ndarray-linalg"], "Linear algebra"),
    ("fmin", ["argmin", "optimization"], "Function minimisation"),
    ("random", ["rand", "fastrand"], "Random numbers"),
    ("uuid", ["uuid"], "UUIDs"),
    ("nanoid", ["nanoid", "uuid"], "Short ids"),
    ("cuid2", ["cuid2", "cuid"], "Collision-resistant ids"),
    ("ulid", ["ulid"], "ULIDs"),
    ("hashids", ["hashids", "harsh"], "Obfuscated ids"),
    ("semver", ["semver", "node-semver"], "Semantic versions"),
    ("compare-versions", ["semver"], "Version comparison"),
    # --- text and language --------------------------------------------------
    ("slugify", ["slug", "slugify"], "URL slugs"),
    ("he", ["html-escape", "he"], "HTML entity encode/decode"),
    ("entities", ["html-escape", "entities"], "Entity tables"),
    ("striptags", ["scraper", "html2text"], "Strip HTML tags"),
    ("turndown", ["htmd", "html2md"], "HTML to Markdown"),
    ("html-to-text", ["html2text"], "HTML to plain text"),
    ("diff", ["similar", "diff"], "Text diffing"),
    ("diff-match-patch", ["diffy", "similar"], "Patch and diff"),
    ("fast-levenshtein", ["strsim", "levenshtein"], "Edit distance"),
    ("js-levenshtein", ["strsim"], "Edit distance"),
    ("string-similarity", ["strsim"], "String similarity"),
    ("fuse.js", ["fuzzy-matcher", "fuzzy-search"], "Fuzzy search"),
    ("natural", ["rust-stemmers", "whatlang"], "NLP toolkit"),
    ("compromise", ["nlprule", "rust-bert"], "English NLP"),
    ("franc", ["whatlang", "lingua"], "Language detection"),
    ("nodehun", ["hunspell", "spellbook"], "Spell checking"),
    ("bad-words", ["censor", "profanity"], "Profanity filter"),
    ("pluralize", ["inflector", "pluralize"], "Pluralisation"),
    ("change-case", ["convert_case", "heck"], "Case conversion"),
    ("title-case", ["convert_case", "titlecase"], "Title casing"),
    ("transliteration", ["transliterate", "deunicode"], "Transliteration"),
    ("iconv-lite", ["encoding_rs", "encoding"], "Character encodings"),
    ("jschardet", ["chardetng", "chardet"], "Encoding detection"),
    ("node-emoji", ["emojis", "emoji"], "Emoji lookup"),
    ("grapheme-splitter", ["unicode-segmentation"], "Grapheme clusters"),
    ("wrap-ansi", ["textwrap", "console"], "ANSI-aware wrapping"),
    ("strip-ansi", ["strip-ansi-escapes", "console"], "Strip ANSI codes"),
    ("cli-table3", ["comfy-table", "tabled"], "Terminal tables"),
    ("boxen", ["comfy-table", "tabled"], "Terminal boxes"),
    ("chalk", ["owo-colors", "colored", "nu-ansi-term"], "Terminal colours"),
    ("ora", ["indicatif", "spinners"], "Terminal spinners"),
    ("cli-progress", ["indicatif"], "Progress bars"),
    ("figlet", ["figlet-rs", "figlet"], "ASCII art text"),
    ("qrcode", ["qrcode", "fast_qr"], "QR codes"),
    ("jsbarcode", ["barcoders", "barcode"], "Barcodes"),
    ("bwip-js", ["barcoders"], "Barcode generation"),
    # --- crypto and security ------------------------------------------------
    ("jsonwebtoken", ["jsonwebtoken", "josekit"], "JWT"),
    ("jose", ["josekit", "jsonwebtoken"], "JOSE / JWE"),
    ("bcryptjs", ["bcrypt", "argon2"], "Password hashing"),
    ("argon2", ["argon2"], "Argon2 hashing"),
    ("crypto-js", ["aes-gcm", "sha2", "digest"], "Crypto primitives"),
    ("node-forge", ["rsa", "x509-parser"], "PKI in pure code"),
    ("otplib", ["totp-rs", "totp"], "TOTP"),
    ("speakeasy", ["totp-rs"], "TOTP"),
    ("zxcvbn", ["zxcvbn", "passwords"], "Password strength"),
    ("sanitize-html", ["ammonia"], "HTML sanitiser"),
    ("helmet", ["tower-http", "axum"], "HTTP security headers"),
    ("csurf", ["axum-csrf", "csrf"], "CSRF tokens"),
    ("express-rate-limit", ["tower-governor", "governor"], "Rate limiting"),
    ("express-slow-down", ["tower-governor"], "Request throttling"),
    ("passport", ["axum-login", "oauth2"], "Authentication"),
    ("oauth2", ["oauth2", "openidconnect"], "OAuth2"),
    ("casbin", ["casbin", "oso"], "Authorisation"),
    ("node-rsa", ["rsa", "ring"], "RSA"),
    # --- web and http -------------------------------------------------------
    ("axios", ["reqwest", "ureq"], "HTTP client"),
    ("node-fetch", ["reqwest", "ureq"], "Fetch API"),
    ("undici", ["reqwest", "hyper"], "HTTP/1.1 client"),
    ("got", ["reqwest"], "HTTP client"),
    ("http-proxy", ["hyper-proxy", "warp"], "HTTP proxy"),
    ("http-proxy-middleware", ["tower-http", "axum"], "Proxy middleware"),
    ("cheerio-select", ["scraper", "select"], "CSS selector engine"),
    ("playwright", ["chromiumoxide", "thirtyfour"], "Browser automation"),
    ("puppeteer", ["chromiumoxide", "headless_chrome"], "Headless Chrome"),
    ("robots-parser", ["robotstxt", "texting_robots"], "robots.txt"),
    ("sitemap", ["sitemap", "sitemap-rs"], "Sitemap generation"),
    ("rss-parser", ["feed-rs", "rss"], "RSS and Atom"),
    ("feed", ["feed-rs", "rss"], "Feed generation"),
    ("metascraper", ["scraper", "readability"], "Page metadata"),
    ("readability", ["readability", "dom_smoothie"], "Article extraction"),
    ("user-agents", ["woothee", "uaparser"], "User-agent parsing"),
    ("ua-parser-js", ["woothee", "uaparser"], "User-agent parsing"),
    ("ipaddr.js", ["ipnet", "ipnetwork"], "IP address parsing"),
    ("geoip-lite", ["maxminddb", "geoip2"], "GeoIP lookup"),
    ("libphonenumber-js", ["phonenumber", "rust-phone"], "Phone number parsing"),
    ("validator", ["validator", "garde"], "Input validation"),
    ("email-validator", ["email_address", "validator"], "Email validation"),
    ("isemail", ["email_address"], "Email validation"),
    ("dom-parser", ["scraper", "tl"], "HTML parsing"),
    ("cookie", ["cookie", "tower-cookies"], "Cookie parsing"),
    ("set-cookie-parser", ["cookie", "cookie_store"], "Cookie parsing"),
    ("tough-cookie", ["cookie_store"], "Cookie jar"),
    ("qs", ["serde_qs", "serde_urlencoded"], "Query string parsing"),
    ("query-string", ["serde_qs", "serde_urlencoded"], "Query strings"),
    ("url-parse", ["url", "urlencoding"], "URL parsing"),
    ("mime", ["mime", "mime_guess"], "MIME types"),
    ("content-type", ["mime", "mime_guess"], "Content-Type parsing"),
    ("busboy", ["multer", "multipart"], "Multipart parsing"),
    ("formidable", ["multer", "multer-rs"], "File uploads"),
    ("archiver", ["zip", "async_zip"], "Archive creation"),
    ("adm-zip", ["zip", "zip-rs"], "ZIP"),
    ("tar", ["tar"], "TAR"),
    ("unzipper", ["zip"], "Unzip"),
    ("node-7z", ["sevenz-rust"], "7-Zip"),
    ("yauzl", ["zip"], "Streaming unzip"),
    # --- data structures and algorithms -------------------------------------
    ("lodash", ["itertools", "lodash-rs"], "Utility belt"),
    ("underscore", ["itertools"], "Utility belt"),
    ("ramda", ["itertools", "tap"], "Functional utilities"),
    ("immutable", ["im", "rpds"], "Persistent collections"),
    ("mobx", ["salsa", "leptos"], "Reactive state"),
    ("rxjs", ["rxrust", "futures"], "Reactive streams"),
    ("async", ["futures", "tokio"], "Async utilities"),
    ("bluebird", ["futures", "tokio"], "Promises"),
    ("p-limit", ["futures", "tokio"], "Concurrency limiting"),
    ("p-queue", ["futures", "tokio"], "Task queue"),
    ("bottleneck", ["governor", "leaky-bucket"], "Rate limiter"),
    ("lru-cache", ["lru", "quick-cache"], "LRU cache"),
    ("node-cache", ["moka", "cached"], "In-memory cache"),
    ("bloom-filters", ["bloomfilter", "bloom"], "Bloom filters"),
    ("hyperloglog", ["hyperloglog", "amadeus-streaming"], "HyperLogLog"),
    ("cuckoo-filter", ["cuckoofilter"], "Cuckoo filters"),
    ("mnemonist", ["petgraph", "indexmap"], "Data structures"),
    ("graphlib", ["petgraph"], "Graph algorithms"),
    ("js-graph-algorithms", ["petgraph", "pathfinding"], "Graph algorithms"),
    ("dijkstrajs", ["pathfinding", "petgraph"], "Dijkstra"),
    ("toposort", ["petgraph", "topological-sort"], "Topological sort"),
    ("binary-search", ["rust-fuzzy-search", "binary-search"], "Binary search"),
    ("heap-js", ["binary-heap-plus", "priority-queue"], "Heaps"),
    ("tinyqueue", ["priority-queue", "binary-heap-plus"], "Priority queues"),
    ("union-find", ["petgraph", "union-find"], "Union-find"),
    ("interval-tree", ["iset", "rangemap"], "Interval trees"),
    ("kdbush", ["rstar", "kdtree"], "Spatial index"),
    ("geokdbush", ["rstar", "geo"], "Geospatial index"),
    ("rbush", ["rstar", "rtree"], "R-tree"),
    ("quadtree", ["rstar"], "Quadtree"),
    # --- graphics and media -------------------------------------------------
    ("sharp", ["image", "fast_image_resize"], "Image resizing"),
    ("jimp", ["image", "imageproc"], "Image manipulation"),
    ("gm", ["image", "magick-rust"], "ImageMagick bindings"),
    ("canvas", ["tiny-skia", "raqote"], "2D drawing"),
    ("svg.js", ["usvg", "resvg", "svg"], "SVG manipulation"),
    ("d3", ["plotters", "charming"], "Data visualisation"),
    ("chart.js", ["plotters", "charming"], "Charts"),
    ("echarts", ["plotters", "charming"], "Charts"),
    ("three", ["wgpu", "bevy"], "3D graphics"),
    ("babylonjs", ["bevy", "wgpu"], "3D engine"),
    ("pixi.js", ["wgpu", "bevy"], "2D WebGL"),
    ("konva", ["tiny-skia", "raqote"], "2D canvas"),
    ("fabric", ["tiny-skia", "raqote"], "Canvas objects"),
    ("gifencoder", ["gif", "image"], "GIF encoding"),
    ("gifuct-js", ["gif"], "GIF decoding"),
    ("apng-js", ["apng", "image"], "APNG"),
    ("libav.js", ["ffmpeg-next", "ffmpeg"], "FFmpeg bindings"),
    ("fluent-ffmpeg", ["ffmpeg-next", "ffmpeg"], "FFmpeg wrapper"),
    ("music-metadata", ["lofty", "id3"], "Audio metadata"),
    ("wavefile", ["hound", "symphonia"], "WAV"),
    ("tone", ["rodio", "cpal"], "Audio synthesis"),
    ("howler", ["rodio", "cpal"], "Audio playback"),
    ("lamejs", ["mp3lame-encoder", "shine"], "MP3 encoding"),
    # --- documents ----------------------------------------------------------
    ("docx", ["docx-rs", "dotext"], "DOCX generation"),
    ("exceljs", ["umya-spreadsheet", "rust_xlsxwriter"], "XLSX read/write"),
    ("xlsx-populate", ["umya-spreadsheet"], "XLSX templating"),
    ("pptxgenjs", ["pptx-rs", "powerpoint"], "PPTX generation"),
    ("pdfkit", ["printpdf", "genpdf"], "PDF generation"),
    ("html-pdf", ["printpdf", "headless_chrome"], "HTML to PDF"),
    ("jspdf", ["printpdf", "genpdf"], "PDF in the browser"),
    ("epub-gen", ["epub-builder", "epub"], "EPUB generation"),
    ("ical-generator", ["ical", "rrule"], "iCalendar"),
    ("ical.js", ["ical", "rrule"], "iCalendar parsing"),
    ("vcard4", ["vcard", "vcard-rs"], "vCard"),
    ("node-xlsx", ["calamine", "umya-spreadsheet"], "XLSX"),
    # --- config and infrastructure ------------------------------------------
    ("dotenv", ["dotenvy", "dotenv"], "Environment files"),
    ("convict", ["config", "figment"], "Configuration"),
    ("rc", ["config", "figment"], "Configuration"),
    ("nconf", ["config", "figment"], "Configuration"),
    ("node-config", ["config", "figment"], "Configuration"),
    ("winston", ["tracing", "log"], "Logging"),
    ("pino", ["tracing", "log"], "Structured logging"),
    ("bunyan", ["tracing", "slog"], "JSON logging"),
    ("log4js", ["log4rs", "tracing"], "Logging"),
    ("morgan", ["tracing", "axum"], "HTTP logging"),
    ("debug", ["log", "tracing"], "Debug logging"),
    ("commander", ["clap", "argh"], "CLI argument parsing"),
    ("yargs", ["clap", "lexopt"], "CLI argument parsing"),
    ("minimist", ["lexopt", "clap"], "Argument parsing"),
    ("inquirer", ["dialoguer", "inquire"], "Interactive prompts"),
    ("prompts", ["dialoguer", "inquire"], "Interactive prompts"),
    ("blessed", ["ratatui", "crossterm"], "TUI"),
    ("ink", ["ratatui", "crossterm"], "TUI"),
    ("chokidar", ["notify", "hotwatch"], "File watching"),
    ("fs-extra", ["fs_extra", "fs-err"], "Filesystem helpers"),
    ("glob", ["glob", "globset"], "Glob matching"),
    ("minimatch", ["globset", "glob"], "Glob matching"),
    ("micromatch", ["globset", "glob"], "Glob matching"),
    ("fast-glob", ["glob", "walkdir"], "Fast globbing"),
    ("rimraf", ["fs_extra", "remove_dir_all"], "Recursive delete"),
    ("mkdirp", ["fs_extra"], "Recursive mkdir"),
    ("tmp", ["tempfile"], "Temporary files"),
    ("proper-lockfile", ["fd-lock", "file-lock"], "File locking"),
    ("node-cron", ["tokio-cron-scheduler", "cron"], "Scheduling"),
    ("bull", ["apalis", "celery-rs"], "Job queues"),
    ("agenda", ["apalis", "tokio-cron-scheduler"], "Job scheduling"),
    ("pm2", ["tokio", "systemd"], "Process manager"),
    ("i18next", ["fluent", "rust-i18n"], "Internationalisation"),
    ("intl-messageformat", ["fluent", "icu"], "Message formatting"),
    ("numeral", ["num-format", "humanize"], "Number formatting"),
    ("humanize", ["humansize", "humantime"], "Human-readable sizes"),
    ("pretty-bytes", ["humansize", "bytesize"], "Byte sizes"),
    ("pretty-ms", ["humantime", "humantime-serde"], "Milliseconds to text"),
    ("filesize", ["humansize", "bytesize"], "File sizes"),
    ("bytes", ["bytesize", "bytes"], "Byte sizes"),
    ("pluralize-id", ["inflector"], "Pluralisation"),
    ("randomcolor", ["random_color", "palette"], "Random colours"),
    ("color", ["palette", "colorsys"], "Colour conversion"),
    ("chroma-js", ["palette", "colorous"], "Colour manipulation"),
    ("tinycolor2", ["palette", "csscolorparser"], "Colour parsing"),
    ("culori", ["palette", "csscolorparser"], "Colour science"),
    ("colorbrewer", ["colorous", "palette"], "Colour maps"),
    ("d3-scale-chromatic", ["colorous", "palette"], "Colour maps"),
    ("postcss", ["lightningcss", "swc_css_parser"], "CSS processing"),
    ("sass", ["grass", "sass-rs"], "Sass"),
    ("less", ["less-rs", "lightningcss"], "Less"),
    ("autoprefixer", ["lightningcss"], "Vendor prefixes"),
    ("cssnano", ["lightningcss"], "CSS minification"),
    ("terser", ["swc", "oxc_minifier"], "JS minification"),
    ("esbuild", ["swc", "rolldown"], "Bundler"),
    ("rollup", ["rolldown", "swc"], "Bundler"),
    ("webpack", ["rspack", "swc"], "Bundler"),
    ("vite", ["rolldown", "trunk"], "Dev server and bundler"),
    ("babel", ["swc", "oxc"], "JS transpiler"),
    ("typescript", ["oxc", "swc"], "TypeScript compiler"),
    ("prettier", ["dprint", "biome"], "Code formatter"),
    ("eslint", ["oxc_linter", "biome"], "JS linter"),
    ("stylelint", ["lightningcss"], "CSS linter"),
    ("postcss-preset-env", ["lightningcss"], "Modern CSS"),
    ("workbox", ["service-worker", "yew"], "Service workers"),
    ("idb", ["indexed_db_futures", "rexie"], "IndexedDB"),
    ("localforage", ["web-sys", "gloo-storage"], "Browser storage"),
    ("dexie", ["rexie", "indexed_db_futures"], "IndexedDB wrapper"),
    ("sql.js", ["rusqlite", "sqlite-wasm-rs"], "SQLite in the browser"),
    ("pouchdb", ["couch_rs", "couchdb"], "CouchDB client"),
    ("firebase", ["firebase-rs", "firestore"], "Firebase"),
    ("aws-sdk", ["aws-sdk-s3", "aws-sdk-rust"], "AWS SDK"),
    ("@google-cloud/storage", ["google-cloud-storage"], "GCS client"),
    ("stripe", ["async-stripe", "stripe-rust"], "Stripe API"),
    ("twilio", ["twilio", "twilio-rust"], "Twilio API"),
    ("sendgrid", ["sendgrid", "sendgrid-rs"], "SendGrid API"),
    ("slack-sdk", ["slack-morphism", "slack"], "Slack API"),
    ("openai", ["async-openai", "openai-api-rs"], "OpenAI API"),
    ("@anthropic-ai/sdk", ["anthropic", "anthropic-sdk"], "Anthropic API"),
    ("langchain", ["langchain-rust", "llm-chain"], "LLM orchestration"),
    ("tiktoken", ["tiktoken-rs", "tiktoken"], "Tokenizer"),
    ("vectordb", ["qdrant-client", "lancedb"], "Vector database"),
    ("hnswlib-node", ["hnsw_rs", "instant-distance"], "ANN search"),
    ("faiss-node", ["faiss", "hnsw_rs"], "Similarity search"),
]


def npm_weekly(package: str) -> int | None:
    """The npm weekly download count, or None when the package is unknown."""
    quoted = urllib.parse.quote(package, safe="@/")
    url = f"https://api.npmjs.org/downloads/point/last-week/{quoted}"
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=20) as r:
            return json.load(r).get("downloads")
    except Exception:
        return None


def crates_for(term: str, limit: int = 4) -> list[tuple[str, int]]:
    """The crates matching a search term, most-downloaded first."""
    url = f"https://crates.io/api/v1/crates?q={urllib.parse.quote(term)}&per_page={limit}"
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=20) as r:
            data = json.load(r)
    except Exception:
        return []
    found = []
    for crate in data.get("crates", []):
        found.append((crate["name"], crate.get("downloads") or 0))
    found.sort(key=lambda c: -c[1])
    return found


def main() -> int:
    # A crate needs some traction before it counts as covering a package.
    ADOPTED = 200_000
    rows = []
    for package, terms, purpose in CANDIDATES:
        weekly = npm_weekly(package)
        best: tuple[str, int] | None = None
        for term in terms:
            for name, downloads in crates_for(term):
                if best is None or downloads > best[1]:
                    best = (name, downloads)
            time.sleep(0.15)
        rows.append((package, weekly, best, purpose, terms))
        flag = ""
        if weekly and weekly > 100_000 and (best is None or best[1] < ADOPTED):
            flag = "  <== GAP"
        print(
            f"{package:32} npm/wk={weekly if weekly is not None else '?':>12} "
            f"rust={best[0] if best else '-':24} crates/dl={best[1] if best else 0:>12}{flag}",
            flush=True,
        )
        time.sleep(0.15)

    gaps = [r for r in rows if r[1] and r[1] > 100_000 and (r[2] is None or r[2][1] < ADOPTED)]
    print(f"\n{len(gaps)} gaps out of {len(rows)} candidates")
    with open("gap_rows.json", "w") as f:
        json.dump(rows, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
