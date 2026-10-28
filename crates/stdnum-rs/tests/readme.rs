//! Runs the README example, so the documentation cannot drift from the API.

use stdnum_rs::{Country, IdNumber, NumberType};

#[test]
fn readme_examples() {
    use stdnum_rs::{Iban, Luhn};

    assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
    assert!(Luhn::parse("4539148803436467").is_ok());
}

/// Every number in `stdnum_vectors.tsv` was labelled by python-stdnum, the
/// reference implementation, in the same mode this crate implements (its strict
/// check-digit mode for the Mexican RFC, which is the only place the two modes
/// differ).
///
/// The file holds numbers copied out of the reference's own documentation and
/// mutations of them: every single-digit substitution of a valid number, the
/// last character dropped and a character appended. A mutation is what a typo
/// produces, and a check-digit routine that accepts one is not doing its job.
///
/// This is the test that found the Luhn check digit missing from the NPWP, the
/// two-shape RFC, the alphanumeric CNPJ and the province field of the NIK.
#[test]
fn every_labelled_number_agrees_with_python_stdnum() {
    let fixture = include_str!("fixtures/stdnum_vectors.tsv");
    let mut agree = 0u32;
    let mut disagreements = Vec::new();
    for line in fixture.lines() {
        let mut fields = line.split('\t');
        let (Some(name), Some(value), Some(expected)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // A name is either a country or an international identifier, and the
        // fixture does not say which, so both are tried.
        let Some(verdict) = known_type(name)
            .map(|kind| stdnum_rs::validate(kind, value).is_ok())
            .or_else(|| country_named(name).map(|c| IdNumber::parse(c, value).is_ok()))
        else {
            // A name the fixture carries but this test cannot dispatch would
            // otherwise be skipped in silence, which is how 334 Aadhaar vectors
            // went unchecked. Every name in the fixture must be reachable.
            unreachable!("the fixture names {name:?}, which this test cannot dispatch");
        };
        if verdict == (expected == "1") {
            agree += 1;
        } else {
            disagreements.push(format!("{name} {value:?}"));
        }
    }
    assert!(agree > 31_000, "only {agree} vectors were compared");
    assert!(
        disagreements.is_empty(),
        "{} numbers disagree with python-stdnum: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}

/// The runtime type for the international identifiers in the fixture, which are
/// not tied to a country.
fn known_type(name: &str) -> Option<NumberType> {
    Some(match name {
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

fn country_named(name: &str) -> Option<Country> {
    Some(match name {
        "IndonesiaNpwp" => Country::IndonesiaNpwp,
        "IndonesiaNik" => Country::IndonesiaNik,
        "BrazilCpf" => Country::BrazilCpf,
        "BrazilCnpj" => Country::BrazilCnpj,
        "IndiaAadhaar" => Country::IndiaAadhaar,
        "SpainDni" => Country::SpainDni,
        "GermanyTaxId" => Country::GermanyTaxId,
        "PolandPesel" => Country::PolandPesel,
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
        "ChileRut" => Country::ChileRut,
        "ColombiaNit" => Country::ColombiaNit,
        "PeruRuc" => Country::PeruRuc,
        "UruguayRut" => Country::UruguayRut,
        "ArgentinaCuit" => Country::ArgentinaCuit,
        "VenezuelaRif" => Country::VenezuelaRif,
        "IrelandVat" => Country::IrelandVat,
        "IrelandPps" => Country::IrelandPps,
        "HungaryAnum" => Country::HungaryAnum,
        "CroatiaOib" => Country::CroatiaOib,
        "BulgariaEgn" => Country::BulgariaEgn,
        "BulgariaPnf" => Country::BulgariaPnf,
        "CzechRc" => Country::CzechRc,
        "SlovakiaRc" => Country::SlovakiaRc,
        "EstoniaIk" => Country::EstoniaIk,
        "LithuaniaAsmens" => Country::LithuaniaAsmens,
        "LuxembourgTva" => Country::LuxembourgTva,
        "SwitzerlandSsn" => Country::SwitzerlandSsn,
        "SwitzerlandUid" => Country::SwitzerlandUid,
        "RussiaInn" => Country::RussiaInn,
        "RussiaOgrn" => Country::RussiaOgrn,
        "UkraineEdrpou" => Country::UkraineEdrpou,
        "IsraelIdnr" => Country::IsraelIdnr,
        "ThailandPin" => Country::ThailandPin,
        "SingaporeUen" => Country::SingaporeUen,
        "MalaysiaNric" => Country::MalaysiaNric,
        "SouthAfricaIdnr" => Country::SouthAfricaIdnr,
        "GreeceAmka" => Country::GreeceAmka,
        "SerbiaPib" => Country::SerbiaPib,
        "MontenegroPib" => Country::MontenegroPib,
        "NorthMacedoniaEdb" => Country::NorthMacedoniaEdb,
        "SloveniaDdv" => Country::SloveniaDdv,
        "RomaniaCf" => Country::RomaniaCf,
        "BelarusUnp" => Country::BelarusUnp,
        "CyprusVat" => Country::CyprusVat,
        "MaltaVat" => Country::MaltaVat,
        "GreeceVat" => Country::GreeceVat,
        "FinlandAlv" => Country::FinlandAlv,
        "SwedenOrgnr" => Country::SwedenOrgnr,
        "SwedenVat" => Country::SwedenVat,
        "CzechDic" => Country::CzechDic,
        "AlgeriaNif" => Country::AlgeriaNif,
        "EgyptTn" => Country::EgyptTn,
        "GhanaTin" => Country::GhanaTin,
        "KenyaPin" => Country::KenyaPin,
        "MoroccoIce" => Country::MoroccoIce,
        "MozambiqueNuit" => Country::MozambiqueNuit,
        "SenegalNinea" => Country::SenegalNinea,
        "TunisiaMf" => Country::TunisiaMf,
        "VietnamMst" => Country::VietnamMst,
        "UnitedStatesSsn" => Country::UnitedStatesSsn,
        "UnitedStatesItin" => Country::UnitedStatesItin,
        "UnitedKingdomNhs" => Country::UnitedKingdomNhs,
        "UnitedKingdomUtr" => Country::UnitedKingdomUtr,
        "UnitedKingdomVat" => Country::UnitedKingdomVat,
        "UnitedKingdomSedol" => Country::UnitedKingdomSedol,
        "JapanCorporateNumber" => Country::JapanCorporateNumber,
        "JapanMyNumber" => Country::JapanMyNumber,
        "PakistanCnic" => Country::PakistanCnic,
        "TaiwanUbn" => Country::TaiwanUbn,
        "LatviaPvn" => Country::LatviaPvn,
        "LiechtensteinPeid" => Country::LiechtensteinPeid,
        "MonacoTva" => Country::MonacoTva,
        "AndorraNrt" => Country::AndorraNrt,
        "AlbaniaNipt" => Country::AlbaniaNipt,
        "AzerbaijanVoen" => Country::AzerbaijanVoen,
        "FaroeIslandsVn" => Country::FaroeIslandsVn,
        "MoldovaIdno" => Country::MoldovaIdno,
        "SanMarinoCoe" => Country::SanMarinoCoe,
        "AustriaUid" => Country::AustriaUid,
        "AustriaTin" => Country::AustriaTin,
        "ParaguayRuc" => Country::ParaguayRuc,
        "ElSalvadorNit" => Country::ElSalvadorNit,
        "IcelandKennitala" => Country::IcelandKennitala,
        "IcelandVsk" => Country::IcelandVsk,
        "EcuadorCi" => Country::EcuadorCi,
        "EcuadorRuc" => Country::EcuadorRuc,
        "GuatemalaNit" => Country::GuatemalaNit,
        "CostaRicaCpj" => Country::CostaRicaCpj,
        "DominicanRepublicRnc" => Country::DominicanRepublicRnc,
        "GuineaNifp" => Country::GuineaNifp,
        "CubaNi" => Country::CubaNi,
        "MauritiusNid" => Country::MauritiusNid,
        _ => return None,
    })
}
