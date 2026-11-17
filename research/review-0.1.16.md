# Review of every crate against its mature comparison package (2026-11-16)

This is the follow-up to `review-0.1.15.md`. That round fixed the holiday
generator and shipped 101 calendars. This round closes the half of an ISO 20022
message pair that was still missing, and adds the gate case that keeps it
correct.

## What this round develops

### `iso20022-rs` 0.2.0 -> 0.3.0, the `pain.002` status report

The crate already read and generated both halves of the interbank pair: a
`pacs.008` credit transfer and the `pacs.002` status report that answers it. It
also read and generated `pain.001`, the payment initiation a customer sends to
its bank. The message a bank sends *back* to that initiation - `pain.002`, the
customer payment status report - was missing, so a caller could send a
`pain.001` and had no way to read the reply. That is the "missing half" pattern
the earlier rounds used to pick a target: the reference defines the message, the
crate models its neighbours, and the gap is one message rather than a domain.

`pain.002.001.10` (`CustomerPaymentStatusReportV10`) is now read and generated.
It carries three levels of outcome, and the model keeps them separate:

* the group header, with the message identifier and the forwarding, debtor and
  creditor agent BICs;
* `OrgnlGrpInfAndSts`, the group-level outcome: which message is being answered,
  its claimed count and control sum, and the `GrpSts` code;
* zero or more `OrgnlPmtInfAndSts`, one per payment instruction in the original,
  each with its own status and reasons and its own `TxInfAndSts` list.

A transaction status keeps the `OrgnlInstrId`, `OrgnlEndToEndId` and
`OrgnlUETR` a reconciliation matches on, the `TxSts` code, the
`StsRsnInf` reasons, and the acceptance time and servicer and clearing
references. As everywhere in this crate, status and reason codes stay `String`s
rather than enums, because the external code lists are large and grow without a
schema change.

**The trap this surfaced, now in `AGENTS.md`.** `iso20022-rs` reads a
`StsRsnInf` list with `find_all`, which walks the whole subtree. That is correct
for a `pacs.002`, where no element nests a `StsRsnInf` of the same type. It is
wrong for a `pain.002`: a payment instruction *contains* its transactions, each
with its own `StsRsnInf`, so a recursive read at the instruction level reports a
transaction's reason as the instruction's. The instruction level now uses a new
`read_status_reasons_direct`, which reads only the direct children; the
transaction level keeps the recursive read, where it is safe. A test pins the
distinction: the fixture gives the instruction `AC04` and a transaction `AC01`,
and the instruction must report only `AC04`.

The shared status, reason and agent helpers moved from `pacs.rs` into
`pub(crate)` free functions so `pain.rs` uses one copy rather than a second. The
refactor removed a duplicated `format_decimal`, `escape` and pair of agent
readers from `pain.rs`; the `OriginalGroupStatus` reader and writer are now
shared the same way.

## The gate that covers it

`research/tools/verify_iso20022.py` builds each message with the crate and
validates it against the official ISO 20022 XSD. `pain002` is the fourth case,
against `pain.002.001.10.xsd`. A message a bank rejects is worse than no message,
so the schema, not a hand-written expectation, is the oracle.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated `pacs.008`, `pain.001`, `pain.002` and `pacs.002` validate |
| `cargo test -p iso20022-rs` | the crate's own suite | 67 unit tests, 4 README tests and 8 doctests pass |

The differential gates of the other crates were not re-run in this round: this
environment carried only `xmlschema`, and the Python and Node oracles those
gates compare against (`holidays`, `python-stdnum`, `pint`, `fuzzywuzzy`,
`hl7apy` and the rest) were not installed. This change touches `iso20022-rs`
alone, and its gate is the XSD one, which ran and passed.

## Documentation

* `crates/iso20022-rs/README.md` gains a `pain.002` section with a runnable
  example, and its opening line now names `pain` status reports as well as
  initiation. `tests/readme.rs` executes the example.
* `PLAN.md`'s `iso20022-rs` section records `pain.002` under "Covers today" and
  extends the list of messages checked against the official schemas.
* `research/tools/README.md` names `pain002` in the gate's message list.
* `AGENTS.md` gains the nested-read trap above.

## Method

This round is a small one by the standard of the last few, and the reason is
deliberate: the target was chosen so the oracle already existed. The ISO 20022
schemas are published, the gate script already downloads and applies them, and
the new message is one more case in a table. The work was therefore less
"measure a domain" and more "finish a pair and prove it validates" - and the
trap the pair exposed is the kind that only shows up when a message nests an
element that also appears at the level above, which is exactly what a status
report does and a status report's neighbour does not.
