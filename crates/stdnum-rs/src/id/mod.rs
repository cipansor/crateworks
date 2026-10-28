//! National identification numbers.
//!
//! Every country defines its own structure and check, so each is implemented
//! separately and reached through [`IdNumber::parse`]. Coverage starts with the
//! numbers this crate's authors could verify against published rules; adding a
//! country means adding one arm and its tests, which is the intended way to
//! extend the crate.
//!
//! # What is checked
//!
//! Only structure and checksum. A number that passes may still be unissued or
//! belong to someone else. Every algorithm here was cross-checked against the
//! independent `python-stdnum` implementation.

use crate::{Error, Result};

mod africa;
mod americas_ext;
mod asia;
mod asia_ext;
mod balkans;
mod brazil;
mod british;
mod eu;
mod europe;
mod europe_ext;
mod latin_america;
mod more_europe;
mod nordics;
mod north_america;
mod oceania;
mod southeast_asia;
mod vat;

use africa::{algeria, egypt, ghana, kenya, morocco, mozambique, senegal, tunisia, vietnam};
use americas_ext::{
    costa_rica, cuba, dominican_republic, ecuador, guatemala, guinea, iceland, mauritius,
};
use asia::{china, india, korea, mexico, turkey};
use asia_ext::{
    greece, israel, japan, malaysia, pakistan, singapore, south_africa, taiwan, thailand,
};
use balkans::{belarus, montenegro, north_macedonia, romania as romania_cf, serbia, slovenia};
use brazil::{cnpj, cpf};
use british::united_kingdom;
use europe::{
    belgium, france, france_nir, germany, italy, netherlands, poland, portugal, romania, spain,
};
use europe_ext::{
    bulgaria, croatia, czechoslovakia, estonia, hungary, ireland, lithuania, luxembourg, russia,
    switzerland, ukraine,
};
use latin_america::{argentina, chile, colombia, el_salvador, paraguay, peru, uruguay, venezuela};
use more_europe::{
    albania, andorra, austria, azerbaijan, faroe_islands, latvia, liechtenstein, moldova, monaco,
    san_marino,
};
use nordics::{denmark, finland, norway, sweden};
use north_america::united_states;
use oceania::{australia, canada, new_zealand};
use southeast_asia::{indonesia, philippines};
use vat::estonia as estonia_vat;
use vat::{
    cyprus, czechia, finland as finland_vat, greece as greece_vat, malta, slovakia as slovakia_vat,
    sweden as sweden_vat,
};

pub use asia::verhoeff_check_digit;

/// Validates a French TVA number, reached through Monaco, which reuses it.
pub(in crate::id) fn france_vat(s: &str) -> Result<()> {
    france::validate_vat(s)
}

/// Removes whitespace and the given separators from `input`, upper-casing the
/// remainder.
///
/// This mirrors the reference's `clean()` plus its `upper()`, for the
/// identifiers whose separator set differs from the crate-wide
/// [`crate::normalize`], which also strips hyphens, dots and colons.
pub(in crate::id) fn clean(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// A country whose identifiers this crate can check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Country {
    /// Indonesia: NPWP tax number, 15 digits.
    IndonesiaNpwp,
    /// Indonesia: NIK identity number, 16 digits.
    IndonesiaNik,
    /// Philippines: PhilSys number, 12 digits with a Luhn check.
    PhilippinesPhilSys,
    /// Brazil: CPF, 11 digits with two check digits.
    BrazilCpf,
    /// Brazil: CNPJ company number, 14 digits.
    BrazilCnpj,
    /// Spain: DNI, NIE or NIF.
    SpainDni,
    /// Germany: Steuerliche Identifikationsnummer, 11 digits.
    GermanyTaxId,
    /// Poland: PESEL, 11 digits.
    PolandPesel,
    /// India: Aadhaar, 12 digits with a Verhoeff check.
    IndiaAadhaar,
    /// Netherlands: BSN, 9 digits with the eleven-test.
    NetherlandsBsn,
    /// Belgium: National Number, 11 digits with a mod-97 check.
    BelgiumNationalNumber,
    /// Sweden: personnummer, 10 or 12 digits with a Luhn check.
    SwedenPersonnummer,
    /// Norway: fødselsnummer, 11 digits with two mod-11 check digits.
    NorwayBirthNumber,
    /// Norway: organisasjonsnummer, 9 digits with a mod-11 check digit.
    NorwayOrgNumber,
    /// Denmark: CPR, 10 digits with a mod-11 check.
    DenmarkCpr,
    /// Finland: HETU, 11 characters with a mod-31 check character.
    FinlandHetu,
    /// Portugal: NIF, 9 digits with a mod-11 check digit.
    PortugalNif,
    /// France: SIREN, 9 digits with a Luhn check.
    FranceSiren,
    /// France: SIRET, 14 digits with a Luhn check.
    FranceSiret,
    /// Italy: codice fiscale, 16 characters or an 11-digit VAT number.
    ItalyCodiceFiscale,
    /// Romania: CNP, 13 digits with a mod-11 check digit.
    RomaniaCnp,
    /// Turkey: T.C. Kimlik No, 11 digits with two check digits.
    TurkeyTckn,
    /// Canada: SIN, 9 digits with a Luhn check.
    CanadaSin,
    /// Australia: ABN, 11 digits with a mod-89 check.
    AustraliaAbn,
    /// Australia: TFN, 8 or 9 digits with a mod-11 check.
    AustraliaTfn,
    /// New Zealand: IRD number, 8 or 9 digits with a mod-11 check.
    NewZealandIrd,
    /// China: resident identity card, 18 characters.
    ChinaRic,
    /// South Korea: resident registration number, 13 digits.
    KoreaRrn,
    /// Mexico: RFC tax number, 12 or 13 characters.
    MexicoRfc,
    /// Chile: RUT, 8 or 9 digits with a check character.
    ChileRut,
    /// Colombia: NIT, 8 to 16 digits.
    ColombiaNit,
    /// Peru: RUC, 11 digits.
    PeruRuc,
    /// Uruguay: RUT, 12 digits.
    UruguayRut,
    /// Argentina: CUIT, 11 digits.
    ArgentinaCuit,
    /// Venezuela: RIF, a taxpayer letter and 9 digits.
    VenezuelaRif,
    /// Ireland: VAT number.
    IrelandVat,
    /// Ireland: PPS number.
    IrelandPps,
    /// Hungary: adószám tax number, 8 digits.
    HungaryAnum,
    /// Croatia: OIB, 11 digits.
    CroatiaOib,
    /// Bulgaria: EGN personal number, 10 digits.
    BulgariaEgn,
    /// Bulgaria: PNF number for a foreigner, 10 digits.
    BulgariaPnf,
    /// Czech Republic: RČ birth number, 9 or 10 digits.
    CzechRc,
    /// Slovakia: RČ birth number, 9 or 10 digits.
    SlovakiaRc,
    /// Estonia: isikukood, 11 digits.
    EstoniaIk,
    /// Lithuania: asmens kodas, 11 digits.
    LithuaniaAsmens,
    /// Luxembourg: TVA VAT number, 8 digits.
    LuxembourgTva,
    /// Switzerland: AHV/AVS social security number, 13 digits.
    SwitzerlandSsn,
    /// Switzerland: UID business number, 12 characters.
    SwitzerlandUid,
    /// Russia: ИНН, 10 or 12 digits.
    RussiaInn,
    /// Russia: ОГРН, 13 or 15 digits.
    RussiaOgrn,
    /// Ukraine: ЄДРПОУ entity code, 8 digits.
    UkraineEdrpou,
    /// Israel: identity number, up to 9 digits.
    IsraelIdnr,
    /// Thailand: personal identification number, 13 digits.
    ThailandPin,
    /// Singapore: UEN, 9 or 10 characters.
    SingaporeUen,
    /// Malaysia: NRIC identity card number, 12 digits.
    MalaysiaNric,
    /// South Africa: identity number, 13 digits.
    SouthAfricaIdnr,
    /// Greece: ΑΜΚΑ social security number, 11 digits.
    GreeceAmka,
    /// Serbia: PIB tax identification number, 9 digits.
    SerbiaPib,
    /// Montenegro: PIB tax number, 8 digits.
    MontenegroPib,
    /// North Macedonia: ЕДБ tax number, 13 digits.
    NorthMacedoniaEdb,
    /// Slovenia: DDV VAT number, 8 digits.
    SloveniaDdv,
    /// Romania: CF VAT number, 2 to 10 digits, or a 13-digit CNP.
    RomaniaCf,
    /// Belarus: УНП payer account number, 9 characters.
    BelarusUnp,
    /// Cyprus: VAT registration number, 9 characters.
    CyprusVat,
    /// Malta: VAT registration number, 8 digits.
    MaltaVat,
    /// Greece: ΦΠΑ/ΑΦΜ VAT number, 9 digits.
    GreeceVat,
    /// Finland: ALV number, 8 digits.
    FinlandAlv,
    /// Sweden: organisationsnummer, 10 digits.
    SwedenOrgnr,
    /// Sweden: VAT number, 12 digits ending `01`.
    SwedenVat,
    /// Czechia: DIČ VAT number, 8 to 10 digits.
    CzechDic,
    /// Algeria: NIF fiscal identification number, 15 or 20 digits.
    AlgeriaNif,
    /// Egypt: tax registration number, 9 digits.
    EgyptTn,
    /// Ghana: TIN taxpayer identification number, 11 characters.
    GhanaTin,
    /// Kenya: KRA PIN, 11 characters.
    KenyaPin,
    /// Morocco: ICE common enterprise identifier, 15 digits.
    MoroccoIce,
    /// Mozambique: NUIT tax number, 9 digits.
    MozambiqueNuit,
    /// Senegal: NINEA business and association number, 7 or 9 digits.
    SenegalNinea,
    /// Tunisia: MF matricule fiscal, 8 or 13 characters.
    TunisiaMf,
    /// Vietnam: MST tax code, 10 or 13 digits.
    VietnamMst,
    /// United States: Social Security Number, 9 digits.
    UnitedStatesSsn,
    /// United States: Individual Taxpayer Identification Number, 9 digits.
    UnitedStatesItin,
    /// United Kingdom: NHS number, 10 digits.
    UnitedKingdomNhs,
    /// United Kingdom: Unique Taxpayer Reference, 10 digits.
    UnitedKingdomUtr,
    /// United Kingdom: VAT registration number.
    UnitedKingdomVat,
    /// United Kingdom: SEDOL securities identifier, 7 characters.
    UnitedKingdomSedol,
    /// Japan: corporate number, 13 digits.
    JapanCorporateNumber,
    /// Japan: My Number individual number, 12 digits.
    JapanMyNumber,
    /// Pakistan: CNIC national identity card number, 13 digits.
    PakistanCnic,
    /// Taiwan: unified business number, 8 digits.
    TaiwanUbn,
    /// Latvia: PVN VAT number, 11 digits.
    LatviaPvn,
    /// Liechtenstein: PEID business or person identifier.
    LiechtensteinPeid,
    /// Monaco: TVA VAT number, 11 characters.
    MonacoTva,
    /// Andorra: NRT taxpayer registry number, 8 characters.
    AndorraNrt,
    /// Albania: NIPT taxpayer identification number, 10 characters.
    AlbaniaNipt,
    /// Azerbaijan: VÖEN taxpayer identification number, 10 digits.
    AzerbaijanVoen,
    /// Faroe Islands: V-number, 6 digits.
    FaroeIslandsVn,
    /// Moldova: IDNO state identification number, 13 digits.
    MoldovaIdno,
    /// San Marino: COE economic operator code, up to 5 digits.
    SanMarinoCoe,
    /// Austria: UID VAT identification number, 9 characters.
    AustriaUid,
    /// Austria: TIN tax number, 9 digits.
    AustriaTin,
    /// Paraguay: RUC taxpayer registry number, up to 9 digits.
    ParaguayRuc,
    /// El Salvador: NIT tax identification number, 14 digits.
    ElSalvadorNit,
    /// Iceland: kennitala, personal or organisation identity code, 10 digits.
    IcelandKennitala,
    /// Iceland: VSK number, VAT registration, 5 or 6 digits.
    IcelandVsk,
    /// Ecuador: CI personal identity code, 10 digits.
    EcuadorCi,
    /// Ecuador: RUC taxpayer number, 13 digits.
    EcuadorRuc,
    /// Guatemala: NIT tax number, up to 12 characters.
    GuatemalaNit,
    /// Costa Rica: CPJ juridical person number, 10 digits.
    CostaRicaCpj,
    /// Dominican Republic: RNC taxpayer number, 9 digits.
    DominicanRepublicRnc,
    /// Guinea: NIFp permanent fiscal identification number, 9 digits.
    GuineaNifp,
    /// Cuba: NI identity number, 11 digits.
    CubaNi,
    /// Mauritius: NID national identifier, 14 characters.
    MauritiusNid,
    /// Poland: NIP tax identification number, 10 digits.
    PolandNip,
    /// Poland: REGON business register number, 9 or 14 digits.
    PolandRegon,
    /// France: NIF tax number, 13 digits.
    FranceNif,
    /// Norway: bank account number, 11 digits.
    NorwayAccountNumber,
    /// Norway: MVA VAT registration.
    NorwayMva,
    /// Denmark: CVR VAT registration, 8 digits.
    DenmarkCvr,
    /// Canada: Business Number, 9 or 15 characters.
    CanadaBusinessNumber,
    /// Australia: ACN company number, 9 digits.
    AustraliaAcn,
    /// India: PAN income-tax identifier, 10 characters.
    IndiaPan,
    /// India: GSTIN goods-and-services-tax number, 15 characters.
    IndiaGstin,
    /// Mexico: CURP population registry code, 18 characters.
    MexicoCurp,
    /// Estonia: Registrikood organisation registration code, 8 digits.
    EstoniaRegistrikood,
    /// Estonia: KMKR VAT registration, 9 digits.
    EstoniaKmkr,
    /// Lithuania: PVM VAT registration, 9 or 12 digits.
    LithuaniaPvm,
    /// Slovakia: IČ DPH VAT registration, 10 digits.
    SlovakiaDph,
    /// Portugal: Cartão de Cidadão number, 12 characters.
    PortugalCc,
    /// South Africa: tax reference number, 10 digits.
    SouthAfricaTin,
    /// United States: ABA routing transit number, 9 digits.
    UnitedStatesRtn,
    /// Turkey: VKN (Vergi Kimlik Numarası) tax number, 10 digits.
    TurkeyVkn,
    /// India: EPIC (voter ID) number, 10 characters.
    IndiaEpic,
    /// South Korea: business registration number, 10 digits.
    KoreaBrn,
    /// Italy: AIC drug identification code, 9 digits or 6 BASE32 characters.
    ItalyAic,
    /// Netherlands: btw-identificatienummer, 12 characters.
    NetherlandsBtw,
    /// Thailand: MOA memorandum-of-association number, 13 digits.
    ThailandMoa,
    /// Canada: British Columbia Personal Health Number, 10 digits.
    CanadaBcPhn,
    /// France: NIR, the social security number, 15 characters with a modulo-97
    /// check over the first thirteen.
    FranceNir,
    /// Italy: Partita IVA, the VAT number, 11 digits with a Luhn check digit.
    ItalyPiva,
    /// Romania: CUI/CIF, the company identifier, 2 to 10 digits with a weighted
    /// check digit.
    RomaniaCui,
    /// Belgium: OGM-VCS, the structured payment communication, 12 digits with a
    /// modulo-97 check.
    BelgiumOgm,
    /// Finland: Y-tunnus, the business identifier, 8 digits with a weighted
    /// check.
    FinlandYtunnus,
    /// Switzerland: ESR, the payment slip reference number, up to 27 digits with
    /// a recursive modulo-10 check.
    SwitzerlandEsr,
    /// China: USCC, the unified social credit code, 18 characters with a
    /// base-31 check character.
    ChinaUscc,
    /// Slovenia: matična številka, the business register number, 7 digits with a
    /// weighted check digit.
    SloveniaMaticna,
    /// Austria: VNR, the social security number, 10 digits with a weighted
    /// check digit.
    AustriaVnr,
    /// Austria: Firmenbuchnummer, the company register number, digits followed
    /// by a letter.
    AustriaFirmenbuch,
    /// Peru: CUI, the identity number on the DNI, 8 digits with an optional
    /// check character.
    PeruCui,
    /// Argentina: DNI, the national identity number, 7 or 8 digits.
    ArgentinaDni,
    /// India: VID, the Aadhaar virtual identity number, 16 digits with a
    /// Verhoeff check digit.
    IndiaVid,
    /// United Kingdom: UPN, the unique pupil number, 13 characters with a
    /// weighted check digit.
    UnitedKingdomUpn,
    /// Netherlands: onderwijsnummer, the student number, 9 digits with a
    /// modified BSN check.
    NetherlandsOnderwijsnummer,
    /// Spain: NIF, the tax number, in its DNI, NIE, K/L/M or CIF form.
    SpainNif,
    /// Spain: CIF, the number of a legal entity, 9 characters.
    SpainCif,
    /// Spain: CCC, the bank account code, 20 digits with two check digits.
    SpainCcc,
    /// Spain: CUPS, the gas and electricity supply-point code, 20 or 22
    /// characters with two check letters.
    SpainCups,
    /// Spain: CAE, the excise-duty activity and establishment code, 13
    /// characters.
    SpainCae,
    /// Spain: Referencia Catastral, the real-estate property identifier, 20
    /// characters with two check characters.
    SpainReferenciaCatastral,
    /// Germany: WKN, the securities identification code, 6 characters.
    GermanyWkn,
    /// Germany: Leitweg-ID, the electronic-invoice routing reference, with a
    /// MOD 97-10 check.
    GermanyLeitweg,
    /// Netherlands: BRIN, the school identification code, 4 or 6 characters.
    NetherlandsBrin,
    /// Netherlands: identity-card and passport number, 9 characters.
    NetherlandsIdentityCard,
    /// France: RCS, the trade-register entry, a city, a kind letter and a SIREN.
    FranceRcs,
    /// United States: EIN, the employer identification number, 9 digits.
    UnitedStatesEin,
    /// United States: PTIN, the tax-return preparer number, `P` and 8 digits.
    UnitedStatesPtin,
    /// United States: ATIN, the adoption taxpayer identification number, 9
    /// digits.
    UnitedStatesAtin,
    /// Israel: company number, 9 digits beginning with 5 and a Luhn check.
    IsraelCompanyNumber,
    /// Ukraine: RNTRC, the individual tax number, 10 digits with a weighted
    /// check digit.
    UkraineRntrc,
    /// Argentina: CBU, the bank account number, 22 digits with two check
    /// digits.
    ArgentinaCbu,
    /// Costa Rica: CPF, the natural-person identity number, 10 digits beginning
    /// with zero.
    CostaRicaCpf,
    /// Finland: association identifier, 1 to 6 digits.
    FinlandAssociationId,
    /// Finland: Veronumero, the construction worker's tax number, 12 digits.
    FinlandVeronumero,
    /// Slovenia: EMŠO, the unique master citizen number, 13 digits with a
    /// weighted check digit.
    SloveniaEmso,
    /// Romania: ONRC, the trade-register number, in its old or new form.
    RomaniaOnrc,
    /// Spain: NIE, the foreigner identity number, `X`, `Y` or `Z` and eight
    /// characters.
    SpainNie,
    /// Spain: postal code, five digits naming a province.
    SpainPostalCode,
    /// Belgium: BIS number, the national number for a partly known birth date,
    /// with the month offset into 20-32 or 40-52.
    BelgiumBis,
    /// Belgium: electronic identity card number, 12 digits with a mod-97 check.
    BelgiumEid,
    /// Belgium: VAT number, 10 digits with a mod-97 check.
    BelgiumVat,
    /// Germany: VAT number (Umsatzsteuer-Identifikationsnummer), 9 digits with
    /// an ISO 7064 Mod 11, 10 check.
    GermanyVat,
    /// Germany: Steuernummer, the tax number a regional office assigns, 10, 11
    /// or 13 digits.
    GermanySteuernummer,
    /// France: accise number, the excise-duty taxpayer reference, 13 characters.
    FranceAccise,
    /// Netherlands: postcode, four digits and two capitals.
    NetherlandsPostcode,
    /// Sweden: postnummer, the postal code, five digits.
    SwedenPostnummer,
    /// Switzerland: VAT number, a UID followed by a VAT suffix.
    SwitzerlandVat,
    /// Bulgaria: VAT number, 9 or 10 digits.
    BulgariaVat,
    /// Costa Rica: CR number, the older juridical-person identity number, 11 or
    /// 12 digits.
    CostaRicaCr,
    /// Dominican Republic: cédula, the national identity number, 11 digits.
    DominicanRepublicCedula,
    /// Dominican Republic: NCF, the tax receipt number.
    DominicanRepublicNcf,
    /// United States: TIN, the taxpayer number in any of its five forms.
    UnitedStatesTin,
    /// Thailand: TIN, the taxpayer number as a personal or company number.
    ThailandTin,
    /// European Union: excise number, a member state's excise reference.
    EuExcise,
    /// European Union: OSS number, the VAT reference for e-commerce.
    EuOss,
    /// France: n° TVA, the VAT number, a SIREN prefixed by two digits.
    FranceTva,
}

impl Country {
    /// Every identifier this crate knows.
    pub const ALL: &'static [Self] = &[
        Self::IndonesiaNpwp,
        Self::IndonesiaNik,
        Self::PhilippinesPhilSys,
        Self::BrazilCpf,
        Self::BrazilCnpj,
        Self::SpainDni,
        Self::GermanyTaxId,
        Self::PolandPesel,
        Self::IndiaAadhaar,
        Self::NetherlandsBsn,
        Self::BelgiumNationalNumber,
        Self::SwedenPersonnummer,
        Self::NorwayBirthNumber,
        Self::NorwayOrgNumber,
        Self::DenmarkCpr,
        Self::FinlandHetu,
        Self::PortugalNif,
        Self::FranceSiren,
        Self::FranceSiret,
        Self::ItalyCodiceFiscale,
        Self::RomaniaCnp,
        Self::TurkeyTckn,
        Self::CanadaSin,
        Self::AustraliaAbn,
        Self::AustraliaTfn,
        Self::NewZealandIrd,
        Self::ChinaRic,
        Self::KoreaRrn,
        Self::MexicoRfc,
        Self::ChileRut,
        Self::ColombiaNit,
        Self::PeruRuc,
        Self::UruguayRut,
        Self::ArgentinaCuit,
        Self::VenezuelaRif,
        Self::IrelandVat,
        Self::IrelandPps,
        Self::HungaryAnum,
        Self::CroatiaOib,
        Self::BulgariaEgn,
        Self::BulgariaPnf,
        Self::CzechRc,
        Self::SlovakiaRc,
        Self::EstoniaIk,
        Self::LithuaniaAsmens,
        Self::LuxembourgTva,
        Self::SwitzerlandSsn,
        Self::SwitzerlandUid,
        Self::RussiaInn,
        Self::RussiaOgrn,
        Self::UkraineEdrpou,
        Self::IsraelIdnr,
        Self::ThailandPin,
        Self::SingaporeUen,
        Self::MalaysiaNric,
        Self::SouthAfricaIdnr,
        Self::GreeceAmka,
        Self::SerbiaPib,
        Self::MontenegroPib,
        Self::NorthMacedoniaEdb,
        Self::SloveniaDdv,
        Self::RomaniaCf,
        Self::BelarusUnp,
        Self::CyprusVat,
        Self::MaltaVat,
        Self::GreeceVat,
        Self::FinlandAlv,
        Self::SwedenOrgnr,
        Self::SwedenVat,
        Self::CzechDic,
        Self::AlgeriaNif,
        Self::EgyptTn,
        Self::GhanaTin,
        Self::KenyaPin,
        Self::MoroccoIce,
        Self::MozambiqueNuit,
        Self::SenegalNinea,
        Self::TunisiaMf,
        Self::VietnamMst,
        Self::UnitedStatesSsn,
        Self::UnitedStatesItin,
        Self::UnitedKingdomNhs,
        Self::UnitedKingdomUtr,
        Self::UnitedKingdomVat,
        Self::UnitedKingdomSedol,
        Self::JapanCorporateNumber,
        Self::JapanMyNumber,
        Self::PakistanCnic,
        Self::TaiwanUbn,
        Self::LatviaPvn,
        Self::LiechtensteinPeid,
        Self::MonacoTva,
        Self::AndorraNrt,
        Self::AlbaniaNipt,
        Self::AzerbaijanVoen,
        Self::FaroeIslandsVn,
        Self::MoldovaIdno,
        Self::SanMarinoCoe,
        Self::AustriaUid,
        Self::AustriaTin,
        Self::ParaguayRuc,
        Self::ElSalvadorNit,
        Self::IcelandKennitala,
        Self::IcelandVsk,
        Self::EcuadorCi,
        Self::EcuadorRuc,
        Self::GuatemalaNit,
        Self::CostaRicaCpj,
        Self::DominicanRepublicRnc,
        Self::GuineaNifp,
        Self::CubaNi,
        Self::MauritiusNid,
        Self::PolandNip,
        Self::PolandRegon,
        Self::FranceNif,
        Self::NorwayAccountNumber,
        Self::NorwayMva,
        Self::DenmarkCvr,
        Self::CanadaBusinessNumber,
        Self::AustraliaAcn,
        Self::IndiaPan,
        Self::IndiaGstin,
        Self::MexicoCurp,
        Self::EstoniaRegistrikood,
        Self::EstoniaKmkr,
        Self::LithuaniaPvm,
        Self::SlovakiaDph,
        Self::PortugalCc,
        Self::SouthAfricaTin,
        Self::UnitedStatesRtn,
        Self::TurkeyVkn,
        Self::IndiaEpic,
        Self::KoreaBrn,
        Self::ItalyAic,
        Self::NetherlandsBtw,
        Self::ThailandMoa,
        Self::CanadaBcPhn,
        Self::FranceNir,
        Self::ItalyPiva,
        Self::RomaniaCui,
        Self::BelgiumOgm,
        Self::FinlandYtunnus,
        Self::SwitzerlandEsr,
        Self::ChinaUscc,
        Self::SloveniaMaticna,
        Self::AustriaVnr,
        Self::AustriaFirmenbuch,
        Self::PeruCui,
        Self::ArgentinaDni,
        Self::IndiaVid,
        Self::UnitedKingdomUpn,
        Self::NetherlandsOnderwijsnummer,
        Self::SpainNif,
        Self::SpainCif,
        Self::SpainCcc,
        Self::SpainCups,
        Self::SpainCae,
        Self::SpainReferenciaCatastral,
        Self::GermanyWkn,
        Self::GermanyLeitweg,
        Self::NetherlandsBrin,
        Self::NetherlandsIdentityCard,
        Self::FranceRcs,
        Self::UnitedStatesEin,
        Self::UnitedStatesPtin,
        Self::UnitedStatesAtin,
        Self::IsraelCompanyNumber,
        Self::UkraineRntrc,
        Self::ArgentinaCbu,
        Self::CostaRicaCpf,
        Self::FinlandAssociationId,
        Self::FinlandVeronumero,
        Self::SloveniaEmso,
        Self::RomaniaOnrc,
        Self::SpainNie,
        Self::SpainPostalCode,
        Self::BelgiumBis,
        Self::BelgiumEid,
        Self::BelgiumVat,
        Self::GermanyVat,
        Self::GermanySteuernummer,
        Self::FranceAccise,
        Self::NetherlandsPostcode,
        Self::SwedenPostnummer,
        Self::SwitzerlandVat,
        Self::BulgariaVat,
        Self::CostaRicaCr,
        Self::DominicanRepublicCedula,
        Self::DominicanRepublicNcf,
        Self::UnitedStatesTin,
        Self::ThailandTin,
        Self::EuExcise,
        Self::EuOss,
        Self::FranceTva,
    ];

    /// A short English description of the identifier.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::IndonesiaNpwp => "Indonesia NPWP tax number",
            Self::IndonesiaNik => "Indonesia NIK identity number",
            Self::PhilippinesPhilSys => "Philippines PhilSys number",
            Self::BrazilCpf => "Brazil CPF",
            Self::BrazilCnpj => "Brazil CNPJ",
            Self::SpainDni => "Spain DNI/NIE/NIF",
            Self::GermanyTaxId => "Germany tax identification number",
            Self::PolandPesel => "Poland PESEL",
            Self::IndiaAadhaar => "India Aadhaar",
            Self::NetherlandsBsn => "Netherlands BSN",
            Self::BelgiumNationalNumber => "Belgium national number",
            Self::SwedenPersonnummer => "Sweden personnummer",
            Self::NorwayBirthNumber => "Norway fødselsnummer",
            Self::NorwayOrgNumber => "Norway organisasjonsnummer",
            Self::DenmarkCpr => "Denmark CPR",
            Self::FinlandHetu => "Finland HETU",
            Self::PortugalNif => "Portugal NIF",
            Self::FranceSiren => "France SIREN",
            Self::FranceSiret => "France SIRET",
            Self::ItalyCodiceFiscale => "Italy codice fiscale",
            Self::RomaniaCnp => "Romania CNP",
            Self::TurkeyTckn => "Turkey T.C. Kimlik No",
            Self::CanadaSin => "Canada SIN",
            Self::AustraliaAbn => "Australia ABN",
            Self::AustraliaTfn => "Australia TFN",
            Self::NewZealandIrd => "New Zealand IRD number",
            Self::ChinaRic => "China resident identity card",
            Self::KoreaRrn => "South Korea resident registration number",
            Self::MexicoRfc => "Mexico RFC",
            Self::ChileRut => "Chile RUT",
            Self::ColombiaNit => "Colombia NIT",
            Self::PeruRuc => "Peru RUC",
            Self::UruguayRut => "Uruguay RUT",
            Self::ArgentinaCuit => "Argentina CUIT",
            Self::VenezuelaRif => "Venezuela RIF",
            Self::IrelandVat => "Ireland VAT number",
            Self::IrelandPps => "Ireland PPS number",
            Self::HungaryAnum => "Hungary adószám",
            Self::CroatiaOib => "Croatia OIB",
            Self::BulgariaEgn => "Bulgaria EGN",
            Self::BulgariaPnf => "Bulgaria PNF",
            Self::CzechRc => "Czech Republic RČ birth number",
            Self::SlovakiaRc => "Slovakia RČ birth number",
            Self::EstoniaIk => "Estonia isikukood",
            Self::LithuaniaAsmens => "Lithuania asmens kodas",
            Self::LuxembourgTva => "Luxembourg TVA",
            Self::SwitzerlandSsn => "Switzerland AHV/AVS number",
            Self::SwitzerlandUid => "Switzerland UID",
            Self::RussiaInn => "Russia ИНН",
            Self::RussiaOgrn => "Russia ОГРН",
            Self::UkraineEdrpou => "Ukraine ЄДРПОУ",
            Self::IsraelIdnr => "Israel identity number",
            Self::ThailandPin => "Thailand personal identification number",
            Self::SingaporeUen => "Singapore UEN",
            Self::MalaysiaNric => "Malaysia NRIC",
            Self::SouthAfricaIdnr => "South Africa identity number",
            Self::GreeceAmka => "Greece ΑΜΚΑ",
            Self::SerbiaPib => "Serbia PIB tax number",
            Self::MontenegroPib => "Montenegro PIB tax number",
            Self::NorthMacedoniaEdb => "North Macedonia ЕДБ tax number",
            Self::SloveniaDdv => "Slovenia DDV VAT number",
            Self::RomaniaCf => "Romania CF VAT number",
            Self::BelarusUnp => "Belarus УНП payer account number",
            Self::CyprusVat => "Cyprus VAT number",
            Self::MaltaVat => "Malta VAT number",
            Self::GreeceVat => "Greece ΦΠΑ VAT number",
            Self::FinlandAlv => "Finland ALV number",
            Self::SwedenOrgnr => "Sweden organisationsnummer",
            Self::SwedenVat => "Sweden VAT number",
            Self::CzechDic => "Czechia DIČ VAT number",
            Self::AlgeriaNif => "Algeria NIF fiscal identification number",
            Self::EgyptTn => "Egypt tax registration number",
            Self::GhanaTin => "Ghana TIN taxpayer identification number",
            Self::KenyaPin => "Kenya KRA PIN",
            Self::MoroccoIce => "Morocco ICE enterprise identifier",
            Self::MozambiqueNuit => "Mozambique NUIT tax number",
            Self::SenegalNinea => "Senegal NINEA business number",
            Self::TunisiaMf => "Tunisia matricule fiscal",
            Self::VietnamMst => "Vietnam MST tax code",
            Self::UnitedStatesSsn => "United States Social Security Number",
            Self::UnitedStatesItin => "United States Individual Taxpayer Identification Number",
            Self::UnitedKingdomNhs => "United Kingdom NHS number",
            Self::UnitedKingdomUtr => "United Kingdom Unique Taxpayer Reference",
            Self::UnitedKingdomVat => "United Kingdom VAT number",
            Self::UnitedKingdomSedol => "United Kingdom SEDOL",
            Self::JapanCorporateNumber => "Japan corporate number",
            Self::JapanMyNumber => "Japan My Number",
            Self::PakistanCnic => "Pakistan CNIC",
            Self::TaiwanUbn => "Taiwan unified business number",
            Self::LatviaPvn => "Latvia PVN VAT number",
            Self::LiechtensteinPeid => "Liechtenstein PEID",
            Self::MonacoTva => "Monaco TVA VAT number",
            Self::AndorraNrt => "Andorra NRT",
            Self::AlbaniaNipt => "Albania NIPT",
            Self::AzerbaijanVoen => "Azerbaijan VÖEN",
            Self::FaroeIslandsVn => "Faroe Islands V-number",
            Self::MoldovaIdno => "Moldova IDNO",
            Self::SanMarinoCoe => "San Marino COE",
            Self::AustriaUid => "Austria UID VAT number",
            Self::AustriaTin => "Austria TIN tax number",
            Self::ParaguayRuc => "Paraguay RUC",
            Self::ElSalvadorNit => "El Salvador NIT",
            Self::IcelandKennitala => "Iceland kennitala",
            Self::IcelandVsk => "Iceland VSK VAT number",
            Self::EcuadorCi => "Ecuador CI identity number",
            Self::EcuadorRuc => "Ecuador RUC taxpayer number",
            Self::GuatemalaNit => "Guatemala NIT",
            Self::CostaRicaCpj => "Costa Rica CPJ juridical person number",
            Self::DominicanRepublicRnc => "Dominican Republic RNC",
            Self::GuineaNifp => "Guinea NIFp tax number",
            Self::CubaNi => "Cuba NI identity number",
            Self::MauritiusNid => "Mauritius NID national identifier",
            Self::PolandNip => "Poland NIP tax number",
            Self::PolandRegon => "Poland REGON business register number",
            Self::FranceNif => "France NIF tax number",
            Self::NorwayAccountNumber => "Norway bank account number",
            Self::NorwayMva => "Norway MVA VAT number",
            Self::DenmarkCvr => "Denmark CVR VAT number",
            Self::CanadaBusinessNumber => "Canada Business Number",
            Self::AustraliaAcn => "Australia ACN company number",
            Self::IndiaPan => "India PAN income-tax identifier",
            Self::IndiaGstin => "India GSTIN tax number",
            Self::MexicoCurp => "Mexico CURP population registry code",
            Self::EstoniaRegistrikood => "Estonia registrikood organisation code",
            Self::EstoniaKmkr => "Estonia KMKR VAT number",
            Self::LithuaniaPvm => "Lithuania PVM VAT number",
            Self::SlovakiaDph => "Slovakia IČ DPH VAT number",
            Self::PortugalCc => "Portugal Cartão de Cidadão number",
            Self::SouthAfricaTin => "South Africa tax reference number",
            Self::UnitedStatesRtn => "United States ABA routing transit number",
            Self::TurkeyVkn => "Turkey VKN tax number",
            Self::IndiaEpic => "India EPIC voter ID number",
            Self::KoreaBrn => "South Korea business registration number",
            Self::ItalyAic => "Italy AIC drug identification code",
            Self::NetherlandsBtw => "Netherlands btw-identificatienummer",
            Self::ThailandMoa => "Thailand MOA number",
            Self::CanadaBcPhn => "Canada British Columbia Personal Health Number",
            Self::FranceNir => "France NIR social security number",
            Self::ItalyPiva => "Italy Partita IVA VAT number",
            Self::RomaniaCui => "Romania CUI/CIF company identifier",
            Self::BelgiumOgm => "Belgium OGM-VCS structured communication",
            Self::FinlandYtunnus => "Finland Y-tunnus business identifier",
            Self::SwitzerlandEsr => "Switzerland ESR payment reference number",
            Self::ChinaUscc => "China Unified Social Credit Code",
            Self::SloveniaMaticna => "Slovenia matična številka register number",
            Self::AustriaVnr => "Austria VNR social security number",
            Self::AustriaFirmenbuch => "Austria Firmenbuchnummer company number",
            Self::PeruCui => "Peru CUI identity number",
            Self::ArgentinaDni => "Argentina DNI national identity number",
            Self::IndiaVid => "India VID virtual identity number",
            Self::UnitedKingdomUpn => "United Kingdom Unique Pupil Number",
            Self::NetherlandsOnderwijsnummer => "Netherlands onderwijsnummer student number",
            Self::SpainNif => "Spain NIF tax number",
            Self::SpainCif => "Spain CIF legal-entity number",
            Self::SpainCcc => "Spain CCC bank account code",
            Self::SpainCups => "Spain CUPS supply-point code",
            Self::SpainCae => "Spain CAE excise-duty activity code",
            Self::SpainReferenciaCatastral => "Spain Referencia Catastral property identifier",
            Self::GermanyWkn => "Germany WKN securities code",
            Self::GermanyLeitweg => "Germany Leitweg-ID invoice routing reference",
            Self::NetherlandsBrin => "Netherlands BRIN school code",
            Self::NetherlandsIdentityCard => "Netherlands identity-card number",
            Self::FranceRcs => "France RCS trade-register number",
            Self::UnitedStatesEin => "United States Employer Identification Number",
            Self::UnitedStatesPtin => "United States Preparer Tax Identification Number",
            Self::UnitedStatesAtin => "United States Adoption Taxpayer Identification Number",
            Self::IsraelCompanyNumber => "Israel company number",
            Self::UkraineRntrc => "Ukraine RNTRC individual tax number",
            Self::ArgentinaCbu => "Argentina CBU bank account number",
            Self::CostaRicaCpf => "Costa Rica CPF natural-person number",
            Self::FinlandAssociationId => "Finland association identifier",
            Self::FinlandVeronumero => "Finland Veronumero individual tax number",
            Self::SloveniaEmso => "Slovenia EMŠO citizen number",
            Self::RomaniaOnrc => "Romania ONRC trade-register number",
            Self::SpainNie => "Spain NIE foreigner identity number",
            Self::SpainPostalCode => "Spain postal code",
            Self::BelgiumBis => "Belgium BIS number",
            Self::BelgiumEid => "Belgium electronic identity card number",
            Self::BelgiumVat => "Belgium VAT number",
            Self::GermanyVat => "Germany VAT number",
            Self::GermanySteuernummer => "Germany Steuernummer tax number",
            Self::FranceAccise => "France accise number",
            Self::NetherlandsPostcode => "Netherlands postcode",
            Self::SwedenPostnummer => "Sweden postnummer",
            Self::SwitzerlandVat => "Switzerland VAT number",
            Self::BulgariaVat => "Bulgaria VAT number",
            Self::CostaRicaCr => "Costa Rica CR number",
            Self::DominicanRepublicCedula => "Dominican Republic cédula",
            Self::DominicanRepublicNcf => "Dominican Republic NCF",
            Self::UnitedStatesTin => "United States TIN",
            Self::ThailandTin => "Thailand TIN",
            Self::EuExcise => "European Union excise number",
            Self::EuOss => "European Union OSS VAT number",
            Self::FranceTva => "France TVA VAT number",
        }
    }
}

/// A validated national identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdNumber {
    country: Country,
    normalized: String,
}

impl IdNumber {
    /// Validates `input` as an identifier of the given country.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] describing the first problem found.
    pub fn parse(country: Country, input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        // Some identifiers use a hyphen as a significant character: the Finnish
        // HETU century sign and the Swedish personnummer separator. Those get a
        // lighter normalisation that only trims and upper-cases, because the
        // full one would delete the character the check depends on.
        let kept = input
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect::<String>();
        // The African and Vietnamese numbers each define their own separator
        // set, and two of them allow characters the global normalisation
        // removes (a colon would be silently deleted here but rejected by the
        // reference), so they clean the raw input themselves.
        let subject: &str = match country {
            Country::FinlandHetu
            | Country::SwedenPersonnummer
            | Country::ItalyCodiceFiscale
            | Country::MexicoRfc
            | Country::ChinaRic => kept.as_str(),
            Country::AlgeriaNif
            | Country::EgyptTn
            | Country::GhanaTin
            | Country::KenyaPin
            | Country::MoroccoIce
            | Country::MozambiqueNuit
            | Country::SenegalNinea
            | Country::TunisiaMf
            | Country::VietnamMst
            | Country::UnitedStatesSsn
            | Country::UnitedStatesItin
            | Country::UnitedKingdomNhs
            | Country::UnitedKingdomUtr
            | Country::UnitedKingdomVat
            | Country::UnitedKingdomSedol
            | Country::JapanCorporateNumber
            | Country::JapanMyNumber
            | Country::PakistanCnic
            | Country::TaiwanUbn
            | Country::LatviaPvn
            | Country::LiechtensteinPeid
            | Country::MonacoTva
            | Country::AndorraNrt
            | Country::AlbaniaNipt
            | Country::AzerbaijanVoen
            | Country::FaroeIslandsVn
            | Country::MoldovaIdno
            | Country::SanMarinoCoe
            | Country::AustriaUid
            | Country::AustriaTin
            | Country::ParaguayRuc
            | Country::ElSalvadorNit
            | Country::IcelandKennitala
            | Country::IcelandVsk
            | Country::EcuadorCi
            | Country::EcuadorRuc
            | Country::GuatemalaNit
            | Country::CostaRicaCpj
            | Country::DominicanRepublicRnc
            | Country::GuineaNifp
            | Country::CubaNi
            | Country::MauritiusNid
            | Country::PolandNip
            | Country::PolandRegon
            | Country::FranceNif
            | Country::NorwayAccountNumber
            | Country::NorwayMva
            | Country::DenmarkCvr
            | Country::CanadaBusinessNumber
            | Country::AustraliaAcn
            | Country::IndiaPan
            | Country::IndiaGstin
            | Country::MexicoCurp
            | Country::EstoniaRegistrikood
            | Country::EstoniaKmkr
            | Country::LithuaniaPvm
            | Country::SlovakiaDph
            | Country::PortugalCc
            | Country::SouthAfricaTin
            | Country::UnitedStatesRtn
            | Country::TurkeyVkn
            | Country::IndiaEpic
            | Country::KoreaBrn
            | Country::ItalyAic
            | Country::NetherlandsBtw
            | Country::ThailandMoa
            | Country::CanadaBcPhn
            | Country::FranceNir
            | Country::ItalyPiva
            | Country::RomaniaCui
            | Country::BelgiumOgm
            | Country::FinlandYtunnus
            | Country::SwitzerlandEsr
            | Country::ChinaUscc
            | Country::SloveniaMaticna
            | Country::AustriaVnr
            | Country::AustriaFirmenbuch
            | Country::PeruCui
            | Country::ArgentinaDni
            | Country::IndiaVid
            | Country::UnitedKingdomUpn
            | Country::NetherlandsOnderwijsnummer
            | Country::GermanyLeitweg
            | Country::FranceRcs
            | Country::UnitedStatesEin
            | Country::UnitedStatesPtin
            | Country::UnitedStatesAtin
            | Country::RomaniaOnrc
            | Country::CostaRicaCpf
            | Country::SpainNie
            | Country::SpainPostalCode
            | Country::BelgiumBis
            | Country::BelgiumEid
            | Country::BelgiumVat
            | Country::GermanyVat
            | Country::GermanySteuernummer
            | Country::FranceAccise
            | Country::NetherlandsPostcode
            | Country::SwedenPostnummer
            | Country::SwitzerlandVat
            | Country::BulgariaVat
            | Country::CostaRicaCr
            | Country::DominicanRepublicCedula
            | Country::DominicanRepublicNcf
            | Country::UnitedStatesTin
            | Country::ThailandTin
            | Country::EuExcise
            | Country::EuOss
            | Country::FranceTva => input,
            _ => normalized.as_str(),
        };
        match country {
            Country::IndonesiaNpwp => indonesia::validate_npwp(subject)?,
            Country::IndonesiaNik => indonesia::validate_nik(subject)?,
            Country::PhilippinesPhilSys => philippines::validate_philsys(subject)?,
            Country::BrazilCpf => cpf::validate(subject)?,
            Country::BrazilCnpj => cnpj::validate(subject)?,
            Country::SpainDni => spain::validate_dni(subject)?,
            Country::GermanyTaxId => germany::validate_tax_id(subject)?,
            Country::PolandPesel => poland::validate_pesel(subject)?,
            Country::IndiaAadhaar => india::validate_aadhaar(subject)?,
            Country::NetherlandsBsn => netherlands::validate_bsn(subject)?,
            Country::BelgiumNationalNumber => belgium::validate(subject)?,
            Country::SwedenPersonnummer => sweden::validate_personnummer(subject)?,
            Country::NorwayBirthNumber => norway::validate_birth_number(subject)?,
            Country::NorwayOrgNumber => norway::validate_org_number(subject)?,
            Country::DenmarkCpr => denmark::validate_cpr(subject)?,
            Country::FinlandHetu => finland::validate_hetu(subject)?,
            Country::PortugalNif => portugal::validate_nif(subject)?,
            Country::FranceSiren => france::validate_siren(subject)?,
            Country::FranceSiret => france::validate_siret(subject)?,
            Country::ItalyCodiceFiscale => italy::validate_codice_fiscale(subject)?,
            Country::RomaniaCnp => romania::validate_cnp(subject)?,
            Country::TurkeyTckn => turkey::validate_tckn(subject)?,
            Country::CanadaSin => canada::validate_sin(subject)?,
            Country::AustraliaAbn => australia::validate_abn(subject)?,
            Country::AustraliaTfn => australia::validate_tfn(subject)?,
            Country::NewZealandIrd => new_zealand::validate_ird(subject)?,
            Country::ChinaRic => china::validate_ric(subject)?,
            Country::KoreaRrn => korea::validate_rrn(subject)?,
            Country::MexicoRfc => mexico::validate_rfc(subject)?,
            Country::ChileRut => chile::validate_rut(subject)?,
            Country::ColombiaNit => colombia::validate_nit(subject)?,
            Country::PeruRuc => peru::validate_ruc(subject)?,
            Country::UruguayRut => uruguay::validate_rut(subject)?,
            Country::ArgentinaCuit => argentina::validate_cuit(subject)?,
            Country::VenezuelaRif => venezuela::validate_rif(subject)?,
            Country::IrelandVat => ireland::validate_vat(subject)?,
            Country::IrelandPps => ireland::validate_pps(subject)?,
            Country::HungaryAnum => hungary::validate_anum(subject)?,
            Country::CroatiaOib => croatia::validate_oib(subject)?,
            Country::BulgariaEgn => bulgaria::validate_egn(subject)?,
            Country::BulgariaPnf => bulgaria::validate_pnf(subject)?,
            Country::CzechRc => czechoslovakia::validate_rc(subject)?,
            Country::SlovakiaRc => czechoslovakia::validate_rc(subject)?,
            Country::EstoniaIk => estonia::validate_ik(subject)?,
            Country::LithuaniaAsmens => lithuania::validate_asmens(subject)?,
            Country::LuxembourgTva => luxembourg::validate_tva(subject)?,
            Country::SwitzerlandSsn => switzerland::validate_ssn(subject)?,
            Country::SwitzerlandUid => switzerland::validate_uid(subject)?,
            Country::RussiaInn => russia::validate_inn(subject)?,
            Country::RussiaOgrn => russia::validate_ogrn(subject)?,
            Country::UkraineEdrpou => ukraine::validate_edrpou(subject)?,
            Country::IsraelIdnr => israel::validate_idnr(subject)?,
            Country::ThailandPin => thailand::validate_pin(subject)?,
            Country::SingaporeUen => singapore::validate_uen(subject)?,
            Country::MalaysiaNric => malaysia::validate_nric(subject)?,
            Country::SouthAfricaIdnr => south_africa::validate_idnr(subject)?,
            Country::GreeceAmka => greece::validate_amka(subject)?,
            Country::SerbiaPib => serbia::validate_pib(subject)?,
            Country::MontenegroPib => montenegro::validate_pib(subject)?,
            Country::NorthMacedoniaEdb => north_macedonia::validate_edb(subject)?,
            Country::SloveniaDdv => slovenia::validate_ddv(subject)?,
            Country::RomaniaCf => romania_cf::validate_cf(subject)?,
            Country::BelarusUnp => belarus::validate_unp(subject)?,
            Country::CyprusVat => cyprus::validate_vat(subject)?,
            Country::MaltaVat => malta::validate_vat(subject)?,
            Country::GreeceVat => greece_vat::validate_vat(subject)?,
            Country::FinlandAlv => finland_vat::validate_alv(subject)?,
            Country::SwedenOrgnr => sweden_vat::validate_org_number(subject)?,
            Country::SwedenVat => sweden_vat::validate_vat(subject)?,
            Country::CzechDic => czechia::validate_dic(subject)?,
            Country::AlgeriaNif => algeria::validate_nif(subject)?,
            Country::EgyptTn => egypt::validate_tn(subject)?,
            Country::GhanaTin => ghana::validate_tin(subject)?,
            Country::KenyaPin => kenya::validate_pin(subject)?,
            Country::MoroccoIce => morocco::validate_ice(subject)?,
            Country::MozambiqueNuit => mozambique::validate_nuit(subject)?,
            Country::SenegalNinea => senegal::validate_ninea(subject)?,
            Country::TunisiaMf => tunisia::validate_mf(subject)?,
            Country::VietnamMst => vietnam::validate_mst(subject)?,
            Country::UnitedStatesSsn => united_states::validate_ssn(subject)?,
            Country::UnitedStatesItin => united_states::validate_itin(subject)?,
            Country::UnitedKingdomNhs => united_kingdom::validate_nhs(subject)?,
            Country::UnitedKingdomUtr => united_kingdom::validate_utr(subject)?,
            Country::UnitedKingdomVat => united_kingdom::validate_vat(subject)?,
            Country::UnitedKingdomSedol => united_kingdom::validate_sedol(subject)?,
            Country::JapanCorporateNumber => japan::validate_cn(subject)?,
            Country::JapanMyNumber => japan::validate_in(subject)?,
            Country::PakistanCnic => pakistan::validate_cnic(subject)?,
            Country::TaiwanUbn => taiwan::validate_ubn(subject)?,
            Country::LatviaPvn => latvia::validate_pvn(subject)?,
            Country::LiechtensteinPeid => liechtenstein::validate_peid(subject)?,
            Country::MonacoTva => monaco::validate_tva(subject)?,
            Country::AndorraNrt => andorra::validate_nrt(subject)?,
            Country::AlbaniaNipt => albania::validate_nipt(subject)?,
            Country::AzerbaijanVoen => azerbaijan::validate_voen(subject)?,
            Country::FaroeIslandsVn => faroe_islands::validate_vn(subject)?,
            Country::MoldovaIdno => moldova::validate_idno(subject)?,
            Country::SanMarinoCoe => san_marino::validate_coe(subject)?,
            Country::AustriaUid => austria::validate_uid(subject)?,
            Country::AustriaTin => austria::validate_tin(subject)?,
            Country::ParaguayRuc => paraguay::validate_ruc(subject)?,
            Country::ElSalvadorNit => el_salvador::validate_nit(subject)?,
            Country::IcelandKennitala => iceland::validate_kennitala(subject)?,
            Country::IcelandVsk => iceland::validate_vsk(subject)?,
            Country::EcuadorCi => ecuador::validate_ci(subject)?,
            Country::EcuadorRuc => ecuador::validate_ruc(subject)?,
            Country::GuatemalaNit => guatemala::validate_nit(subject)?,
            Country::CostaRicaCpj => costa_rica::validate_cpj(subject)?,
            Country::DominicanRepublicRnc => dominican_republic::validate_rnc(subject)?,
            Country::GuineaNifp => guinea::validate_nifp(subject)?,
            Country::CubaNi => cuba::validate_ni(subject)?,
            Country::MauritiusNid => mauritius::validate_nid(subject)?,
            Country::PolandNip => poland::validate_nip(subject)?,
            Country::PolandRegon => poland::validate_regon(subject)?,
            Country::FranceNif => france::validate_nif(subject)?,
            Country::NorwayAccountNumber => norway::validate_account_number(subject)?,
            Country::NorwayMva => norway::validate_mva(subject)?,
            Country::DenmarkCvr => denmark::validate_cvr(subject)?,
            Country::CanadaBusinessNumber => canada::validate_business_number(subject)?,
            Country::AustraliaAcn => australia::validate_acn(subject)?,
            Country::IndiaPan => india::validate_pan(subject)?,
            Country::IndiaGstin => india::validate_gstin(subject)?,
            Country::MexicoCurp => mexico::validate_curp(subject)?,
            Country::EstoniaRegistrikood => estonia::validate_registrikood(subject)?,
            Country::EstoniaKmkr => estonia_vat::validate_kmkr(subject)?,
            Country::LithuaniaPvm => lithuania::validate_pvm(subject)?,
            Country::SlovakiaDph => slovakia_vat::validate_dph(subject)?,
            Country::PortugalCc => portugal::validate_cc(subject)?,
            Country::SouthAfricaTin => south_africa::validate_tin(subject)?,
            Country::UnitedStatesRtn => united_states::validate_rtn(subject)?,
            Country::TurkeyVkn => turkey::validate_vkn(subject)?,
            Country::IndiaEpic => india::validate_epic(subject)?,
            Country::KoreaBrn => korea::validate_brn(subject)?,
            Country::ItalyAic => italy::validate_aic(subject)?,
            Country::NetherlandsBtw => netherlands::validate_btw(subject)?,
            Country::ThailandMoa => thailand::validate_moa(subject)?,
            Country::CanadaBcPhn => canada::validate_bc_phn(subject)?,
            Country::FranceNir => france_nir::validate_nir(subject)?,
            Country::ItalyPiva => italy::validate_piva(subject)?,
            Country::RomaniaCui => romania_cf::validate_cui(subject)?,
            Country::BelgiumOgm => belgium::validate_ogm(subject)?,
            Country::FinlandYtunnus => finland::validate_ytunnus(subject)?,
            Country::SwitzerlandEsr => switzerland::validate_esr(subject)?,
            Country::ChinaUscc => china::validate_uscc(subject)?,
            Country::SloveniaMaticna => slovenia::validate_maticna(subject)?,
            Country::AustriaVnr => austria::validate_vnr(subject)?,
            Country::AustriaFirmenbuch => austria::validate_businessid(subject)?,
            Country::PeruCui => peru::validate_cui(subject)?,
            Country::ArgentinaDni => argentina::validate_dni(subject)?,
            Country::IndiaVid => india::validate_vid(subject)?,
            Country::UnitedKingdomUpn => united_kingdom::validate_upn(subject)?,
            Country::NetherlandsOnderwijsnummer => netherlands::validate_onderwijsnummer(subject)?,
            Country::SpainNif => spain::validate_nif(subject)?,
            Country::SpainCif => spain::validate_cif(subject)?,
            Country::SpainCcc => spain::validate_ccc(subject)?,
            Country::SpainCups => spain::validate_cups(subject)?,
            Country::SpainCae => spain::validate_cae(subject)?,
            Country::SpainReferenciaCatastral => spain::validate_cadastral(subject)?,
            Country::GermanyWkn => germany::validate_wkn(subject)?,
            Country::GermanyLeitweg => germany::validate_leitweg(subject)?,
            Country::NetherlandsBrin => netherlands::validate_brin(subject)?,
            Country::NetherlandsIdentityCard => netherlands::validate_identity_card(subject)?,
            Country::FranceRcs => france::validate_rcs(subject)?,
            Country::UnitedStatesEin => united_states::validate_ein(subject)?,
            Country::UnitedStatesPtin => united_states::validate_ptin(subject)?,
            Country::UnitedStatesAtin => united_states::validate_atin(subject)?,
            Country::IsraelCompanyNumber => israel::validate_company_number(subject)?,
            Country::UkraineRntrc => ukraine::validate_rntrc(subject)?,
            Country::ArgentinaCbu => argentina::validate_cbu(subject)?,
            Country::CostaRicaCpf => costa_rica::validate_cpf(subject)?,
            Country::FinlandAssociationId => finland::validate_association_id(subject)?,
            Country::FinlandVeronumero => finland::validate_veronumero(subject)?,
            Country::SloveniaEmso => slovenia::validate_emso(subject)?,
            Country::RomaniaOnrc => romania_cf::validate_onrc(subject)?,
            Country::SpainNie => spain::validate_nie(subject)?,
            Country::SpainPostalCode => spain::validate_postal_code(subject)?,
            Country::BelgiumBis => belgium::validate_bis(subject)?,
            Country::BelgiumEid => belgium::validate_eid(subject)?,
            Country::BelgiumVat => belgium::validate_vat(subject)?,
            Country::GermanyVat => germany::validate_vat(subject)?,
            Country::GermanySteuernummer => germany::validate_steuernummer(subject)?,
            Country::FranceAccise => france::validate_accise(subject)?,
            Country::NetherlandsPostcode => netherlands::validate_postcode(subject)?,
            Country::SwedenPostnummer => sweden::validate_postnummer(subject)?,
            Country::SwitzerlandVat => switzerland::validate_vat(subject)?,
            Country::BulgariaVat => bulgaria::validate_vat(subject)?,
            Country::CostaRicaCr => costa_rica::validate_cr(subject)?,
            Country::DominicanRepublicCedula => dominican_republic::validate_cedula(subject)?,
            Country::DominicanRepublicNcf => dominican_republic::validate_ncf(subject)?,
            Country::UnitedStatesTin => united_states::validate_tin(subject)?,
            Country::ThailandTin => thailand::validate_tin(subject)?,
            Country::EuExcise => eu::excise::validate(subject)?,
            Country::EuOss => eu::oss::validate(subject)?,
            Country::FranceTva => france::validate_vat(subject)?,
        }
        Ok(Self {
            country,
            normalized,
        })
    }

    /// The country this identifier belongs to.
    #[must_use]
    pub const fn country(&self) -> Country {
        self.country
    }

    /// The normalised identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_rejected_for_every_country() {
        for &country in Country::ALL {
            assert_eq!(
                IdNumber::parse(country, "   "),
                Err(Error::Empty),
                "{country:?}"
            );
        }
    }

    #[test]
    fn every_country_has_a_description() {
        for &country in Country::ALL {
            assert!(!country.description().is_empty(), "{country:?}");
        }
    }

    #[test]
    fn a_one_character_input_is_never_valid() {
        // Catches a validator that returns early without checking length.
        // The Finnish association identifier is the one exception: the
        // reference itself accepts a one-digit number when the register has
        // issued it, so the invariant does not hold for that scheme.
        for &country in Country::ALL {
            if country == Country::FinlandAssociationId {
                assert!(IdNumber::parse(country, "1").is_ok());
                continue;
            }
            assert!(
                IdNumber::parse(country, "1").is_err(),
                "{country:?} accepted a one-character input"
            );
        }
    }

    #[test]
    fn accessors_and_normalisation() {
        let n = IdNumber::parse(Country::BrazilCpf, "390.533.447-05").unwrap();
        assert_eq!(n.country(), Country::BrazilCpf);
        assert_eq!(n.as_str(), "39053344705");
    }

    /// The identifiers added in 0.4.0, each pinned by a reference value and a
    /// mutation that must be rejected. The fixture gate covers these over
    /// thousands of inputs; this test states the shape the crate promises.
    #[test]
    fn newly_covered_identifiers() {
        let ok = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_ok(),
                "{country:?} rejected {value:?}"
            );
        };
        let no = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_err(),
                "{country:?} accepted {value:?}"
            );
        };
        ok(Country::PolandNip, "PL 8567346215");
        no(Country::PolandNip, "PL 8567346216");
        ok(Country::PolandRegon, "192598184");
        ok(Country::PolandRegon, "12345678512347");
        no(Country::PolandRegon, "192598183");
        ok(Country::FranceNif, "3023217600053");
        no(Country::FranceNif, "3023217600054");
        ok(Country::NorwayAccountNumber, "8601 11 17947");
        ok(Country::NorwayAccountNumber, "0000.4090403");
        no(Country::NorwayAccountNumber, "8601 11 17949");
        ok(Country::NorwayMva, "NO 995 525 828 MVA");
        no(Country::NorwayMva, "NO 995 525 829 MVA");
        ok(Country::DenmarkCvr, "DK 13585628");
        no(Country::DenmarkCvr, "DK 13585627");
        ok(Country::CanadaBusinessNumber, "12302 6635");
        no(Country::CanadaBusinessNumber, "12302 6636");
        ok(Country::AustraliaAcn, "004 085 616");
        no(Country::AustraliaAcn, "004 085 617");
        ok(Country::IndiaPan, "ACUPA7085R");
        no(Country::IndiaPan, "ACUPA7085RR");
        ok(Country::IndiaGstin, "27AAPFU0939F1ZV");
        no(Country::IndiaGstin, "27AAPFU0939F1ZO");
        ok(Country::MexicoCurp, "BOXW310820HNERXN09");
        no(Country::MexicoCurp, "BOXW310820HNERXN08");
        ok(Country::EstoniaRegistrikood, "12345678");
        no(Country::EstoniaRegistrikood, "12345679");
        ok(Country::EstoniaKmkr, "100594102");
        no(Country::EstoniaKmkr, "100594103");
        ok(Country::LithuaniaPvm, "119511515");
        ok(Country::LithuaniaPvm, "LT 100001919017");
        no(Country::LithuaniaPvm, "100001919018");
        ok(Country::SlovakiaDph, "SK 202 274 96 19");
        no(Country::SlovakiaDph, "SK 202 274 96 18");
        ok(Country::PortugalCc, "00000000 0 ZZ4");
        no(Country::PortugalCc, "00000000 0 ZZ3");
        ok(Country::SouthAfricaTin, "0001339050");
        no(Country::SouthAfricaTin, "0001339051");
    }

    /// The identifiers added in 0.5.0, each pinned by a reference value and a
    /// mutation that must be rejected. The fixture gate covers these over
    /// thousands of inputs; this test states the shape the crate promises.
    #[test]
    fn newly_covered_identifiers_0_5() {
        let ok = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_ok(),
                "{country:?} rejected {value:?}"
            );
        };
        let no = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_err(),
                "{country:?} accepted {value:?}"
            );
        };
        ok(Country::UnitedStatesRtn, "111000025");
        no(Country::UnitedStatesRtn, "112000025");
        ok(Country::TurkeyVkn, "4540536920");
        no(Country::TurkeyVkn, "4540536921");
        ok(Country::IndiaEpic, "WKH1186253");
        no(Country::IndiaEpic, "WKH1186263");
        ok(Country::KoreaBrn, "116-82-00276");
        no(Country::KoreaBrn, "1168200000");
        ok(Country::ItalyAic, "000307052");
        ok(Country::ItalyAic, "009CVD");
        no(Country::ItalyAic, "000307053");
        ok(Country::NetherlandsBtw, "004495445B01");
        ok(Country::NetherlandsBtw, "NL002455799B11");
        no(Country::NetherlandsBtw, "123456789B90");
        ok(Country::ThailandMoa, "0994000617721");
        no(Country::ThailandMoa, "0-99-4-000-61772-3");
        ok(Country::CanadaBcPhn, "9698 658 215");
        no(Country::CanadaBcPhn, "9698648215");
    }

    /// The identifiers added in 0.6.0, each pinned by a reference value and a
    /// mutation that must be rejected.
    #[test]
    fn newly_covered_identifiers_0_6() {
        let ok = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_ok(),
                "{country:?} rejected {value:?}"
            );
        };
        let no = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_err(),
                "{country:?} accepted {value:?}"
            );
        };
        ok(Country::FranceNir, "2 95 10 99 126 111 93");
        no(Country::FranceNir, "295109912611190");
        ok(Country::FranceNir, "253072B07300470");
        ok(Country::ItalyPiva, "IT 00743110157");
        no(Country::ItalyPiva, "00743110150");
        ok(Country::RomaniaCui, "RO 185 472 90");
        no(Country::RomaniaCui, "18547291");
        ok(Country::BelgiumOgm, "+++010/8068/17183+++");
        no(Country::BelgiumOgm, "010/8068/17180");
        ok(Country::FinlandYtunnus, "2077474-0");
        no(Country::FinlandYtunnus, "2077474-1");
        ok(Country::SwitzerlandEsr, "21 00000 00003 13947 14300 09017");
        no(Country::SwitzerlandEsr, "210000000003139471430009016");
        ok(Country::ChinaUscc, "91110000600037341L");
        no(Country::ChinaUscc, "A1110000600037341L");
        ok(Country::SloveniaMaticna, "9331310000");
        no(Country::SloveniaMaticna, "9331311");
        ok(Country::AustriaVnr, "1237 010180");
        no(Country::AustriaVnr, "2237 010180");
        ok(Country::AustriaFirmenbuch, "FN 122119m");
        no(Country::AustriaFirmenbuch, "122119M");
        ok(Country::PeruCui, "10117410-2");
        no(Country::PeruCui, "10117410-3");
        ok(Country::ArgentinaDni, "20.123.456");
        no(Country::ArgentinaDni, "2012345A");
        ok(Country::IndiaVid, "2341234123412341");
        no(Country::IndiaVid, "2222222222222222");
        ok(Country::UnitedKingdomUpn, "M20112345678A");
        no(Country::UnitedKingdomUpn, "A20112345678A");
        ok(Country::NetherlandsOnderwijsnummer, "1012.22.331");
        no(Country::NetherlandsOnderwijsnummer, "100252333");
    }

    /// The identifiers added in 0.7.0, each pinned by a reference value and a
    /// mutation that must be rejected. The fixture gate covers these over
    /// thousands of inputs; this test states the shape the crate promises.
    #[test]
    fn newly_covered_identifiers_0_7() {
        let ok = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_ok(),
                "{country:?} rejected {value:?}"
            );
        };
        let no = |country, value: &str| {
            assert!(
                IdNumber::parse(country, value).is_err(),
                "{country:?} accepted {value:?}"
            );
        };
        ok(Country::SpainNie, "X2482300W");
        ok(Country::SpainNie, "Y1234567X");
        no(Country::SpainNie, "X2482300A");
        no(Country::SpainNie, "X2482300");
        ok(Country::SpainPostalCode, "01000");
        no(Country::SpainPostalCode, "0100");
        ok(Country::BelgiumBis, "98 47 28 997 65");
        no(Country::BelgiumBis, "98 47 28 997 64");
        ok(Country::BelgiumEid, "000-0011032-71");
        no(Country::BelgiumEid, "000-0011032-25");
        ok(Country::BelgiumVat, "BE0123456749");
        no(Country::BelgiumVat, "BE0123456748");
        ok(Country::GermanyVat, "DE 136,695 976");
        no(Country::GermanyVat, "DE 136,695 977");
        ok(Country::FranceAccise, "FR023004N9448");
        no(Country::FranceAccise, "FR023004A9448");
        ok(Country::NetherlandsPostcode, "2611 ET");
        no(Country::NetherlandsPostcode, "2611 E");
        ok(Country::SwedenPostnummer, "114 18");
        no(Country::SwedenPostnummer, "1141");
        ok(Country::SwitzerlandVat, "CHE-107.787.577 IVA");
        no(Country::SwitzerlandVat, "CHE-107.787.578 IVA");
        ok(Country::BulgariaVat, "BG 175 074 752");
        no(Country::BulgariaVat, "BG 175 074 753");
        ok(Country::CostaRicaCr, "155812994816");
        no(Country::CostaRicaCr, "255812994816");
        ok(Country::DominicanRepublicCedula, "00113918205");
        no(Country::DominicanRepublicCedula, "00113918204");
        ok(Country::DominicanRepublicCedula, "00000021249");
        ok(Country::DominicanRepublicNcf, "B0100000001");
        no(Country::DominicanRepublicNcf, "B0000000001");
        ok(Country::UnitedStatesTin, "042103594");
        // A taxpayer number has no check digit, so the rejection is structural:
        // the nine-digit shape is required.
        no(Country::UnitedStatesTin, "04210359");
        no(Country::UnitedStatesTin, "04210359A");
        ok(Country::ThailandTin, "1-2345-45678-78-1");
        no(Country::ThailandTin, "1-2345-45678-78-2");
        ok(Country::FranceTva, "FR 40 303 265 045");
        no(Country::FranceTva, "84 323 140 391");
        ok(Country::EuExcise, "LU 987ABC");
        no(Country::EuExcise, "XX00000987ABC");
        ok(Country::EuOss, "EU 372022452");
        no(Country::EuOss, "EU999022452");
    }
}
