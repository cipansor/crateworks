//! `pain` payment initiation messages.
//!
//! A `pain.001` message is a customer instructing their bank to make one or
//! more payments. This module can both read such a message and build one, which
//! is the direction most applications need: they collect payment details from a
//! user and have to hand the bank a file in the format it expects.
//!
//! [`CustomerPaymentStatusReport`] is `pain.002`, the bank's answer: whether
//! the instruction, or a payment inside it, was accepted, rejected or is still
//! pending, and why. A `pain.001` is not known to have been accepted until its
//! `pain.002` comes back, so reading one is the other half of sending a
//! `pain.001`.

use std::fmt::Write as _;

use crate::amount::Amount;
use crate::camt::CreditDebit;
use crate::pacs::{
    OriginalGroupStatus, StatusReason, agent_bic, escape, format_decimal, optional_decimal,
    read_original_group, read_status_reasons, read_status_reasons_direct, write_agent,
    write_original_group_tagged, write_status_reasons,
};
use crate::{Error, Result, find, find_all, optional_text, required_text};
use xmlkit::Element;

/// One payment instruction inside a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransfer {
    /// Instruction identifier, unique within the message.
    pub instruction_id: String,
    /// Amount to transfer.
    pub amount: Amount,
    /// Name of the person or company receiving the money.
    pub creditor_name: String,
    /// Creditor IBAN.
    pub creditor_iban: String,
    /// Optional BIC of the creditor's bank.
    pub creditor_bic: Option<String>,
    /// Remittance information shown to the creditor.
    pub remittance: Option<String>,
}

/// A `pain.001` customer credit transfer initiation message.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransferInitiation {
    /// Message identifier, assigned by the sender.
    pub message_id: String,
    /// Name of the account being debited.
    pub debtor_name: String,
    /// IBAN of the account being debited.
    pub debtor_iban: String,
    /// BIC of the debtor's bank. The schema requires `DbtrAgt`, so a message
    /// without one cannot be written.
    pub debtor_bic: Option<String>,
    /// Requested execution date, an ISO 8601 date such as `2026-10-21`. The
    /// schema requires `ReqdExctnDt`, so a message without one cannot be
    /// written.
    pub requested_execution_date: Option<String>,
    /// The payment instructions.
    pub transfers: Vec<CreditTransfer>,
}

/// The outcome of one payment inside a `pain.001` instruction, an ISO 20022
/// `PaymentTransaction105`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionStatus {
    /// Identifier for this status, if the reporter assigned one.
    pub status_id: Option<String>,
    /// The instruction identifier being reported on.
    pub original_instruction_id: Option<String>,
    /// The end-to-end identifier being reported on, the field a reconciliation
    /// matches on.
    pub original_end_to_end_id: Option<String>,
    /// The UETR being reported on, if present.
    pub original_uetr: Option<String>,
    /// The transaction status code, such as `ACSC` or `RJCT`.
    pub transaction_status: Option<String>,
    /// The reasons given for the status.
    pub status_reasons: Vec<StatusReason>,
    /// When the status was accepted, if stated.
    pub acceptance_datetime: Option<String>,
    /// The account servicer's own reference, if present.
    pub account_servicer_reference: Option<String>,
    /// The clearing system reference, if present.
    pub clearing_system_reference: Option<String>,
}

/// The outcome of one `pain.001` payment instruction, an ISO 20022
/// `OriginalPaymentInstruction32`.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalPaymentInstruction {
    /// Identifier of the instruction being reported on, the `PmtInfId`.
    pub original_payment_instruction_id: String,
    /// The transaction count it claimed, as written.
    pub original_number_of_transactions: Option<String>,
    /// The control sum it claimed, if stated.
    pub original_control_sum: Option<f64>,
    /// The instruction status code, such as `ACCP` or `RJCT`.
    pub payment_information_status: Option<String>,
    /// The reasons given for the instruction status.
    pub status_reasons: Vec<StatusReason>,
    /// The per-transaction outcomes, in document order.
    pub transaction_statuses: Vec<TransactionStatus>,
}

/// A `pain.002` customer payment status report.
///
/// This is the message a bank sends to say what happened to a `pain.001`: the
/// group-level outcome, the outcome of each payment instruction in it, and the
/// outcome of each payment inside those instructions. A payment a `pain.001`
/// initiated is not known to have been accepted until its `pain.002` comes
/// back, so reading one is the other half of sending one.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomerPaymentStatusReport {
    /// Message identifier, assigned by the sending bank.
    pub message_id: String,
    /// Creation date and time, kept as written.
    pub creation_datetime: Option<String>,
    /// Forwarding bank BIC, if present.
    pub forwarding_agent_bic: Option<String>,
    /// Debtor bank BIC, if present.
    pub debtor_agent_bic: Option<String>,
    /// Creditor bank BIC, if present.
    pub creditor_agent_bic: Option<String>,
    /// The group-level outcome, the message the report answers.
    pub original_group: OriginalGroupStatus,
    /// The per-instruction outcomes, in document order.
    pub original_payment_instructions: Vec<OriginalPaymentInstruction>,
}

impl CreditTransferInitiation {
    /// Reads a `pain.001` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`], [`Error::MissingElement`] or
    /// [`Error::InvalidValue`] as appropriate.
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let group = find(&body, "GrpHdr").ok_or(Error::MissingElement("GrpHdr"))?;
        let message_id = required_text(group, "MsgId")?;
        let debtor_name = required_text(&body, "Dbtr").or_else(|_| {
            find(&body, "Dbtr")
                .map(Element::deep_text)
                .ok_or(Error::MissingElement("Dbtr"))
        })?;
        let debtor_iban = find(&body, "DbtrAcct")
            .and_then(|a| optional_text(a, "IBAN"))
            .ok_or(Error::MissingElement("DbtrAcct/IBAN"))?;
        let debtor_bic = find(&body, "DbtrAgt").and_then(agent_bic);
        let requested_execution_date = find(&body, "ReqdExctnDt")
            .and_then(|d| optional_text(d, "Dt").or_else(|| optional_text(d, "DtTm")));

        let mut elements = Vec::new();
        find_all(&body, "CdtTrfTxInf", &mut elements);
        let mut transfers = Vec::with_capacity(elements.len());
        for tx in elements {
            transfers.push(Self::read_transfer(tx)?);
        }
        Ok(Self {
            message_id,
            debtor_name,
            debtor_iban,
            debtor_bic,
            requested_execution_date,
            transfers,
        })
    }

    fn read_transfer(tx: &Element) -> Result<CreditTransfer> {
        let instruction_id = find(tx, "PmtId")
            .and_then(|p| optional_text(p, "InstrId").or_else(|| optional_text(p, "EndToEndId")))
            .unwrap_or_else(|| "NOTPROVIDED".to_string());
        let amt_el = find(tx, "InstdAmt")
            .or_else(|| find(tx, "Amt"))
            .ok_or(Error::MissingElement("InstdAmt"))?;
        let amount = Amount::from_element(amt_el)?;
        let creditor_name = find(tx, "Cdtr")
            .and_then(|c| optional_text(c, "Nm"))
            .ok_or(Error::MissingElement("Cdtr/Nm"))?;
        let creditor_iban = find(tx, "CdtrAcct")
            .and_then(|a| optional_text(a, "IBAN"))
            .ok_or(Error::MissingElement("CdtrAcct/IBAN"))?;
        let creditor_bic = find(tx, "CdtrAgt").and_then(agent_bic);
        let remittance = find(tx, "RmtInf").and_then(|r| optional_text(r, "Ustrd"));
        Ok(CreditTransfer {
            instruction_id,
            amount,
            creditor_name,
            creditor_iban,
            creditor_bic,
            remittance,
        })
    }

    /// The sum of all transfers, in the currency of the first transfer.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.transfers.iter().map(|t| t.amount.amount).sum()
    }

    /// The number of transfers, which the control sum must agree with.
    #[must_use]
    pub fn number_of_transfers(&self) -> usize {
        self.transfers.len()
    }

    /// Writes the message as a `pain.001.001.09` document.
    ///
    /// The control sum and the transaction count are computed from the
    /// transfers rather than taken from the caller, so the output is internally
    /// consistent by construction.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when there are no transfers, because a
    /// bank rejects an empty batch, or when the debtor bank BIC or the requested
    /// execution date is absent. The schema marks `DbtrAgt` and `ReqdExctnDt` as
    /// required, so a message without them would fail validation.
    pub fn to_xml(&self) -> Result<String> {
        if self.transfers.is_empty() {
            return Err(Error::MissingElement("CdtTrfTxInf"));
        }
        let Some(debtor_bic) = &self.debtor_bic else {
            return Err(Error::MissingElement("DbtrAgt"));
        };
        let Some(execution_date) = &self.requested_execution_date else {
            return Err(Error::MissingElement("ReqdExctnDt"));
        };
        let now = "2026-01-01T00:00:00Z";
        let mut out = String::with_capacity(1024 + self.transfers.len() * 512);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pain.001.001.09\">\n  <CstmrCdtTrfInitn>\n",
        );
        out.push_str("    <GrpHdr>\n");
        let _ = writeln!(out, "      <MsgId>{}</MsgId>", escape(&self.message_id));
        let _ = writeln!(out, "      <CreDtTm>{now}</CreDtTm>");
        let _ = writeln!(out, "      <NbOfTxs>{}</NbOfTxs>", self.transfers.len());
        let _ = writeln!(
            out,
            "      <CtrlSum>{}</CtrlSum>",
            format_decimal(self.total())
        );
        out.push_str("      <InitgPty>\n");
        let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&self.debtor_name));
        out.push_str("      </InitgPty>\n    </GrpHdr>\n");

        out.push_str("    <PmtInf>\n");
        let _ = writeln!(
            out,
            "      <PmtInfId>{}-PMT</PmtInfId>",
            escape(&self.message_id)
        );
        out.push_str("      <PmtMtd>TRF</PmtMtd>\n");
        let _ = writeln!(out, "      <NbOfTxs>{}</NbOfTxs>", self.transfers.len());
        let _ = writeln!(
            out,
            "      <CtrlSum>{}</CtrlSum>",
            format_decimal(self.total())
        );
        let _ = writeln!(
            out,
            "      <ReqdExctnDt>\n        <Dt>{}</Dt>\n      </ReqdExctnDt>",
            escape(execution_date)
        );
        out.push_str("      <Dbtr>\n");
        let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&self.debtor_name));
        out.push_str("      </Dbtr>\n");
        out.push_str("      <DbtrAcct>\n        <Id>\n");
        let _ = writeln!(out, "          <IBAN>{}</IBAN>", escape(&self.debtor_iban));
        out.push_str("        </Id>\n      </DbtrAcct>\n");
        out.push_str("      <DbtrAgt>\n        <FinInstnId>\n");
        let _ = writeln!(out, "          <BICFI>{}</BICFI>", escape(debtor_bic));
        out.push_str("        </FinInstnId>\n      </DbtrAgt>\n");

        for (i, t) in self.transfers.iter().enumerate() {
            out.push_str("      <CdtTrfTxInf>\n");
            out.push_str("        <PmtId>\n");
            let _ = writeln!(
                out,
                "          <InstrId>{}</InstrId>",
                escape(&t.instruction_id)
            );
            let _ = writeln!(
                out,
                "          <EndToEndId>{}</EndToEndId>",
                escape(&t.instruction_id)
            );
            out.push_str("        </PmtId>\n");
            let _ = writeln!(
                out,
                "        <Amt><InstdAmt Ccy=\"{}\">{}</InstdAmt></Amt>",
                escape(t.amount.currency.as_str()),
                format_decimal(t.amount.amount)
            );
            if let Some(bic) = &t.creditor_bic {
                out.push_str("        <CdtrAgt>\n          <FinInstnId>\n");
                let _ = writeln!(out, "            <BICFI>{}</BICFI>", escape(bic));
                out.push_str("          </FinInstnId>\n        </CdtrAgt>\n");
            }
            out.push_str("        <Cdtr>\n");
            let _ = writeln!(out, "          <Nm>{}</Nm>", escape(&t.creditor_name));
            out.push_str("        </Cdtr>\n");
            out.push_str("        <CdtrAcct>\n          <Id>\n");
            let _ = writeln!(out, "            <IBAN>{}</IBAN>", escape(&t.creditor_iban));
            out.push_str("          </Id>\n        </CdtrAcct>\n");
            if let Some(remit) = &t.remittance {
                out.push_str("        <RmtInf>\n");
                let _ = writeln!(out, "          <Ustrd>{}</Ustrd>", escape(remit));
                out.push_str("        </RmtInf>\n");
            }
            let _ = writeln!(out, "      </CdtTrfTxInf>");
            let _ = i;
        }
        out.push_str("    </PmtInf>\n  </CstmrCdtTrfInitn>\n</Document>\n");
        Ok(out)
    }
}

impl CustomerPaymentStatusReport {
    /// Reads a `pain.002` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input, [`Error::MissingElement`]
    /// when `GrpHdr` or the group-level `OrgnlGrpInfAndSts` is absent, or
    /// [`Error::InvalidValue`] for a malformed control sum.
    ///
    /// # Example
    ///
    /// ```
    /// use iso20022_rs::pain::CustomerPaymentStatusReport;
    ///
    /// let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.002.001.10">
    ///   <CstmrPmtStsRpt>
    ///     <GrpHdr><MsgId>S1</MsgId><CreDtTm>2026-10-21T09:35:00Z</CreDtTm></GrpHdr>
    ///     <OrgnlGrpInfAndSts>
    ///       <OrgnlMsgId>MSG-1</OrgnlMsgId>
    ///       <OrgnlMsgNmId>pain.001.001.09</OrgnlMsgNmId>
    ///       <GrpSts>ACCP</GrpSts>
    ///     </OrgnlGrpInfAndSts>
    ///   </CstmrPmtStsRpt>
    /// </Document>"#;
    ///
    /// let report = CustomerPaymentStatusReport::parse(xml).unwrap();
    /// assert_eq!(report.message_id, "S1");
    /// assert_eq!(report.original_group.original_message_name_id, "pain.001.001.09");
    /// assert_eq!(report.original_group.group_status.as_deref(), Some("ACCP"));
    /// ```
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let group = find(&body, "GrpHdr").ok_or(Error::MissingElement("GrpHdr"))?;
        let message_id = required_text(group, "MsgId")?;
        let creation_datetime = optional_text(group, "CreDtTm");
        let forwarding_agent_bic = find(group, "FwdgAgt").and_then(agent_bic);
        let debtor_agent_bic = find(group, "DbtrAgt").and_then(agent_bic);
        let creditor_agent_bic = find(group, "CdtrAgt").and_then(agent_bic);

        let group_element =
            find(&body, "OrgnlGrpInfAndSts").ok_or(Error::MissingElement("OrgnlGrpInfAndSts"))?;
        let original_group = read_original_group(group_element)?;

        let mut instruction_elements = Vec::new();
        find_all(&body, "OrgnlPmtInfAndSts", &mut instruction_elements);
        let mut original_payment_instructions = Vec::with_capacity(instruction_elements.len());
        for element in instruction_elements {
            original_payment_instructions.push(Self::read_instruction(element)?);
        }

        Ok(Self {
            message_id,
            creation_datetime,
            forwarding_agent_bic,
            debtor_agent_bic,
            creditor_agent_bic,
            original_group,
            original_payment_instructions,
        })
    }

    fn read_instruction(element: &Element) -> Result<OriginalPaymentInstruction> {
        let mut transaction_elements = Vec::new();
        find_all(element, "TxInfAndSts", &mut transaction_elements);
        let mut transaction_statuses = Vec::with_capacity(transaction_elements.len());
        for transaction in transaction_elements {
            transaction_statuses.push(Self::read_transaction(transaction));
        }
        Ok(OriginalPaymentInstruction {
            original_payment_instruction_id: required_text(element, "OrgnlPmtInfId")?,
            original_number_of_transactions: optional_text(element, "OrgnlNbOfTxs"),
            original_control_sum: optional_decimal(element, "OrgnlCtrlSum")?,
            payment_information_status: optional_text(element, "PmtInfSts"),
            // Direct children only: a transaction's own reasons are read below.
            status_reasons: read_status_reasons_direct(element),
            transaction_statuses,
        })
    }

    fn read_transaction(element: &Element) -> TransactionStatus {
        TransactionStatus {
            status_id: optional_text(element, "StsId"),
            original_instruction_id: optional_text(element, "OrgnlInstrId"),
            original_end_to_end_id: optional_text(element, "OrgnlEndToEndId"),
            original_uetr: optional_text(element, "OrgnlUETR"),
            transaction_status: optional_text(element, "TxSts"),
            status_reasons: read_status_reasons(element),
            acceptance_datetime: optional_text(element, "AccptncDtTm"),
            account_servicer_reference: optional_text(element, "AcctSvcrRef"),
            clearing_system_reference: optional_text(element, "ClrSysRef"),
        }
    }

    /// Writes the message as a `pain.002.001.10` document.
    ///
    /// Element order follows the schema sequence, which a validating parser
    /// requires: `GrpHdr` precedes `OrgnlGrpInfAndSts`, which precedes the
    /// `OrgnlPmtInfAndSts` blocks, and within one of those the instruction
    /// status precedes its transactions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when the report carries no group-level
    /// status. The schema requires `OrgnlGrpInfAndSts`, so a report without one
    /// would fail validation at the receiving bank.
    pub fn to_xml(&self) -> Result<String> {
        if self.original_group.original_message_id.is_empty() {
            return Err(Error::MissingElement("OrgnlGrpInfAndSts"));
        }
        let now = self
            .creation_datetime
            .as_deref()
            .unwrap_or("2026-01-01T00:00:00Z");
        let mut out = String::with_capacity(1024 + self.original_payment_instructions.len() * 512);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pain.002.001.10\">\n  <CstmrPmtStsRpt>\n",
        );
        out.push_str("    <GrpHdr>\n");
        let _ = writeln!(out, "      <MsgId>{}</MsgId>", escape(&self.message_id));
        let _ = writeln!(out, "      <CreDtTm>{}</CreDtTm>", escape(now));
        if let Some(bic) = &self.forwarding_agent_bic {
            write_agent(&mut out, "FwdgAgt", bic);
        }
        if let Some(bic) = &self.debtor_agent_bic {
            write_agent(&mut out, "DbtrAgt", bic);
        }
        if let Some(bic) = &self.creditor_agent_bic {
            write_agent(&mut out, "CdtrAgt", bic);
        }
        out.push_str("    </GrpHdr>\n");

        write_original_group_tagged(&mut out, 4, "OrgnlGrpInfAndSts", &self.original_group);

        for instruction in &self.original_payment_instructions {
            out.push_str("    <OrgnlPmtInfAndSts>\n");
            let _ = writeln!(
                out,
                "      <OrgnlPmtInfId>{}</OrgnlPmtInfId>",
                escape(&instruction.original_payment_instruction_id)
            );
            if let Some(n) = &instruction.original_number_of_transactions {
                let _ = writeln!(out, "      <OrgnlNbOfTxs>{}</OrgnlNbOfTxs>", escape(n));
            }
            if let Some(sum) = instruction.original_control_sum {
                let _ = writeln!(
                    out,
                    "      <OrgnlCtrlSum>{}</OrgnlCtrlSum>",
                    format_decimal(sum)
                );
            }
            if let Some(status) = &instruction.payment_information_status {
                let _ = writeln!(out, "      <PmtInfSts>{}</PmtInfSts>", escape(status));
            }
            write_status_reasons(&mut out, 6, &instruction.status_reasons);
            for transaction in &instruction.transaction_statuses {
                Self::write_transaction(&mut out, transaction);
            }
            out.push_str("    </OrgnlPmtInfAndSts>\n");
        }

        out.push_str("  </CstmrPmtStsRpt>\n</Document>\n");
        Ok(out)
    }

    fn write_transaction(out: &mut String, transaction: &TransactionStatus) {
        out.push_str("      <TxInfAndSts>\n");
        if let Some(id) = &transaction.status_id {
            let _ = writeln!(out, "        <StsId>{}</StsId>", escape(id));
        }
        if let Some(id) = &transaction.original_instruction_id {
            let _ = writeln!(out, "        <OrgnlInstrId>{}</OrgnlInstrId>", escape(id));
        }
        if let Some(id) = &transaction.original_end_to_end_id {
            let _ = writeln!(
                out,
                "        <OrgnlEndToEndId>{}</OrgnlEndToEndId>",
                escape(id)
            );
        }
        if let Some(uetr) = &transaction.original_uetr {
            let _ = writeln!(out, "        <OrgnlUETR>{}</OrgnlUETR>", escape(uetr));
        }
        if let Some(status) = &transaction.transaction_status {
            let _ = writeln!(out, "        <TxSts>{}</TxSts>", escape(status));
        }
        write_status_reasons(out, 8, &transaction.status_reasons);
        if let Some(dt) = &transaction.acceptance_datetime {
            let _ = writeln!(out, "        <AccptncDtTm>{}</AccptncDtTm>", escape(dt));
        }
        if let Some(reference) = &transaction.account_servicer_reference {
            let _ = writeln!(
                out,
                "        <AcctSvcrRef>{}</AcctSvcrRef>",
                escape(reference)
            );
        }
        if let Some(reference) = &transaction.clearing_system_reference {
            let _ = writeln!(out, "        <ClrSysRef>{}</ClrSysRef>", escape(reference));
        }
        out.push_str("      </TxInfAndSts>\n");
    }
}

/// Whether a control sum matches the sum of the entries it covers.
///
/// Banks reject a file whose control sum disagrees with its entries, so a
/// generator should check this before sending.
#[must_use]
pub fn control_sum_matches(entries: &[f64], control_sum: f64) -> bool {
    // Compare at the scale the format uses, two decimal places.
    let sum: f64 = entries.iter().sum();
    (sum - control_sum).abs() < 0.005
}

/// Sign of an entry, re-exported so callers do not need the `camt` module.
#[must_use]
pub fn entry_sign(indicator: CreditDebit) -> f64 {
    indicator.sign()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.09">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>MSG-2026-001</MsgId>
      <CreDtTm>2026-10-21T09:00:00Z</CreDtTm>
      <NbOfTxs>2</NbOfTxs>
      <CtrlSum>750.00</CtrlSum>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-1</PmtInfId>
      <ReqdExctnDt><Dt>2026-10-25</Dt></ReqdExctnDt>
      <Dbtr><Nm>Acme GmbH</Nm></Dbtr>
      <DbtrAcct><Id><IBAN>DE89370400440532013000</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></DbtrAgt>
      <CdtTrfTxInf>
        <PmtId><InstrId>INSTR-1</InstrId></PmtId>
        <Amt><InstdAmt Ccy="EUR">500.00</InstdAmt></Amt>
        <Cdtr><Nm>Supplier One</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>GB82WEST12345698765432</IBAN></Id></CdtrAcct>
        <RmtInf><Ustrd>Invoice 1001</Ustrd></RmtInf>
      </CdtTrfTxInf>
      <CdtTrfTxInf>
        <PmtId><InstrId>INSTR-2</InstrId></PmtId>
        <Amt><InstdAmt Ccy="EUR">250.00</InstdAmt></Amt>
        <Cdtr><Nm>Supplier Two</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>NL91ABNA0417164300</IBAN></Id></CdtrAcct>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#;

    #[test]
    fn parses_a_payment_initiation() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        assert_eq!(p.message_id, "MSG-2026-001");
        assert_eq!(p.debtor_name, "Acme GmbH");
        assert_eq!(p.debtor_iban, "DE89370400440532013000");
        assert_eq!(p.debtor_bic.as_deref(), Some("DEUTDEFF"));
        assert_eq!(p.requested_execution_date.as_deref(), Some("2026-10-25"));
        assert_eq!(p.number_of_transfers(), 2);
        assert_eq!(p.total(), 750.0);
    }

    #[test]
    fn a_message_without_a_debtor_bank_cannot_be_written() {
        // The schema marks DbtrAgt as required.
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.debtor_bic = None;
        assert_eq!(p.to_xml(), Err(Error::MissingElement("DbtrAgt")));
    }

    #[test]
    fn a_message_without_an_execution_date_cannot_be_written() {
        // The schema marks ReqdExctnDt as required.
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.requested_execution_date = None;
        assert_eq!(p.to_xml(), Err(Error::MissingElement("ReqdExctnDt")));
    }

    #[test]
    fn transfers_carry_their_details() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        let first = &p.transfers[0];
        assert_eq!(first.instruction_id, "INSTR-1");
        assert_eq!(first.amount.amount, 500.0);
        assert_eq!(first.amount.currency.as_str(), "EUR");
        assert_eq!(first.creditor_name, "Supplier One");
        assert_eq!(first.creditor_iban, "GB82WEST12345698765432");
        assert_eq!(first.remittance.as_deref(), Some("Invoice 1001"));
        assert!(p.transfers[1].remittance.is_none());
    }

    #[test]
    fn round_trip_through_generation() {
        let original = CreditTransferInitiation::parse(PAIN).unwrap();
        let xml = original.to_xml().unwrap();
        let reparsed = CreditTransferInitiation::parse(&xml).unwrap();
        assert_eq!(reparsed.message_id, original.message_id);
        assert_eq!(reparsed.debtor_iban, original.debtor_iban);
        assert_eq!(reparsed.transfers.len(), original.transfers.len());
        assert_eq!(reparsed.total(), original.total());
        assert_eq!(
            reparsed.transfers[0].creditor_iban,
            original.transfers[0].creditor_iban
        );
        assert_eq!(
            reparsed.transfers[0].remittance,
            original.transfers[0].remittance
        );
    }

    #[test]
    fn generated_control_sum_agrees_with_entries() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        let xml = p.to_xml().unwrap();
        let amounts: Vec<f64> = p.transfers.iter().map(|t| t.amount.amount).collect();
        assert!(control_sum_matches(&amounts, p.total()));
        assert!(xml.contains("<CtrlSum>750.00</CtrlSum>"));
        assert!(xml.contains("<NbOfTxs>2</NbOfTxs>"));
    }

    #[test]
    fn generated_xml_escapes_special_characters() {
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.transfers[0].creditor_name = "Smith & Sons <Ltd>".into();
        let xml = p.to_xml().unwrap();
        assert!(xml.contains("Smith &amp; Sons &lt;Ltd&gt;"));
        // Re-parsing must return the original text.
        let back = CreditTransferInitiation::parse(&xml).unwrap();
        assert_eq!(back.transfers[0].creditor_name, "Smith & Sons <Ltd>");
    }

    #[test]
    fn empty_batch_cannot_be_written() {
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.transfers.clear();
        assert_eq!(p.to_xml(), Err(Error::MissingElement("CdtTrfTxInf")));
    }

    #[test]
    fn missing_debtor_iban_is_an_error() {
        let xml = r#"<Document><CstmrCdtTrfInitn>
            <GrpHdr><MsgId>M</MsgId></GrpHdr>
            <PmtInf><Dbtr><Nm>X</Nm></Dbtr></PmtInf>
        </CstmrCdtTrfInitn></Document>"#;
        assert_eq!(
            CreditTransferInitiation::parse(xml),
            Err(Error::MissingElement("DbtrAcct/IBAN"))
        );
    }

    #[test]
    fn control_sum_comparison_uses_two_decimals() {
        assert!(control_sum_matches(&[0.1, 0.2], 0.3));
        assert!(!control_sum_matches(&[1.0], 1.01));
        assert!(control_sum_matches(&[1.0], 1.004));
    }

    #[test]
    fn entry_sign_follows_the_indicator() {
        assert_eq!(entry_sign(CreditDebit::Credit), 1.0);
        assert_eq!(entry_sign(CreditDebit::Debit), -1.0);
    }

    const PAIN002: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.002.001.10">
  <CstmrPmtStsRpt>
    <GrpHdr>
      <MsgId>STATUS-001</MsgId>
      <CreDtTm>2026-10-21T09:40:00Z</CreDtTm>
      <FwdgAgt><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></FwdgAgt>
      <DbtrAgt><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></DbtrAgt>
      <CdtrAgt><FinInstnId><BICFI>COBADEFF</BICFI></FinInstnId></CdtrAgt>
    </GrpHdr>
    <OrgnlGrpInfAndSts>
      <OrgnlMsgId>MSG-2026-001</OrgnlMsgId>
      <OrgnlMsgNmId>pain.001.001.09</OrgnlMsgNmId>
      <OrgnlNbOfTxs>2</OrgnlNbOfTxs>
      <OrgnlCtrlSum>750.00</OrgnlCtrlSum>
      <GrpSts>PART</GrpSts>
    </OrgnlGrpInfAndSts>
    <OrgnlPmtInfAndSts>
      <OrgnlPmtInfId>PMT-1</OrgnlPmtInfId>
      <OrgnlNbOfTxs>2</OrgnlNbOfTxs>
      <OrgnlCtrlSum>750.00</OrgnlCtrlSum>
      <PmtInfSts>PART</PmtInfSts>
      <StsRsnInf><Rsn><Cd>AC04</Cd></Rsn></StsRsnInf>
      <TxInfAndSts>
        <StsId>STS-1</StsId>
        <OrgnlInstrId>INSTR-1</OrgnlInstrId>
        <OrgnlEndToEndId>E2E-1</OrgnlEndToEndId>
        <TxSts>ACSC</TxSts>
        <AcctSvcrRef>ACCT-REF-1</AcctSvcrRef>
      </TxInfAndSts>
      <TxInfAndSts>
        <StsId>STS-2</StsId>
        <OrgnlInstrId>INSTR-2</OrgnlInstrId>
        <OrgnlEndToEndId>E2E-2</OrgnlEndToEndId>
        <TxSts>RJCT</TxSts>
        <StsRsnInf>
          <Rsn><Cd>AC01</Cd></Rsn>
          <AddtlInf>Account identifier invalid</AddtlInf>
        </StsRsnInf>
      </TxInfAndSts>
    </OrgnlPmtInfAndSts>
  </CstmrPmtStsRpt>
</Document>"#;

    #[test]
    fn parses_a_payment_status_report() {
        let r = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        assert_eq!(r.message_id, "STATUS-001");
        assert_eq!(r.forwarding_agent_bic.as_deref(), Some("DEUTDEFF"));
        assert_eq!(r.debtor_agent_bic.as_deref(), Some("DEUTDEFF"));
        assert_eq!(r.creditor_agent_bic.as_deref(), Some("COBADEFF"));
        assert_eq!(r.original_group.original_message_id, "MSG-2026-001");
        assert_eq!(r.original_group.original_message_name_id, "pain.001.001.09");
        assert_eq!(r.original_group.original_control_sum, Some(750.0));
        assert_eq!(r.original_group.group_status.as_deref(), Some("PART"));
    }

    #[test]
    fn instruction_and_transaction_outcomes_are_read() {
        let r = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        assert_eq!(r.original_payment_instructions.len(), 1);
        let instruction = &r.original_payment_instructions[0];
        assert_eq!(instruction.original_payment_instruction_id, "PMT-1");
        assert_eq!(
            instruction.payment_information_status.as_deref(),
            Some("PART")
        );
        assert_eq!(instruction.original_control_sum, Some(750.0));
        assert_eq!(instruction.transaction_statuses.len(), 2);

        let accepted = &instruction.transaction_statuses[0];
        assert_eq!(accepted.original_instruction_id.as_deref(), Some("INSTR-1"));
        assert_eq!(accepted.transaction_status.as_deref(), Some("ACSC"));
        assert_eq!(
            accepted.account_servicer_reference.as_deref(),
            Some("ACCT-REF-1")
        );
        assert!(accepted.status_reasons.is_empty());

        let rejected = &instruction.transaction_statuses[1];
        assert_eq!(rejected.transaction_status.as_deref(), Some("RJCT"));
        assert_eq!(rejected.status_reasons.len(), 1);
        assert_eq!(rejected.status_reasons[0].code.as_deref(), Some("AC01"));
        assert_eq!(
            rejected.status_reasons[0].additional_information,
            ["Account identifier invalid"]
        );
    }

    #[test]
    fn instruction_reasons_are_not_confused_with_transaction_reasons() {
        // The instruction carries AC04 and a transaction carries AC01; a
        // recursive read would report both at the instruction level.
        let r = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        let instruction = &r.original_payment_instructions[0];
        assert_eq!(instruction.status_reasons.len(), 1);
        assert_eq!(instruction.status_reasons[0].code.as_deref(), Some("AC04"));
    }

    #[test]
    fn status_report_round_trips_through_generation() {
        let original = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        let xml = original.to_xml().unwrap();
        let reparsed = CustomerPaymentStatusReport::parse(&xml).unwrap();
        assert_eq!(reparsed.message_id, original.message_id);
        assert_eq!(reparsed.original_group, original.original_group);
        assert_eq!(
            reparsed.original_payment_instructions,
            original.original_payment_instructions
        );
        assert_eq!(reparsed.creditor_agent_bic, original.creditor_agent_bic);
    }

    #[test]
    fn status_report_without_a_group_status_cannot_be_written() {
        let mut r = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        r.original_group.original_message_id.clear();
        assert_eq!(r.to_xml(), Err(Error::MissingElement("OrgnlGrpInfAndSts")));
    }

    #[test]
    fn status_report_missing_the_group_header_is_an_error() {
        let xml = r#"<Document><CstmrPmtStsRpt>
            <GrpHdr><MsgId>M</MsgId></GrpHdr>
        </CstmrPmtStsRpt></Document>"#;
        assert_eq!(
            CustomerPaymentStatusReport::parse(xml),
            Err(Error::MissingElement("OrgnlGrpInfAndSts"))
        );
    }

    #[test]
    fn status_report_escapes_special_characters() {
        let mut r = CustomerPaymentStatusReport::parse(PAIN002).unwrap();
        r.original_payment_instructions[0].transaction_statuses[1].status_reasons[0]
            .additional_information = vec!["A & B <C>".into()];
        let xml = r.to_xml().unwrap();
        assert!(xml.contains("A &amp; B &lt;C&gt;"));
        let back = CustomerPaymentStatusReport::parse(&xml).unwrap();
        assert_eq!(
            back.original_payment_instructions[0].transaction_statuses[1].status_reasons[0]
                .additional_information,
            ["A & B <C>"]
        );
    }

    #[test]
    fn a_status_report_with_no_instructions_still_round_trips() {
        // Only the group header is required, so a report may name no
        // instruction; the group outcome alone is still a valid message.
        let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.002.001.10">
          <CstmrPmtStsRpt>
            <GrpHdr><MsgId>S1</MsgId><CreDtTm>2026-10-21T09:40:00Z</CreDtTm></GrpHdr>
            <OrgnlGrpInfAndSts>
              <OrgnlMsgId>MSG-1</OrgnlMsgId>
              <OrgnlMsgNmId>pain.001.001.09</OrgnlMsgNmId>
              <GrpSts>RJCT</GrpSts>
            </OrgnlGrpInfAndSts>
          </CstmrPmtStsRpt>
        </Document>"#;
        let r = CustomerPaymentStatusReport::parse(xml).unwrap();
        assert!(r.original_payment_instructions.is_empty());
        let back = CustomerPaymentStatusReport::parse(&r.to_xml().unwrap()).unwrap();
        assert_eq!(back.original_group.group_status.as_deref(), Some("RJCT"));
    }
}
