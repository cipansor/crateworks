# iso20022-rs

[![crates.io](https://img.shields.io/crates/v/iso20022-rs.svg)](https://crates.io/crates/iso20022-rs)
[![docs.rs](https://docs.rs/iso20022-rs/badge.svg)](https://docs.rs/iso20022-rs)

ISO 20022 payment messages: `pacs` interbank credit transfers and payment
status reports, `camt` bank statements and `pain` payment initiation.

```rust
use iso20022_rs::camt::BankStatement;

let xml = r#"<Document><BkToCstmrStmt><Stmt>
  <Id>S1</Id>
  <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">100.00</Amt></Bal>
  <Ntry><Amt Ccy="EUR">25.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
</Stmt></BkToCstmrStmt></Document>"#;

let s = BankStatement::parse(xml).unwrap();
assert_eq!(s.id, "S1");
assert_eq!(s.closing_balance(), 125.0);
```

`pacs.008` is the interbank customer credit transfer, the message one bank
sends another to move money for a customer and the ISO 20022 replacement for
the SWIFT MT103. It can be read and generated, and the generated document is
checked against the official schema:

```rust
use iso20022_rs::pacs::CreditTransfer;

let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
  <FIToFICstmrCdtTrf>
    <GrpHdr><MsgId>M1</MsgId><NbOfTxs>1</NbOfTxs>
      <SttlmInf><SttlmMtd>CLRG</SttlmMtd></SttlmInf></GrpHdr>
    <CdtTrfTxInf>
      <PmtId><EndToEndId>E2E-1</EndToEndId></PmtId>
      <IntrBkSttlmAmt Ccy="EUR">100.00</IntrBkSttlmAmt>
      <ChrgBr>SLEV</ChrgBr>
      <Dbtr><Nm>Alice</Nm></Dbtr>
      <DbtrAgt><FinInstnId><BICFI>DEUTDEFFXXX</BICFI></FinInstnId></DbtrAgt>
      <CdtrAgt><FinInstnId><BICFI>COBADEFFXXX</BICFI></FinInstnId></CdtrAgt>
      <Cdtr><Nm>Bob</Nm></Cdtr>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#;

let transfer = CreditTransfer::parse(xml).unwrap();
assert_eq!(transfer.total(), 100.0);
assert!(transfer.control_sum_matches());
```

`pacs.002` is the payment status report, the answer to a `pacs.008`. It says
whether a transfer, or a whole batch, was accepted or rejected, and why. It can
be read and generated:

```rust
use iso20022_rs::pacs::PaymentStatusReport;

let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
  <FIToFIPmtStsRpt>
    <GrpHdr><MsgId>S1</MsgId><CreDtTm>2026-01-01T00:00:00Z</CreDtTm></GrpHdr>
    <TxInfAndSts>
      <OrgnlEndToEndId>E2E-1</OrgnlEndToEndId>
      <TxSts>ACSC</TxSts>
    </TxInfAndSts>
  </FIToFIPmtStsRpt>
</Document>"#;

let report = PaymentStatusReport::parse(xml).unwrap();
assert_eq!(report.statuses[0].transaction_status.as_deref(), Some("ACSC"));
```

Status codes (`TxSts`, `GrpSts`) and reason codes (`Rsn/Cd`) are kept as written
rather than mapped to an enum, because the ISO 20022 external code lists are
large and grow without a schema change; a caller that needs to recognise a code
compares the string.

`pain.001` can be read and generated, with the control sum and transaction count
computed from the transfers so the output is internally consistent. Generating
one requires the debtor bank BIC and a requested execution date, because the
schema marks `DbtrAgt` and `ReqdExctnDt` as required.

ISO 20022 is the XML message standard that is replacing the older SWIFT MT
formats. It is organised into business areas whose message names carry a
three-letter prefix:

* `pain` - payment initiation, sent by a customer to their bank.
* `pacs` - payment clearing and settlement, sent between banks.
* `camt` - cash management, including account statements.

## Scope

Each message is modelled with the fields an application actually needs,
rather than the full generated schema. Unknown elements are ignored, so a
document from a newer schema version still parses. Namespace URIs are
recorded but not resolved.

Generating a `pacs.008` requires a debtor and a creditor agent BIC on every
transaction, because the schema marks `DbtrAgt` and `CdtrAgt` as required; a
message without them would fail validation at the receiving bank.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
