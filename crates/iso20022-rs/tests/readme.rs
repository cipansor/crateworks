//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use iso20022_rs::camt::BankStatement;

    let xml = r#"<Document><BkToCstmrStmt><Stmt>
  <Id>S1</Id>
  <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">100.00</Amt></Bal>
  <Ntry><Amt Ccy="EUR">25.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
</Stmt></BkToCstmrStmt></Document>"#;

    let s = BankStatement::parse(xml).unwrap();
    assert_eq!(s.id, "S1");
    assert_eq!(s.closing_balance(), 125.0);
}

#[test]
fn readme_pacs_example() {
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
}

#[test]
fn readme_pacs002_example() {
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
    assert_eq!(
        report.statuses[0].transaction_status.as_deref(),
        Some("ACSC")
    );
}
