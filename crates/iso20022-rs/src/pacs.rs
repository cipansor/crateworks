//! `pacs` payment clearing and settlement messages.
//!
//! The main message here is `pacs.008`, the interbank customer credit transfer:
//! the message one bank sends another to move money for a customer. It is the
//! ISO 20022 replacement for the SWIFT MT103, and it is what the `pain.001`
//! message in this crate ultimately turns into once the debtor's bank has
//! accepted it.
//!
//! [`PaymentStatusReport`] is `pacs.002`, the answer: whether the transfer, or
//! the batch it was part of, was accepted, settled or rejected, and why. It is
//! the ISO 20022 replacement for the SWIFT MT199 and MT299 status messages, and
//! reading one is the other half of sending a `pacs.008`.
//!
//! [`CreditTransfer::parse`] and [`PaymentStatusReport::parse`] read a whole
//! document, and each type's `to_xml` writes one. The field names follow the
//! ISO 20022 schema so that a reader who knows the standard can map the code
//! back to it without a lookup table.

use std::fmt::Write as _;
use std::fmt::{self, Display};

use crate::amount::Amount;
use crate::{Error, Result, find, find_all, optional_text, required_text};
use xmlkit::Element;

/// How the interbank settlement is made, the ISO 20022 `SettlementMethod1Code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementMethod {
    /// Settlement on an individual, instructed basis.
    InstructedAgent,
    /// Settlement on an individual, agreed basis.
    InstructedReimbursementAgent,
    /// Settlement on a cover basis, through a cover account.
    Cover,
    /// Settlement through a clearing system.
    Clearing,
}

impl SettlementMethod {
    /// Reads an ISO 20022 `SttlmMtd` element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] for a code outside the schema's four.
    pub fn parse(text: &str) -> Result<Self> {
        match text.trim() {
            "INDA" => Ok(Self::InstructedAgent),
            "INGA" => Ok(Self::InstructedReimbursementAgent),
            "COVE" => Ok(Self::Cover),
            "CLRG" => Ok(Self::Clearing),
            other => Err(Error::InvalidValue {
                element: "SttlmMtd",
                found: other.to_string(),
            }),
        }
    }

    /// The ISO 20022 four-letter code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InstructedAgent => "INDA",
            Self::InstructedReimbursementAgent => "INGA",
            Self::Cover => "COVE",
            Self::Clearing => "CLRG",
        }
    }
}

impl Display for SettlementMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Who bears the interbank charges, the ISO 20022 `ChargeBearerType1Code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeBearer {
    /// All charges borne by the debtor.
    BorneByDebtor,
    /// All charges borne by the creditor.
    BorneByCreditor,
    /// Charges shared between debtor and creditor.
    Shared,
    /// Charges follow the service level, the usual SEPA choice.
    FollowingServiceLevel,
}

impl ChargeBearer {
    /// Reads an ISO 20022 `ChrgBr` element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] for a code outside the schema's four.
    pub fn parse(text: &str) -> Result<Self> {
        match text.trim() {
            "DEBT" => Ok(Self::BorneByDebtor),
            "CRED" => Ok(Self::BorneByCreditor),
            "SHAR" => Ok(Self::Shared),
            "SLEV" => Ok(Self::FollowingServiceLevel),
            other => Err(Error::InvalidValue {
                element: "ChrgBr",
                found: other.to_string(),
            }),
        }
    }

    /// The ISO 20022 four-letter code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BorneByDebtor => "DEBT",
            Self::BorneByCreditor => "CRED",
            Self::Shared => "SHAR",
            Self::FollowingServiceLevel => "SLEV",
        }
    }
}

impl Display for ChargeBearer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Why a payment or a whole message was not accepted, an ISO 20022
/// `StatusReasonInformation12`.
///
/// The reason is either an external status code (`Rsn/Cd`, from the ISO 20022
/// code list) or a proprietary one (`Rsn/Prtry`). Both are kept as written
/// rather than mapped to an enum, because the external list is large and grows
/// without a schema change; a caller that needs to recognise a code compares
/// the string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReason {
    /// The external ISO 20022 status reason code, such as `AC01`.
    pub code: Option<String>,
    /// A proprietary reason, used when the code list has no match.
    pub proprietary: Option<String>,
    /// Additional, free-text information about the reason.
    pub additional_information: Vec<String>,
}

/// The message a status report answers, an ISO 20022 `OriginalGroupInformation29`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalGroupInfo {
    /// Identifier of the message being reported on.
    pub original_message_id: String,
    /// Its message name, for example `pacs.008.001.08`.
    pub original_message_name_id: String,
    /// When it was created, if stated.
    pub original_creation_datetime: Option<String>,
}

/// The group-level outcome of a whole message, an ISO 20022
/// `OriginalGroupHeader17`.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalGroupStatus {
    /// Identifier of the message being reported on.
    pub original_message_id: String,
    /// Its message name, for example `pacs.008.001.08`.
    pub original_message_name_id: String,
    /// When it was created, if stated.
    pub original_creation_datetime: Option<String>,
    /// The transaction count it claimed, as written.
    pub original_number_of_transactions: Option<String>,
    /// The control sum it claimed, if stated.
    pub original_control_sum: Option<f64>,
    /// The group status code, such as `ACCP` or `RJCT`.
    pub group_status: Option<String>,
    /// The reasons given for the group status.
    pub status_reasons: Vec<StatusReason>,
}

/// The outcome of one transaction, an ISO 20022 `TxInfAndSts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionStatus {
    /// Identifier for this status, if the reporter assigned one.
    pub status_id: Option<String>,
    /// The message this transaction came from, when stated here.
    pub original_group_info: Option<OriginalGroupInfo>,
    /// The instruction identifier being reported on.
    pub original_instruction_id: Option<String>,
    /// The end-to-end identifier being reported on, the field a reconciliation
    /// matches on.
    pub original_end_to_end_id: Option<String>,
    /// The transaction identifier being reported on.
    pub original_transaction_id: Option<String>,
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

/// A `pacs.002` payment status report.
///
/// This is the message a bank sends to say what happened to a payment: whether
/// a `pacs.008` transfer, or a whole batch of them, was accepted, settled or
/// rejected, and why. It is the ISO 20022 replacement for the SWIFT MT199 and
/// MT299 status messages. A payment that a `pacs.008` sent is not known to have
/// succeeded until its `pacs.002` comes back, so reading one is the other half
/// of sending one.
#[derive(Debug, Clone, PartialEq)]
pub struct PaymentStatusReport {
    /// Message identifier, assigned by the sending bank.
    pub message_id: String,
    /// Creation date and time, kept as written.
    pub creation_datetime: Option<String>,
    /// Instructing bank BIC, if present.
    pub instructing_agent_bic: Option<String>,
    /// Instructed bank BIC, if present.
    pub instructed_agent_bic: Option<String>,
    /// Group-level outcomes, in document order.
    pub original_groups: Vec<OriginalGroupStatus>,
    /// Per-transaction outcomes, in document order.
    pub statuses: Vec<TransactionStatus>,
}

/// One interbank credit transfer, an ISO 20022 `CdtTrfTxInf`.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransferTransaction {
    /// Instruction identifier, unique within the message. Optional in the
    /// schema and absent from many messages.
    pub instruction_id: Option<String>,
    /// End-to-end identifier, assigned by the debtor and passed through
    /// unchanged. This is the field a reconciliation matches on.
    pub end_to_end_id: String,
    /// The UETR, the unique end-to-end transaction reference a SWIFT gpi
    /// transfer carries, if present.
    pub uetr: Option<String>,
    /// The interbank settlement amount.
    pub amount: Amount,
    /// Who bears the charges.
    pub charge_bearer: ChargeBearer,
    /// Name of the debtor.
    pub debtor_name: String,
    /// Debtor account IBAN, if the account is identified that way.
    pub debtor_iban: Option<String>,
    /// Debtor bank BIC, if present.
    pub debtor_agent_bic: Option<String>,
    /// Name of the creditor.
    pub creditor_name: String,
    /// Creditor account IBAN, if the account is identified that way.
    pub creditor_iban: Option<String>,
    /// Creditor bank BIC, if present.
    pub creditor_agent_bic: Option<String>,
    /// Remittance information, the free text a payer supplied.
    pub remittance: Option<String>,
}

/// A `pacs.008` interbank customer credit transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransfer {
    /// Message identifier, assigned by the sending bank.
    pub message_id: String,
    /// Creation date and time, kept as written.
    pub creation_datetime: Option<String>,
    /// How the interbank settlement is made.
    pub settlement_method: SettlementMethod,
    /// The control sum as it appeared in the message, when one was present.
    ///
    /// Generation computes the sum from the transactions instead, so this is
    /// only the value a received message carried. Compare it with
    /// [`CreditTransfer::total`] to check a file before trusting it.
    pub control_sum: Option<f64>,
    /// The transfers, in document order.
    pub transactions: Vec<CreditTransferTransaction>,
}

impl CreditTransfer {
    /// Reads a complete `pacs.008` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input, [`Error::MissingElement`]
    /// when a required element is absent, or [`Error::InvalidValue`] for a
    /// malformed amount, settlement method or charge bearer.
    ///
    /// # Example
    ///
    /// ```
    /// use iso20022_rs::pacs::CreditTransfer;
    ///
    /// let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
    ///   <FIToFICstmrCdtTrf>
    ///     <GrpHdr><MsgId>MSG-1</MsgId><NbOfTxs>1</NbOfTxs>
    ///       <SttlmInf><SttlmMtd>CLRG</SttlmMtd></SttlmInf></GrpHdr>
    ///     <CdtTrfTxInf>
    ///       <PmtId><EndToEndId>E2E-1</EndToEndId></PmtId>
    ///       <IntrBkSttlmAmt Ccy="EUR">100.00</IntrBkSttlmAmt>
    ///       <ChrgBr>SLEV</ChrgBr>
    ///       <Dbtr><Nm>Alice</Nm></Dbtr>
    ///       <Cdtr><Nm>Bob</Nm></Cdtr>
    ///     </CdtTrfTxInf>
    ///   </FIToFICstmrCdtTrf>
    /// </Document>"#;
    ///
    /// let transfer = CreditTransfer::parse(xml).unwrap();
    /// assert_eq!(transfer.message_id, "MSG-1");
    /// assert_eq!(transfer.transactions[0].amount.amount, 100.0);
    /// assert_eq!(transfer.total(), 100.0);
    /// ```
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let group = find(&body, "GrpHdr").ok_or(Error::MissingElement("GrpHdr"))?;
        let message_id = required_text(group, "MsgId")?;
        let creation_datetime = optional_text(group, "CreDtTm");
        let settlement_method = SettlementMethod::parse(&required_text(
            find(group, "SttlmInf").ok_or(Error::MissingElement("SttlmInf"))?,
            "SttlmMtd",
        )?)?;
        let control_sum = find(group, "CtrlSum")
            .map(Element::deep_text)
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .map(|t| {
                t.parse::<f64>().map_err(|_| Error::InvalidValue {
                    element: "CtrlSum",
                    found: t,
                })
            })
            .transpose()?;

        let mut elements = Vec::new();
        find_all(&body, "CdtTrfTxInf", &mut elements);
        let mut transactions = Vec::with_capacity(elements.len());
        for tx in elements {
            transactions.push(Self::read_transaction(tx)?);
        }
        Ok(Self {
            message_id,
            creation_datetime,
            settlement_method,
            control_sum,
            transactions,
        })
    }

    fn read_transaction(tx: &Element) -> Result<CreditTransferTransaction> {
        let pmt_id = find(tx, "PmtId").ok_or(Error::MissingElement("PmtId"))?;
        let end_to_end_id = required_text(pmt_id, "EndToEndId")?;
        let instruction_id = optional_text(pmt_id, "InstrId");
        let uetr = optional_text(pmt_id, "UETR");

        let amt_el = find(tx, "IntrBkSttlmAmt").ok_or(Error::MissingElement("IntrBkSttlmAmt"))?;
        let amount = Amount::from_element(amt_el)?;
        let charge_bearer = ChargeBearer::parse(&required_text(tx, "ChrgBr")?)?;

        let debtor_name = find(tx, "Dbtr")
            .and_then(|d| optional_text(d, "Nm"))
            .ok_or(Error::MissingElement("Dbtr/Nm"))?;
        let debtor_iban = find(tx, "DbtrAcct").and_then(|a| optional_text(a, "IBAN"));
        let debtor_agent_bic = find(tx, "DbtrAgt").and_then(agent_bic);

        let creditor_name = find(tx, "Cdtr")
            .and_then(|c| optional_text(c, "Nm"))
            .ok_or(Error::MissingElement("Cdtr/Nm"))?;
        let creditor_iban = find(tx, "CdtrAcct").and_then(|a| optional_text(a, "IBAN"));
        let creditor_agent_bic = find(tx, "CdtrAgt").and_then(agent_bic);

        let remittance = find(tx, "RmtInf").and_then(|r| optional_text(r, "Ustrd"));

        Ok(CreditTransferTransaction {
            instruction_id,
            end_to_end_id,
            uetr,
            amount,
            charge_bearer,
            debtor_name,
            debtor_iban,
            debtor_agent_bic,
            creditor_name,
            creditor_iban,
            creditor_agent_bic,
            remittance,
        })
    }

    /// The sum of the transaction amounts, in the currency of the first one.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.transactions.iter().map(|t| t.amount.amount).sum()
    }

    /// The number of transactions, which the control sum must agree with.
    #[must_use]
    pub fn number_of_transactions(&self) -> usize {
        self.transactions.len()
    }

    /// Whether the control sum in the message agrees with the transactions.
    ///
    /// Returns `true` when the message carried no control sum, since there is
    /// then nothing to disagree with. A bank rejects a file whose control sum
    /// does not match its transactions, so this is worth checking on receipt.
    #[must_use]
    pub fn control_sum_matches(&self) -> bool {
        match self.control_sum {
            None => true,
            Some(sum) => crate::pain::control_sum_matches(
                &self
                    .transactions
                    .iter()
                    .map(|t| t.amount.amount)
                    .collect::<Vec<_>>(),
                sum,
            ),
        }
    }

    /// Writes the message as a `pacs.008.001.08` document.
    ///
    /// The control sum and the transaction count are computed from the
    /// transactions rather than taken from the caller, so the output is
    /// internally consistent by construction. Element order follows the
    /// schema sequence, which a validating parser requires.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when there are no transactions, or
    /// when a transaction has no debtor or creditor bank BIC. The schema marks
    /// `DbtrAgt` and `CdtrAgt` as required, so a message without them is not
    /// valid and a bank would reject it; refusing to write one is better than
    /// emitting a document that fails validation.
    pub fn to_xml(&self) -> Result<String> {
        if self.transactions.is_empty() {
            return Err(Error::MissingElement("CdtTrfTxInf"));
        }
        for t in &self.transactions {
            if t.debtor_agent_bic.is_none() {
                return Err(Error::MissingElement("DbtrAgt"));
            }
            if t.creditor_agent_bic.is_none() {
                return Err(Error::MissingElement("CdtrAgt"));
            }
        }
        let now = self
            .creation_datetime
            .as_deref()
            .unwrap_or("2026-01-01T00:00:00Z");
        let mut out = String::with_capacity(1024 + self.transactions.len() * 640);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08\">\n  <FIToFICstmrCdtTrf>\n",
        );
        out.push_str("    <GrpHdr>\n");
        let _ = writeln!(out, "      <MsgId>{}</MsgId>", escape(&self.message_id));
        let _ = writeln!(out, "      <CreDtTm>{}</CreDtTm>", escape(now));
        let _ = writeln!(out, "      <NbOfTxs>{}</NbOfTxs>", self.transactions.len());
        let _ = writeln!(
            out,
            "      <CtrlSum>{}</CtrlSum>",
            format_decimal(self.total())
        );
        out.push_str("      <SttlmInf>\n");
        let _ = writeln!(
            out,
            "        <SttlmMtd>{}</SttlmMtd>",
            self.settlement_method
        );
        out.push_str("      </SttlmInf>\n    </GrpHdr>\n");

        for t in &self.transactions {
            out.push_str("    <CdtTrfTxInf>\n      <PmtId>\n");
            if let Some(instr) = &t.instruction_id {
                let _ = writeln!(out, "        <InstrId>{}</InstrId>", escape(instr));
            }
            let _ = writeln!(
                out,
                "        <EndToEndId>{}</EndToEndId>",
                escape(&t.end_to_end_id)
            );
            if let Some(uetr) = &t.uetr {
                let _ = writeln!(out, "        <UETR>{}</UETR>", escape(uetr));
            }
            out.push_str("      </PmtId>\n");
            let _ = writeln!(
                out,
                "      <IntrBkSttlmAmt Ccy=\"{}\">{}</IntrBkSttlmAmt>",
                escape(t.amount.currency.as_str()),
                format_decimal(t.amount.amount)
            );
            let _ = writeln!(out, "      <ChrgBr>{}</ChrgBr>", t.charge_bearer);

            out.push_str("      <Dbtr>\n");
            let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&t.debtor_name));
            out.push_str("      </Dbtr>\n");
            if let Some(iban) = &t.debtor_iban {
                out.push_str("      <DbtrAcct>\n        <Id>\n");
                let _ = writeln!(out, "          <IBAN>{}</IBAN>", escape(iban));
                out.push_str("        </Id>\n      </DbtrAcct>\n");
            }
            if let Some(bic) = &t.debtor_agent_bic {
                write_agent(&mut out, "DbtrAgt", bic);
            }
            // `CdtrAgt` precedes `Cdtr` in the schema sequence.
            if let Some(bic) = &t.creditor_agent_bic {
                write_agent(&mut out, "CdtrAgt", bic);
            }
            out.push_str("      <Cdtr>\n");
            let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&t.creditor_name));
            out.push_str("      </Cdtr>\n");
            if let Some(iban) = &t.creditor_iban {
                out.push_str("      <CdtrAcct>\n        <Id>\n");
                let _ = writeln!(out, "          <IBAN>{}</IBAN>", escape(iban));
                out.push_str("        </Id>\n      </CdtrAcct>\n");
            }
            if let Some(remit) = &t.remittance {
                out.push_str("      <RmtInf>\n");
                let _ = writeln!(out, "        <Ustrd>{}</Ustrd>", escape(remit));
                out.push_str("      </RmtInf>\n");
            }
            out.push_str("    </CdtTrfTxInf>\n");
        }
        out.push_str("  </FIToFICstmrCdtTrf>\n</Document>\n");
        Ok(out)
    }
}

impl PaymentStatusReport {
    /// Reads a complete `pacs.002` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input, [`Error::MissingElement`]
    /// when a required element is absent, or [`Error::InvalidValue`] for a
    /// malformed control sum.
    ///
    /// # Example
    ///
    /// ```
    /// use iso20022_rs::pacs::PaymentStatusReport;
    ///
    /// let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
    ///   <FIToFIPmtStsRpt>
    ///     <GrpHdr><MsgId>STS-1</MsgId><CreDtTm>2026-10-21T09:35:00Z</CreDtTm></GrpHdr>
    ///     <TxInfAndSts>
    ///       <OrgnlEndToEndId>E2E-1</OrgnlEndToEndId>
    ///       <TxSts>ACSC</TxSts>
    ///     </TxInfAndSts>
    ///   </FIToFIPmtStsRpt>
    /// </Document>"#;
    ///
    /// let report = PaymentStatusReport::parse(xml).unwrap();
    /// assert_eq!(report.message_id, "STS-1");
    /// assert_eq!(report.statuses[0].transaction_status.as_deref(), Some("ACSC"));
    /// assert_eq!(report.statuses[0].original_end_to_end_id.as_deref(), Some("E2E-1"));
    /// ```
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let group = find(&body, "GrpHdr").ok_or(Error::MissingElement("GrpHdr"))?;
        let message_id = required_text(group, "MsgId")?;
        let creation_datetime = optional_text(group, "CreDtTm");
        let instructing_agent_bic = find(group, "InstgAgt").and_then(agent_bic);
        let instructed_agent_bic = find(group, "InstdAgt").and_then(agent_bic);

        let mut group_elements = Vec::new();
        find_all(&body, "OrgnlGrpInfAndSts", &mut group_elements);
        let mut original_groups = Vec::with_capacity(group_elements.len());
        for element in group_elements {
            original_groups.push(Self::read_original_group(element)?);
        }

        let mut status_elements = Vec::new();
        find_all(&body, "TxInfAndSts", &mut status_elements);
        let mut statuses = Vec::with_capacity(status_elements.len());
        for element in status_elements {
            statuses.push(Self::read_transaction_status(element)?);
        }

        Ok(Self {
            message_id,
            creation_datetime,
            instructing_agent_bic,
            instructed_agent_bic,
            original_groups,
            statuses,
        })
    }

    fn read_original_group(element: &Element) -> Result<OriginalGroupStatus> {
        let original_message_id = required_text(element, "OrgnlMsgId")?;
        let original_message_name_id = required_text(element, "OrgnlMsgNmId")?;
        let original_creation_datetime = optional_text(element, "OrgnlCreDtTm");
        let original_number_of_transactions = optional_text(element, "OrgnlNbOfTxs");
        let original_control_sum = optional_decimal(element, "OrgnlCtrlSum")?;
        let group_status = optional_text(element, "GrpSts");
        Ok(OriginalGroupStatus {
            original_message_id,
            original_message_name_id,
            original_creation_datetime,
            original_number_of_transactions,
            original_control_sum,
            group_status,
            status_reasons: read_status_reasons(element),
        })
    }

    fn read_transaction_status(element: &Element) -> Result<TransactionStatus> {
        let original_group_info = match find(element, "OrgnlGrpInf") {
            None => None,
            Some(group) => Some(OriginalGroupInfo {
                original_message_id: required_text(group, "OrgnlMsgId")?,
                original_message_name_id: required_text(group, "OrgnlMsgNmId")?,
                original_creation_datetime: optional_text(group, "OrgnlCreDtTm"),
            }),
        };
        Ok(TransactionStatus {
            status_id: optional_text(element, "StsId"),
            original_group_info,
            original_instruction_id: optional_text(element, "OrgnlInstrId"),
            original_end_to_end_id: optional_text(element, "OrgnlEndToEndId"),
            original_transaction_id: optional_text(element, "OrgnlTxId"),
            original_uetr: optional_text(element, "OrgnlUETR"),
            transaction_status: optional_text(element, "TxSts"),
            status_reasons: read_status_reasons(element),
            acceptance_datetime: optional_text(element, "AccptncDtTm"),
            account_servicer_reference: optional_text(element, "AcctSvcrRef"),
            clearing_system_reference: optional_text(element, "ClrSysRef"),
        })
    }

    /// Writes the message as a `pacs.002.001.10` document.
    ///
    /// Element order follows the schema sequence, which a validating parser
    /// requires: at group level `OrgnlGrpInfAndSts` precedes `TxInfAndSts`, and
    /// within a transaction the original identifiers precede the status.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when the report carries neither a
    /// group status nor a transaction status. The schema permits an empty
    /// report, but a status report that reports nothing is a mistake worth
    /// catching rather than writing out.
    pub fn to_xml(&self) -> Result<String> {
        if self.original_groups.is_empty() && self.statuses.is_empty() {
            return Err(Error::MissingElement("TxInfAndSts"));
        }
        let now = self
            .creation_datetime
            .as_deref()
            .unwrap_or("2026-01-01T00:00:00Z");
        let mut out = String::with_capacity(1024 + self.statuses.len() * 512);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10\">\n  <FIToFIPmtStsRpt>\n",
        );
        out.push_str("    <GrpHdr>\n");
        let _ = writeln!(out, "      <MsgId>{}</MsgId>", escape(&self.message_id));
        let _ = writeln!(out, "      <CreDtTm>{}</CreDtTm>", escape(now));
        if let Some(bic) = &self.instructing_agent_bic {
            write_agent(&mut out, "InstgAgt", bic);
        }
        if let Some(bic) = &self.instructed_agent_bic {
            write_agent(&mut out, "InstdAgt", bic);
        }
        out.push_str("    </GrpHdr>\n");

        for group in &self.original_groups {
            out.push_str("    <OrgnlGrpInfAndSts>\n");
            let _ = writeln!(
                out,
                "      <OrgnlMsgId>{}</OrgnlMsgId>",
                escape(&group.original_message_id)
            );
            let _ = writeln!(
                out,
                "      <OrgnlMsgNmId>{}</OrgnlMsgNmId>",
                escape(&group.original_message_name_id)
            );
            if let Some(dt) = &group.original_creation_datetime {
                let _ = writeln!(out, "      <OrgnlCreDtTm>{}</OrgnlCreDtTm>", escape(dt));
            }
            if let Some(n) = &group.original_number_of_transactions {
                let _ = writeln!(out, "      <OrgnlNbOfTxs>{}</OrgnlNbOfTxs>", escape(n));
            }
            if let Some(sum) = group.original_control_sum {
                let _ = writeln!(
                    out,
                    "      <OrgnlCtrlSum>{}</OrgnlCtrlSum>",
                    format_decimal(sum)
                );
            }
            if let Some(status) = &group.group_status {
                let _ = writeln!(out, "      <GrpSts>{}</GrpSts>", escape(status));
            }
            write_status_reasons(&mut out, 6, &group.status_reasons);
            out.push_str("    </OrgnlGrpInfAndSts>\n");
        }

        for status in &self.statuses {
            out.push_str("    <TxInfAndSts>\n");
            if let Some(id) = &status.status_id {
                let _ = writeln!(out, "      <StsId>{}</StsId>", escape(id));
            }
            if let Some(group) = &status.original_group_info {
                out.push_str("      <OrgnlGrpInf>\n");
                let _ = writeln!(
                    out,
                    "        <OrgnlMsgId>{}</OrgnlMsgId>",
                    escape(&group.original_message_id)
                );
                let _ = writeln!(
                    out,
                    "        <OrgnlMsgNmId>{}</OrgnlMsgNmId>",
                    escape(&group.original_message_name_id)
                );
                if let Some(dt) = &group.original_creation_datetime {
                    let _ = writeln!(out, "        <OrgnlCreDtTm>{}</OrgnlCreDtTm>", escape(dt));
                }
                out.push_str("      </OrgnlGrpInf>\n");
            }
            if let Some(id) = &status.original_instruction_id {
                let _ = writeln!(out, "      <OrgnlInstrId>{}</OrgnlInstrId>", escape(id));
            }
            if let Some(id) = &status.original_end_to_end_id {
                let _ = writeln!(
                    out,
                    "      <OrgnlEndToEndId>{}</OrgnlEndToEndId>",
                    escape(id)
                );
            }
            if let Some(id) = &status.original_transaction_id {
                let _ = writeln!(out, "      <OrgnlTxId>{}</OrgnlTxId>", escape(id));
            }
            if let Some(uetr) = &status.original_uetr {
                let _ = writeln!(out, "      <OrgnlUETR>{}</OrgnlUETR>", escape(uetr));
            }
            if let Some(tx_status) = &status.transaction_status {
                let _ = writeln!(out, "      <TxSts>{}</TxSts>", escape(tx_status));
            }
            write_status_reasons(&mut out, 6, &status.status_reasons);
            if let Some(dt) = &status.acceptance_datetime {
                let _ = writeln!(out, "      <AccptncDtTm>{}</AccptncDtTm>", escape(dt));
            }
            if let Some(reference) = &status.account_servicer_reference {
                let _ = writeln!(
                    out,
                    "      <AcctSvcrRef>{}</AcctSvcrRef>",
                    escape(reference)
                );
            }
            if let Some(reference) = &status.clearing_system_reference {
                let _ = writeln!(out, "      <ClrSysRef>{}</ClrSysRef>", escape(reference));
            }
            out.push_str("    </TxInfAndSts>\n");
        }

        out.push_str("  </FIToFIPmtStsRpt>\n</Document>\n");
        Ok(out)
    }
}

/// Reads every `StsRsnInf` under `parent` into a reason.
fn read_status_reasons(parent: &Element) -> Vec<StatusReason> {
    let mut elements = Vec::new();
    find_all(parent, "StsRsnInf", &mut elements);
    elements
        .iter()
        .map(|element| {
            let reason = find(element, "Rsn");
            let code = reason.and_then(|r| optional_text(r, "Cd"));
            let proprietary = reason.and_then(|r| optional_text(r, "Prtry"));
            let mut additional = Vec::new();
            find_all(element, "AddtlInf", &mut additional);
            StatusReason {
                code,
                proprietary,
                additional_information: additional
                    .iter()
                    .filter_map(|e| {
                        let text = e.deep_text().trim().to_string();
                        (!text.is_empty()).then_some(text)
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Writes `StsRsnInf` blocks at the given indentation.
fn write_status_reasons(out: &mut String, indent: usize, reasons: &[StatusReason]) {
    let pad = " ".repeat(indent);
    for reason in reasons {
        let _ = writeln!(out, "{pad}<StsRsnInf>");
        if let Some(code) = &reason.code {
            let _ = writeln!(out, "{pad}  <Rsn><Cd>{}</Cd></Rsn>", escape(code));
        } else if let Some(proprietary) = &reason.proprietary {
            let _ = writeln!(
                out,
                "{pad}  <Rsn><Prtry>{}</Prtry></Rsn>",
                escape(proprietary)
            );
        }
        for info in &reason.additional_information {
            let _ = writeln!(out, "{pad}  <AddtlInf>{}</AddtlInf>", escape(info));
        }
        let _ = writeln!(out, "{pad}</StsRsnInf>");
    }
}

/// Reads an optional decimal child, as the control sums are.
fn optional_decimal(element: &Element, name: &'static str) -> Result<Option<f64>> {
    match optional_text(element, name) {
        None => Ok(None),
        Some(text) => text
            .parse::<f64>()
            .map(Some)
            .map_err(|_| Error::InvalidValue {
                element: name,
                found: text,
            }),
    }
}

/// Writes a `<XxxAgt><FinInstnId><BICFI>..</BICFI></FinInstnId></XxxAgt>` block.
fn write_agent(out: &mut String, tag: &str, bic: &str) {
    let _ = writeln!(
        out,
        "      <{tag}>\n        <FinInstnId>\n          <BICFI>{}</BICFI>\n        </FinInstnId>\n      </{tag}>",
        escape(bic)
    );
}

/// Reads a BIC from an agent element such as `<DbtrAgt>`.
///
/// The code lives in `FinInstnId` and may be tagged `BICFI` (the current name)
/// or `BIC` (the older one), so both are accepted.
fn agent_bic(agent: &Element) -> Option<String> {
    let fin = find(agent, "FinInstnId").unwrap_or(agent);
    optional_text(fin, "BICFI").or_else(|| optional_text(fin, "BIC"))
}

/// Formats a decimal with two places, the usual convention for currency.
fn format_decimal(value: f64) -> String {
    format!("{value:.2}")
}

/// Escapes text for XML content.
fn escape(s: &str) -> String {
    xmlkit::escape_text(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACS008: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>MSG-001</MsgId>
      <CreDtTm>2026-10-21T09:30:00Z</CreDtTm>
      <NbOfTxs>2</NbOfTxs>
      <CtrlSum>150.00</CtrlSum>
      <SttlmInf><SttlmMtd>CLRG</SttlmMtd></SttlmInf>
    </GrpHdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>INSTR-1</InstrId>
        <EndToEndId>E2E-1</EndToEndId>
        <UETR>8a562c67-ca16-48ba-b074-65581be6f501</UETR>
      </PmtId>
      <IntrBkSttlmAmt Ccy="EUR">100.00</IntrBkSttlmAmt>
      <ChrgBr>SLEV</ChrgBr>
      <Dbtr><Nm>Alice Corp</Nm></Dbtr>
      <DbtrAcct><Id><IBAN>DE89370400440532013001</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BICFI>DEUTDEFFXXX</BICFI></FinInstnId></DbtrAgt>
      <CdtrAgt><FinInstnId><BICFI>COBADEFFXXX</BICFI></FinInstnId></CdtrAgt>
      <Cdtr><Nm>Bob Ltd</Nm></Cdtr>
      <CdtrAcct><Id><IBAN>DE89370400440532013000</IBAN></Id></CdtrAcct>
      <RmtInf><Ustrd>INV-1</Ustrd></RmtInf>
    </CdtTrfTxInf>
    <CdtTrfTxInf>
      <PmtId><EndToEndId>E2E-2</EndToEndId></PmtId>
      <IntrBkSttlmAmt Ccy="EUR">50.00</IntrBkSttlmAmt>
      <ChrgBr>SHAR</ChrgBr>
      <Dbtr><Nm>Alice Corp</Nm></Dbtr>
      <Cdtr><Nm>Carol GmbH</Nm></Cdtr>
    </CdtTrfTxInf>
      </FIToFICstmrCdtTrf>
</Document>"#;

    const PACS002: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
  <FIToFIPmtStsRpt>
    <GrpHdr>
      <MsgId>STS-001</MsgId>
      <CreDtTm>2026-10-21T09:35:00Z</CreDtTm>
      <InstgAgt><FinInstnId><BICFI>DEUTDEFFXXX</BICFI></FinInstnId></InstgAgt>
      <InstdAgt><FinInstnId><BICFI>COBADEFFXXX</BICFI></FinInstnId></InstdAgt>
    </GrpHdr>
    <OrgnlGrpInfAndSts>
      <OrgnlMsgId>MSG-001</OrgnlMsgId>
      <OrgnlMsgNmId>pacs.008.001.08</OrgnlMsgNmId>
      <OrgnlNbOfTxs>2</OrgnlNbOfTxs>
      <OrgnlCtrlSum>150.00</OrgnlCtrlSum>
      <GrpSts>ACSP</GrpSts>
    </OrgnlGrpInfAndSts>
    <TxInfAndSts>
      <StsId>STS-1</StsId>
      <OrgnlGrpInf>
        <OrgnlMsgId>MSG-001</OrgnlMsgId>
        <OrgnlMsgNmId>pacs.008.001.08</OrgnlMsgNmId>
      </OrgnlGrpInf>
      <OrgnlInstrId>INSTR-1</OrgnlInstrId>
      <OrgnlEndToEndId>E2E-1</OrgnlEndToEndId>
      <OrgnlUETR>8a562c67-ca16-48ba-b074-65581be6f501</OrgnlUETR>
      <TxSts>ACSC</TxSts>
      <AccptncDtTm>2026-10-21T09:35:00Z</AccptncDtTm>
      <AcctSvcrRef>ACCT-REF-1</AcctSvcrRef>
      <ClrSysRef>CLR-1</ClrSysRef>
    </TxInfAndSts>
    <TxInfAndSts>
      <OrgnlEndToEndId>E2E-2</OrgnlEndToEndId>
      <TxSts>RJCT</TxSts>
      <StsRsnInf>
        <Rsn><Cd>AC01</Cd></Rsn>
        <AddtlInf>Account identifier invalid</AddtlInf>
      </StsRsnInf>
    </TxInfAndSts>
  </FIToFIPmtStsRpt>
</Document>"#;

    #[test]
    fn parses_the_status_report_header_and_group() {
        let r = PaymentStatusReport::parse(PACS002).unwrap();
        assert_eq!(r.message_id, "STS-001");
        assert_eq!(r.creation_datetime.as_deref(), Some("2026-10-21T09:35:00Z"));
        assert_eq!(r.instructing_agent_bic.as_deref(), Some("DEUTDEFFXXX"));
        assert_eq!(r.instructed_agent_bic.as_deref(), Some("COBADEFFXXX"));
        let group = &r.original_groups[0];
        assert_eq!(group.original_message_id, "MSG-001");
        assert_eq!(group.original_message_name_id, "pacs.008.001.08");
        assert_eq!(group.original_number_of_transactions.as_deref(), Some("2"));
        assert_eq!(group.original_control_sum, Some(150.0));
        assert_eq!(group.group_status.as_deref(), Some("ACSP"));
    }

    #[test]
    fn reads_a_settled_transaction_in_full() {
        let r = PaymentStatusReport::parse(PACS002).unwrap();
        let first = &r.statuses[0];
        assert_eq!(first.status_id.as_deref(), Some("STS-1"));
        assert_eq!(first.original_instruction_id.as_deref(), Some("INSTR-1"));
        assert_eq!(first.original_end_to_end_id.as_deref(), Some("E2E-1"));
        assert_eq!(first.transaction_status.as_deref(), Some("ACSC"));
        assert_eq!(
            first.account_servicer_reference.as_deref(),
            Some("ACCT-REF-1")
        );
        assert_eq!(first.clearing_system_reference.as_deref(), Some("CLR-1"));
        let group = first.original_group_info.as_ref().unwrap();
        assert_eq!(group.original_message_id, "MSG-001");
    }

    #[test]
    fn reads_a_rejection_reason() {
        let r = PaymentStatusReport::parse(PACS002).unwrap();
        let rejected = &r.statuses[1];
        assert_eq!(rejected.transaction_status.as_deref(), Some("RJCT"));
        let reason = &rejected.status_reasons[0];
        assert_eq!(reason.code.as_deref(), Some("AC01"));
        assert_eq!(reason.proprietary, None);
        assert_eq!(
            reason.additional_information,
            ["Account identifier invalid"]
        );
    }

    #[test]
    fn a_status_report_round_trips_through_xml() {
        let original = PaymentStatusReport::parse(PACS002).unwrap();
        let written = original.to_xml().unwrap();
        let reparsed = PaymentStatusReport::parse(&written).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn a_proprietary_reason_is_kept() {
        let xml = PACS002.replace(
            "<Rsn><Cd>AC01</Cd></Rsn>",
            "<Rsn><Prtry>MY-REASON</Prtry></Rsn>",
        );
        let r = PaymentStatusReport::parse(&xml).unwrap();
        let reason = &r.statuses[1].status_reasons[0];
        assert_eq!(reason.code, None);
        assert_eq!(reason.proprietary.as_deref(), Some("MY-REASON"));
        assert!(r.to_xml().unwrap().contains("<Prtry>MY-REASON</Prtry>"));
    }

    #[test]
    fn an_empty_status_report_cannot_be_written() {
        let r = PaymentStatusReport {
            message_id: "STS-1".into(),
            creation_datetime: None,
            instructing_agent_bic: None,
            instructed_agent_bic: None,
            original_groups: Vec::new(),
            statuses: Vec::new(),
        };
        assert_eq!(r.to_xml(), Err(Error::MissingElement("TxInfAndSts")));
    }

    #[test]
    fn an_empty_group_status_carries_reasons() {
        // A group-level rejection with the reason on the group, not the transaction.
        let xml = PACS002.replace(
            "<GrpSts>ACSP</GrpSts>",
            "<GrpSts>RJCT</GrpSts><StsRsnInf><Rsn><Cd>AM04</Cd></Rsn></StsRsnInf>",
        );
        let r = PaymentStatusReport::parse(&xml).unwrap();
        let group = &r.original_groups[0];
        assert_eq!(group.group_status.as_deref(), Some("RJCT"));
        assert_eq!(group.status_reasons[0].code.as_deref(), Some("AM04"));
    }

    #[test]
    fn the_status_report_namespace_is_pacs_002_001_10() {
        let r = PaymentStatusReport::parse(PACS002).unwrap();
        let xml = r.to_xml().unwrap();
        assert!(xml.contains("urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10"));
        assert!(xml.contains("<OrgnlGrpInfAndSts>"));
    }

    #[test]
    fn a_status_text_is_escaped_on_the_way_out() {
        let mut r = PaymentStatusReport::parse(PACS002).unwrap();
        r.statuses[0].status_reasons.push(StatusReason {
            code: None,
            proprietary: Some("A & B <C>".into()),
            additional_information: Vec::new(),
        });
        let xml = r.to_xml().unwrap();
        assert!(xml.contains("A &amp; B &lt;C&gt;"));
    }

    #[test]
    fn parses_the_group_header() {
        let t = CreditTransfer::parse(PACS008).unwrap();
        assert_eq!(t.message_id, "MSG-001");
        assert_eq!(t.creation_datetime.as_deref(), Some("2026-10-21T09:30:00Z"));
        assert_eq!(t.settlement_method, SettlementMethod::Clearing);
        assert_eq!(t.control_sum, Some(150.0));
        assert_eq!(t.transactions.len(), 2);
    }

    #[test]
    fn reads_the_first_transaction_in_full() {
        let t = CreditTransfer::parse(PACS008).unwrap();
        let first = &t.transactions[0];
        assert_eq!(first.instruction_id.as_deref(), Some("INSTR-1"));
        assert_eq!(first.end_to_end_id, "E2E-1");
        assert_eq!(
            first.uetr.as_deref(),
            Some("8a562c67-ca16-48ba-b074-65581be6f501")
        );
        assert_eq!(first.amount.amount, 100.0);
        assert_eq!(first.amount.currency.as_str(), "EUR");
        assert_eq!(first.charge_bearer, ChargeBearer::FollowingServiceLevel);
        assert_eq!(first.debtor_name, "Alice Corp");
        assert_eq!(first.debtor_iban.as_deref(), Some("DE89370400440532013001"));
        assert_eq!(first.debtor_agent_bic.as_deref(), Some("DEUTDEFFXXX"));
        assert_eq!(first.creditor_name, "Bob Ltd");
        assert_eq!(
            first.creditor_iban.as_deref(),
            Some("DE89370400440532013000")
        );
        assert_eq!(first.creditor_agent_bic.as_deref(), Some("COBADEFFXXX"));
        assert_eq!(first.remittance.as_deref(), Some("INV-1"));
    }

    #[test]
    fn reads_a_minimal_transaction() {
        let t = CreditTransfer::parse(PACS008).unwrap();
        let second = &t.transactions[1];
        assert!(second.instruction_id.is_none());
        assert!(second.uetr.is_none());
        assert!(second.debtor_iban.is_none());
        assert!(second.debtor_agent_bic.is_none());
        assert_eq!(second.charge_bearer, ChargeBearer::Shared);
        assert_eq!(second.creditor_name, "Carol GmbH");
    }

    #[test]
    fn control_sum_agrees_with_the_transactions() {
        let t = CreditTransfer::parse(PACS008).unwrap();
        assert_eq!(t.total(), 150.0);
        assert_eq!(t.number_of_transactions(), 2);
        assert!(t.control_sum_matches());
    }

    #[test]
    fn a_wrong_control_sum_is_reported() {
        let xml = PACS008.replace("<CtrlSum>150.00</CtrlSum>", "<CtrlSum>999.00</CtrlSum>");
        let t = CreditTransfer::parse(&xml).unwrap();
        assert!(!t.control_sum_matches());
    }

    #[test]
    fn a_message_without_a_control_sum_has_nothing_to_check() {
        let xml = PACS008.replace("<CtrlSum>150.00</CtrlSum>", "");
        let t = CreditTransfer::parse(&xml).unwrap();
        assert_eq!(t.control_sum, None);
        assert!(t.control_sum_matches());
    }

    #[test]
    fn round_trips_through_xml() {
        let original = writable();
        let written = original.to_xml().unwrap();
        let reparsed = CreditTransfer::parse(&written).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn generation_computes_the_control_sum() {
        let mut t = writable();
        t.control_sum = Some(999.0);
        let xml = t.to_xml().unwrap();
        assert!(xml.contains("<CtrlSum>100.00</CtrlSum>"));
        assert!(!xml.contains("999"));
    }

    #[test]
    fn generation_places_the_creditor_agent_before_the_creditor() {
        // The schema sequence requires CdtrAgt before Cdtr, and a validating
        // parser rejects the document otherwise.
        let t = writable();
        let xml = t.to_xml().unwrap();
        let agent = xml.find("<CdtrAgt>").unwrap();
        let creditor = xml.find("<Cdtr>").unwrap();
        assert!(agent < creditor, "CdtrAgt must precede Cdtr");
    }

    #[test]
    fn empty_message_cannot_be_written() {
        let mut t = CreditTransfer::parse(PACS008).unwrap();
        t.transactions.clear();
        assert_eq!(t.to_xml(), Err(Error::MissingElement("CdtTrfTxInf")));
    }

    #[test]
    fn a_transaction_without_agent_bics_cannot_be_written() {
        // The schema marks DbtrAgt and CdtrAgt as required, so writing a
        // message that lacks them would produce a document a bank rejects.
        let mut t = CreditTransfer::parse(PACS008).unwrap();
        t.transactions.remove(0);
        assert_eq!(t.to_xml(), Err(Error::MissingElement("DbtrAgt")));

        let mut t = CreditTransfer::parse(PACS008).unwrap();
        t.transactions.remove(0);
        t.transactions[0].debtor_agent_bic = Some("DEUTDEFFXXX".into());
        assert_eq!(t.to_xml(), Err(Error::MissingElement("CdtrAgt")));
    }

    #[test]
    fn a_document_without_a_settlement_method_is_rejected() {
        let xml = r#"<Document><FIToFICstmrCdtTrf>
            <GrpHdr><MsgId>M</MsgId></GrpHdr>
        </FIToFICstmrCdtTrf></Document>"#;
        assert_eq!(
            CreditTransfer::parse(xml),
            Err(Error::MissingElement("SttlmInf"))
        );
    }

    #[test]
    fn an_unknown_settlement_method_is_rejected() {
        let xml = PACS008.replace("<SttlmMtd>CLRG</SttlmMtd>", "<SttlmMtd>XXXX</SttlmMtd>");
        assert_eq!(
            CreditTransfer::parse(&xml),
            Err(Error::InvalidValue {
                element: "SttlmMtd",
                found: "XXXX".into(),
            })
        );
    }

    #[test]
    fn an_unknown_charge_bearer_is_rejected() {
        let xml = PACS008.replace("<ChrgBr>SLEV</ChrgBr>", "<ChrgBr>ZZZZ</ChrgBr>");
        assert_eq!(
            CreditTransfer::parse(&xml),
            Err(Error::InvalidValue {
                element: "ChrgBr",
                found: "ZZZZ".into(),
            })
        );
    }

    #[test]
    fn a_transaction_without_an_end_to_end_id_is_rejected() {
        let xml = r#"<Document><FIToFICstmrCdtTrf>
            <GrpHdr><MsgId>M</MsgId><SttlmInf><SttlmMtd>CLRG</SttlmMtd></SttlmInf></GrpHdr>
            <CdtTrfTxInf>
              <PmtId><InstrId>I</InstrId></PmtId>
              <IntrBkSttlmAmt Ccy="EUR">1.00</IntrBkSttlmAmt>
              <ChrgBr>SLEV</ChrgBr><Dbtr><Nm>A</Nm></Dbtr><Cdtr><Nm>B</Nm></Cdtr>
            </CdtTrfTxInf>
        </FIToFICstmrCdtTrf></Document>"#;
        assert_eq!(
            CreditTransfer::parse(xml),
            Err(Error::MissingElement("EndToEndId"))
        );
    }

    #[test]
    fn settlement_methods_round_trip_through_their_codes() {
        for (code, method) in [
            ("INDA", SettlementMethod::InstructedAgent),
            ("INGA", SettlementMethod::InstructedReimbursementAgent),
            ("COVE", SettlementMethod::Cover),
            ("CLRG", SettlementMethod::Clearing),
        ] {
            assert_eq!(SettlementMethod::parse(code).unwrap(), method);
            assert_eq!(method.code(), code);
        }
    }

    #[test]
    fn charge_bearers_round_trip_through_their_codes() {
        for (code, bearer) in [
            ("DEBT", ChargeBearer::BorneByDebtor),
            ("CRED", ChargeBearer::BorneByCreditor),
            ("SHAR", ChargeBearer::Shared),
            ("SLEV", ChargeBearer::FollowingServiceLevel),
        ] {
            assert_eq!(ChargeBearer::parse(code).unwrap(), bearer);
            assert_eq!(bearer.code(), code);
        }
    }

    #[test]
    fn a_legacy_bic_tag_is_accepted() {
        let xml = PACS008.replace("<BICFI>DEUTDEFFXXX</BICFI>", "<BIC>DEUTDEFFXXX</BIC>");
        let t = CreditTransfer::parse(&xml).unwrap();
        assert_eq!(
            t.transactions[0].debtor_agent_bic.as_deref(),
            Some("DEUTDEFFXXX")
        );
    }

    #[test]
    fn text_is_escaped_on_the_way_out() {
        let mut t = writable();
        t.transactions[0].remittance = Some("A & B <C>".into());
        let xml = t.to_xml().unwrap();
        assert!(xml.contains("A &amp; B &lt;C&gt;"));
    }

    /// The fixture reduced to its first transaction, which has both agent BICs,
    /// so the message can be written under the schema's required-agent rule.
    /// The control sum is set to the one transaction's amount so that a
    /// generated document reparses equal to the original.
    fn writable() -> CreditTransfer {
        let mut t = CreditTransfer::parse(PACS008).unwrap();
        t.transactions.remove(1);
        t.control_sum = Some(100.0);
        t
    }
}
