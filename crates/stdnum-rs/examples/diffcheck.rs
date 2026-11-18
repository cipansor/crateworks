//! Reads "Type<TAB>value" lines and prints the verdict for each, so the
//! crate can be compared against an independent implementation.
use std::io::{self, BufRead};

use stdnum_rs::{Country, IdNumber, NumberType, validate};

fn country(name: &str) -> Option<Country> {
    Some(match name {
        "BrazilCpf" => Country::BrazilCpf,
        "BrazilCnpj" => Country::BrazilCnpj,
        "SpainDni" => Country::SpainDni,
        "GermanyTaxId" => Country::GermanyTaxId,
        "PolandPesel" => Country::PolandPesel,
        "IndonesiaNpwp" => Country::IndonesiaNpwp,
        "IndonesiaNik" => Country::IndonesiaNik,
        "IndiaAadhaar" => Country::IndiaAadhaar,
        "NetherlandsBsn" => Country::NetherlandsBsn,
        "BelgiumNationalNumber" => Country::BelgiumNationalNumber,
        "SwedenPersonnummer" => Country::SwedenPersonnummer,
        "NorwayBirthNumber" => Country::NorwayBirthNumber,
        "NorwayOrgNumber" => Country::NorwayOrgNumber,
        "DenmarkCpr" => Country::DenmarkCpr,
        "FinlandHetu" => Country::FinlandHetu,
        "PortugalNif" => Country::PortugalNif,
        "FranceSiren" => Country::FranceSiren,
        "FranceSiret" => Country::FranceSiret,
        "ItalyCodiceFiscale" => Country::ItalyCodiceFiscale,
        "RomaniaCnp" => Country::RomaniaCnp,
        "TurkeyTckn" => Country::TurkeyTckn,
        "CanadaSin" => Country::CanadaSin,
        "AustraliaAbn" => Country::AustraliaAbn,
        "AustraliaTfn" => Country::AustraliaTfn,
        "NewZealandIrd" => Country::NewZealandIrd,
        "ChinaRic" => Country::ChinaRic,
        "KoreaRrn" => Country::KoreaRrn,
        "MexicoRfc" => Country::MexicoRfc,
        _ => return None,
    })
}

/// The international types, which the fixture names by their Rust type rather
/// than by a `Country` variant. The four the crate's README opens with are
/// listed first so the tool can replay the fixture for them too.
fn international(name: &str) -> Option<NumberType> {
    Some(match name {
        "Ean" => NumberType::Ean,
        "Iban" => NumberType::Iban,
        "Isbn" => NumberType::Isbn,
        "Luhn" => NumberType::Luhn,
        "Damm" => NumberType::Damm,
        "Verhoeff" => NumberType::Verhoeff,
        "Isin" => NumberType::Isin,
        "Cusip" => NumberType::Cusip,
        "Figi" => NumberType::Figi,
        "Lei" => NumberType::Lei,
        "Bic" => NumberType::Bic,
        "Grid" => NumberType::Grid,
        "Issn" => NumberType::Issn,
        "Isrc" => NumberType::Isrc,
        "Imei" => NumberType::Imei,
        "Ismn" => NumberType::Ismn,
        "Isni" => NumberType::Isni,
        "CasNumber" => NumberType::CasNumber,
        "CreditorReference" => NumberType::CreditorReference,
        "Imo" => NumberType::Imo,
        "Meid" => NumberType::Meid,
        "Isan" => NumberType::Isan,
        "BitcoinAddress" => NumberType::BitcoinAddress,
        "Mac" => NumberType::Mac,
        "Isil" => NumberType::Isil,
        "EcNumber" => NumberType::EcNumber,
        "EuropeanBanknote" => NumberType::EuropeanBanknote,
        "SepaCreditorIdentifier" => NumberType::SepaCreditorIdentifier,
        "Eic" => NumberType::Eic,
        _ => return None,
    })
}

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let Some((name, value)) = line.split_once('\t') else {
            continue;
        };
        // A `Country` name is validated through `IdNumber`; anything else is
        // an international type the crate dispatches by name.
        let verdict = if let Some(c) = country(name) {
            IdNumber::parse(c, value).is_ok()
        } else if let Some(kind) = international(name) {
            validate(kind, value).is_ok()
        } else {
            continue;
        };
        println!(
            "{name}\t{value}\t{}",
            if verdict { "accept" } else { "reject" }
        );
    }
}
