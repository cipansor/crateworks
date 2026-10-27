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

use std::fmt::Write as _;

use crate::message::Separators;

/// The MSA-1 code: what happened to the message being acknowledged.
///
/// The three application codes are the ones a receiver sends. The two
/// commit-level codes (`CA`/`CE`) exist in the standard but are only meaningful
/// with the enhanced acknowledgement mode, which this crate does not build.
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
    let version = if context.version.is_empty() {
        "2.5.1"
    } else {
        &context.version
    };
    let encoding = format!(
        "{}{}{}{}",
        separators.component, separators.repetition, separators.escape, separators.subcomponent
    );

    let mut out = String::with_capacity(128);
    // MSH-1 and MSH-2 are written without a leading separator: the encoding
    // characters follow the field separator directly. Every later field is
    // preceded by one. Building the list explicitly keeps the numbering honest
    // rather than relying on a hand-counted format string.
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

    let _ = write!(
        out,
        "MSA{field}{code}{field}{original}{field}{text}\r",
        field = separators.field,
        code = code.as_str(),
        original = context.control_id,
    );
    out
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
}
