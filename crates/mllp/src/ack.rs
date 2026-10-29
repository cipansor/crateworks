//! Building an HL7 v2 acknowledgement (`ACK`) for a received message.
//!
//! An interface that receives a message must answer it. HL7 v2 defines the
//! acknowledgement as a message of its own: `MSH`, then `MSA`, whose MSH-10
//! echoes the control id of the message being acknowledged and whose MSA-1 is
//! the code that says what happened to it.
//!
//! Only the fields a receiver must fill are set. An `ACK` is deliberately
//! minimal: it carries the sending and receiving facilities swapped, so the
//! original sender can match it, and the standard's defaults everywhere else.
//!
//! When an application error needs to be explained, the *enhanced* form adds an
//! `ERR` segment and fills MSA-4 with the same error condition code, which is
//! what [`build_enhanced`] writes.

use std::fmt::Write as _;

use crate::message::Separators;

/// The MSA-1 code: what happened to the message being acknowledged.
///
/// The three application codes are the ones a receiver sends. The two
/// commit-level codes (`CA`/`CE`) belong to the two-message commit protocol,
/// which is a separate feature this crate does not build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckCode {
    /// `AA` - the message was accepted.
    Accept,
    /// `AE` - the message was rejected because of an application error.
    Error,
    /// `AR` - the message was rejected because of a version or format problem.
    Reject,
}

impl AckCode {
    /// The two-character code as it appears in MSA-1.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "AA",
            Self::Error => "AE",
            Self::Reject => "AR",
        }
    }
}

impl std::fmt::Display for AckCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The header values an `ACK` copies from the message it answers.
///
/// MSH-10 is the control id of the original message and is what MSA-2 echoes
/// back; MSH-3, MSH-4, MSH-5 and MSH-6 are the sending and receiving
/// applications and facilities, which swap in the reply. MSH-12 is the version,
/// which the reply keeps so both ends agree on the grammar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AckContext {
    /// MSH-10 of the message being acknowledged.
    pub control_id: String,
    /// MSH-3 of the message being acknowledged, the sending application.
    pub sending_application: String,
    /// MSH-4 of the message being acknowledged, the sending facility.
    pub sending_facility: String,
    /// MSH-5 of the message being acknowledged, the receiving application.
    pub receiving_application: String,
    /// MSH-6 of the message being acknowledged, the receiving facility.
    pub receiving_facility: String,
    /// MSH-12 of the message being acknowledged, the version. Empty means 2.5.1.
    pub version: String,
}

impl AckContext {
    /// Reads the fields an `ACK` needs out of the message being answered.
    ///
    /// Every field is optional in the standard, so a message that omits one
    /// yields an empty string rather than an error; the reply is still valid.
    #[must_use]
    pub fn from_message(message: &crate::Message<'_>) -> Self {
        let field = |reference: &str| message.value(reference).unwrap_or_default();
        Self {
            control_id: field("MSH-10"),
            sending_application: field("MSH-3"),
            sending_facility: field("MSH-4"),
            receiving_application: field("MSH-5"),
            receiving_facility: field("MSH-6"),
            version: field("MSH-12"),
        }
    }
}

/// The ERR-4 severity: how serious the error the `ERR` segment describes is.
///
/// The three values are HL7 table 0516. A receiver that reports an `AE` or an
/// `AR` normally sends [`Severity::Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// `W` - a warning; the message was still processed.
    Warning,
    /// `E` - an error; the message was not processed.
    Error,
    /// `F` - a fatal error; the connection is unusable.
    Fatal,
}

impl Severity {
    /// The single-letter code as it appears in ERR-4.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "W",
            Self::Error => "E",
            Self::Fatal => "F",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The error condition an enhanced acknowledgement reports.
///
/// It is written into ERR-3 as a coded-with-exceptions (CWE) value and its code
/// is repeated in MSA-4, which is how the enhanced mode ties the reply's summary
/// to the segment that explains it. The codes are HL7 table 0357; `207`, for
/// example, is "application internal error".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDetail {
    /// ERR-3.1 and MSA-4: the error condition code, HL7 table 0357.
    pub code: String,
    /// ERR-3.2: the human-readable text for the code.
    pub text: String,
    /// ERR-3.3: the coding system the code belongs to, normally `HL70357`.
    pub coding_system: String,
    /// ERR-4: the severity.
    pub severity: Severity,
}

impl ErrorDetail {
    /// An error condition with the standard coding system `HL70357`.
    #[must_use]
    pub fn new(code: impl Into<String>, text: impl Into<String>, severity: Severity) -> Self {
        Self {
            code: code.into(),
            text: text.into(),
            coding_system: "HL70357".into(),
            severity,
        }
    }
}

/// Builds the ER7 text of an `ACK` answering `context`.
///
/// `control_id` is the new message's own MSH-10, which the original sender uses
/// to match the reply; it is not the one being echoed. The two are separate on
/// purpose: MSA-2 must carry the *original* control id, while MSH-10 of the
/// reply is fresh.
///
/// `text` is MSA-3, the human-readable text. It is empty for an `AA`, where
/// there is nothing to say.
#[must_use]
pub fn build(
    context: &AckContext,
    code: AckCode,
    control_id: &str,
    text: &str,
    separators: Separators,
) -> String {
    let mut out = String::with_capacity(128);
    write_header(&mut out, context, control_id, separators);
    write_msa(&mut out, code, context, text, None, separators);
    out
}

/// Builds the ER7 text of an *enhanced* `ACK`, which explains an error.
///
/// The enhanced mode is what a receiver sends when it has to say more than
/// "rejected": it fills MSA-4 with the error condition code and appends an `ERR`
/// segment carrying the same code with its text and severity. The code is
/// written to both places because MSA-4 is what a simple reader looks at and
/// ERR-3 is what carries the detail.
///
/// `code` should be [`AckCode::Error`] or [`AckCode::Reject`]; a caller that
/// passes [`AckCode::Accept`] gets the enhanced structure with an accept code,
/// which is legal but unusual.
#[must_use]
pub fn build_enhanced(
    context: &AckContext,
    code: AckCode,
    control_id: &str,
    text: &str,
    error: &ErrorDetail,
    separators: Separators,
) -> String {
    let mut out = String::with_capacity(192);
    write_header(&mut out, context, control_id, separators);
    write_msa(&mut out, code, context, text, Some(&error.code), separators);
    // ERR-1 (error location) and ERR-2 (error location, alternate) are empty:
    // they name the segment and field the error is in, which the caller knows
    // but this builder is not told. ERR-3 is the code, ERR-4 the severity.
    let _ = write!(
        out,
        "ERR{field}{field}{field}{code}{component}{text}{component}{system}{field}{severity}\r",
        field = separators.field,
        component = separators.component,
        code = error.code,
        text = error.text,
        system = error.coding_system,
        severity = error.severity.as_str(),
    );
    out
}

/// Writes the MSH segment every acknowledgement starts with.
///
/// MSH-1 and MSH-2 are written without a leading separator: the encoding
/// characters follow the field separator directly. Every later field is
/// preceded by one. Building the list explicitly keeps the numbering honest
/// rather than relying on a hand-counted format string.
fn write_header(out: &mut String, context: &AckContext, control_id: &str, separators: Separators) {
    let version = if context.version.is_empty() {
        "2.5.1"
    } else {
        &context.version
    };
    let encoding = format!(
        "{}{}{}{}",
        separators.component, separators.repetition, separators.escape, separators.subcomponent
    );

    out.push_str("MSH");
    out.push(separators.field);
    out.push_str(&encoding);
    // MSH-3..MSH-12. MSH-7 (build time) and MSH-8 (security) stay empty, which
    // is legal and means unknown; a caller that knows the time sets it after.
    let fields = [
        context.receiving_application.as_str(),
        context.receiving_facility.as_str(),
        context.sending_application.as_str(),
        context.sending_facility.as_str(),
        "",
        "",
        "ACK",
        control_id,
        "P",
        version,
    ];
    for value in fields {
        out.push(separators.field);
        out.push_str(value);
    }
    out.push('\r');
}

/// Writes the MSA segment, with MSA-4 present only in the enhanced mode.
fn write_msa(
    out: &mut String,
    code: AckCode,
    context: &AckContext,
    text: &str,
    condition: Option<&str>,
    separators: Separators,
) {
    let field = separators.field;
    let _ = write!(
        out,
        "MSA{field}{code}{field}{original}{field}{text}",
        code = code.as_str(),
        original = context.control_id,
    );
    // A minimal ACK stops at MSA-3. The enhanced one appends MSA-4, the error
    // condition code.
    if let Some(condition) = condition {
        let _ = write!(out, "{field}{condition}");
    }
    out.push('\r');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    const ADT: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|MSG0001|P|2.5.1\r\
                       PID|1||12345^^^MRN||DOE^JOHN\r";

    #[test]
    fn the_ack_swaps_the_facilities_and_echoes_the_control_id() {
        let message = Message::parse(ADT).unwrap();
        let context = AckContext::from_message(&message);
        let ack = build(
            &context,
            AckCode::Accept,
            "ACK0001",
            "",
            message.separators(),
        );

        // MSH-10 is the reply's own control id; MSA-2 echoes the original.
        let reply = Message::parse(&ack).unwrap();
        assert_eq!(reply.value("MSH-10").unwrap(), "ACK0001");
        assert_eq!(reply.value("MSA-2").unwrap(), "MSG0001");
        assert_eq!(reply.value("MSA-1").unwrap(), "AA");
        // The facilities swap: the original receiver is now the sender.
        assert_eq!(reply.value("MSH-3").unwrap(), "RECV");
        assert_eq!(reply.value("MSH-5").unwrap(), "SEND");
    }

    #[test]
    fn every_header_field_lands_in_the_standard_position() {
        let message = Message::parse(ADT).unwrap();
        let context = AckContext::from_message(&message);
        let ack = build(
            &context,
            AckCode::Error,
            "ACK0002",
            "bad patient",
            Separators::default(),
        );

        // The hand-counted field order is the easiest thing to get wrong, so
        // assert the whole header text, not just the fields read back.
        let header = ack.split('\r').next().unwrap();
        assert_eq!(header, "MSH|^~\\&|RECV|FAC|SEND|FAC|||ACK|ACK0002|P|2.5.1");
        assert_eq!(
            Message::parse(&ack).unwrap().value("MSA-3").unwrap(),
            "bad patient"
        );
    }

    #[test]
    fn a_missing_version_defaults_to_2_5_1() {
        let context = AckContext {
            control_id: "X".into(),
            ..AckContext::default()
        };
        let ack = build(&context, AckCode::Reject, "R", "", Separators::default());
        assert_eq!(
            Message::parse(&ack).unwrap().value("MSH-12").unwrap(),
            "2.5.1"
        );
    }

    #[test]
    fn the_reply_uses_the_separators_of_the_original() {
        let message =
            Message::parse("MSH#*~\\&#SEND#FAC#R#F#20260101##ADT*A01#M1#P#2.5.1\rPID#1\r").unwrap();
        let context = AckContext::from_message(&message);
        let ack = build(&context, AckCode::Accept, "A1", "", message.separators());
        assert!(ack.starts_with("MSH#*~\\&#"));
        let reply = Message::parse(&ack).unwrap();
        assert_eq!(reply.separators().field, '#');
        assert_eq!(reply.value("MSA-2").unwrap(), "M1");
    }

    #[test]
    fn the_code_renders_as_its_two_letters() {
        assert_eq!(AckCode::Accept.as_str(), "AA");
        assert_eq!(AckCode::Error.to_string(), "AE");
        assert_eq!(AckCode::Reject.to_string(), "AR");
    }

    #[test]
    fn a_minimal_ack_has_no_msa_4_or_err() {
        let message = Message::parse(ADT).unwrap();
        let context = AckContext::from_message(&message);
        let ack = build(&context, AckCode::Error, "A1", "bad", Separators::default());
        let reply = Message::parse(&ack).unwrap();
        assert!(reply.value("MSA-4").is_none());
        assert!(reply.segment("ERR").is_none());
    }

    #[test]
    fn the_enhanced_ack_carries_msa_4_and_an_err_segment() {
        let message = Message::parse(ADT).unwrap();
        let context = AckContext::from_message(&message);
        let error = ErrorDetail::new("207", "Application internal error", Severity::Error);
        let ack = build_enhanced(
            &context,
            AckCode::Error,
            "ACK0003",
            "bad patient",
            &error,
            Separators::default(),
        );

        // The whole MSA and ERR text, because the field positions are the
        // easiest thing to get wrong.
        let lines: Vec<&str> = ack.split('\r').collect();
        assert_eq!(lines[1], "MSA|AE|MSG0001|bad patient|207");
        assert_eq!(lines[2], "ERR|||207^Application internal error^HL70357|E");

        let reply = Message::parse(&ack).unwrap();
        assert_eq!(reply.value("MSA-4").unwrap(), "207");
        // ERR-3 is a CWE: the code repeats in MSA-4, the text and system follow.
        let err3 = reply.value("ERR-3").unwrap();
        assert_eq!(err3, "207^Application internal error^HL70357");
        assert_eq!(reply.value("ERR-4").unwrap(), "E");
    }

    #[test]
    fn the_enhanced_ack_uses_the_separators_of_the_original() {
        let message =
            Message::parse("MSH#*~\\&#SEND#FAC#R#F#20260101##ADT*A01#M1#P#2.5.1\rPID#1\r").unwrap();
        let context = AckContext::from_message(&message);
        let error = ErrorDetail::new("100", "Segment sequence error", Severity::Error);
        let ack = build_enhanced(
            &context,
            AckCode::Reject,
            "A2",
            "",
            &error,
            message.separators(),
        );
        let lines: Vec<&str> = ack.split('\r').collect();
        assert_eq!(lines[1], "MSA#AR#M1##100");
        assert_eq!(lines[2], "ERR###100*Segment sequence error*HL70357#E");
    }

    #[test]
    fn the_severity_renders_as_its_letter() {
        assert_eq!(Severity::Warning.as_str(), "W");
        assert_eq!(Severity::Error.to_string(), "E");
        assert_eq!(Severity::Fatal.to_string(), "F");
        assert_eq!(
            ErrorDetail::new("207", "x", Severity::Error).coding_system,
            "HL70357"
        );
    }
}
