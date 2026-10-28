//! The identifiers `python-stdnum` covers that the crate reached past without
//! finishing: Iceland, Ecuador, Guatemala, Costa Rica, the Dominican Republic,
//! Guinea, Cuba and Mauritius.
//!
//! Each is a shape and check-digit test. The reference defines no registry
//! lookup for any of them, so a number that passes may still be unissued.
//!
//! Every one of these cleans the raw input itself, because the reference's
//! `compact` removes a different separator set per country. Taking the crate's
//! global normalisation would silently accept input the reference rejects.

use crate::{Error, Result, checksum};

/// Removes the given separators, strips whitespace and upper-cases.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Whether `day`/`month`/`year` is a real calendar date.
///
/// The reference calls `datetime.date(...)`, so the leap-year rule must match
/// Python's rather than accept 31 February.
fn is_real_date(year: i32, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= max_day
}

/// Iceland: kennitala, the personal and organisation identity code.
pub(super) mod iceland {
    use super::*;

    /// Validates an Icelandic kennitala.
    ///
    /// Ten digits: `DDMMYY-RRCC`, where the last digit is a century marker
    /// (`9` for 1900-1999, `0` for 2000 and later) and the digit before it is
    /// the check digit. An organisation's kennitala carries its registration
    /// date with 40 added to the day.
    pub(in crate::id) fn validate_kennitala(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        // The reference's pattern fixes the shape before the date is read: the
        // day tens digit is 0-7, the month tens digit 0-1, and the century
        // marker is exactly 0 or 9. Anything else is a format error, not a
        // bad date, so it is rejected here rather than falling through to the
        // calendar check.
        if digits[0] > 7 || digits[2] > 1 || (digits[9] != 0 && digits[9] != 9) {
            return Err(Error::OutOfRange);
        }
        let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = i32::from(digits[4]) * 10 + i32::from(digits[5]);
        let year = if digits[9] == 9 {
            year + 1900
        } else {
            year + 2000
        };
        let day = if day >= 40 { day - 40 } else { day };
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        let weights = [3, 2, 7, 6, 5, 4, 3, 2, 1, 0];
        let sum: u32 = digits
            .iter()
            .zip(weights)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        if !sum.is_multiple_of(11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Icelandic VSK number, the VAT registration.
    ///
    /// Five or six digits with no check digit; an `IS` prefix is allowed.
    pub(in crate::id) fn validate_vsk(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let number = number.strip_prefix("IS").unwrap_or(&number);
        let digits = checksum::digits_only(number)?;
        if digits.len() != 5 && digits.len() != 6 {
            return Err(Error::WrongLength {
                expected: 5,
                found: digits.len(),
            });
        }
        Ok(())
    }
}

/// Ecuador: CI personal identity code and RUC taxpayer number.
pub(super) mod ecuador {
    use super::*;

    /// The CI check: double every second digit from the left, subtract nine
    /// from anything above nine, and the total must be a multiple of ten.
    fn ci_checksum_ok(digits: &[u8]) -> bool {
        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let product = u32::from(n) * if i % 2 == 0 { 2 } else { 1 };
                if product > 9 { product - 9 } else { product }
            })
            .sum();
        sum.is_multiple_of(10)
    }

    /// The province code the first two digits must carry.
    fn province_ok(digits: &[u8]) -> bool {
        let code = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        (1..=24).contains(&code) || code == 30 || code == 50
    }

    /// Validates an Ecuadorian CI, the personal identity code.
    ///
    /// Ten digits: a province code, then six digits, then a check digit.
    pub(in crate::id) fn validate_ci(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        if !province_ok(&digits) {
            return Err(Error::OutOfRange);
        }
        if digits[2] > 6 {
            return Err(Error::OutOfRange);
        }
        if !ci_checksum_ok(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Ecuadorian RUC, the taxpayer number.
    ///
    /// Thirteen digits: the third digit names the taxpayer kind. Below `6` the
    /// number is a CI plus an establishment suffix; `6` is a public body and
    /// `9` a legal entity, each with its own weighted check.
    pub(in crate::id) fn validate_ruc(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 13 {
            return Err(Error::WrongLength {
                expected: 13,
                found: digits.len(),
            });
        }
        if !province_ok(&digits) {
            return Err(Error::OutOfRange);
        }
        let weighted = |slice: &[u8], weights: &[u32]| -> bool {
            let sum: u32 = slice
                .iter()
                .zip(weights)
                .map(|(&n, &w)| u32::from(n) * w)
                .sum();
            sum.is_multiple_of(11)
        };
        let natural = || -> Result<()> {
            if digits[10..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            // The reference validates the first ten digits as a CI, so an
            // impossible province or check digit rejects the whole number.
            if digits[2] > 6 {
                return Err(Error::OutOfRange);
            }
            if !ci_checksum_ok(&digits[..10]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        let public = || -> Result<()> {
            if digits[9..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            if !weighted(&digits[..9], &[3, 2, 7, 6, 5, 4, 3, 2, 1]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        let juridical = || -> Result<()> {
            if digits[10..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            if !weighted(&digits[..10], &[4, 3, 2, 7, 6, 5, 4, 3, 2, 1]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        match digits[2] {
            0..=5 => natural(),
            6 => public().or_else(|_| natural()),
            9 => public().or_else(|_| juridical()),
            _ => Err(Error::OutOfRange),
        }
    }
}

/// Guatemala: NIT.
pub(super) mod guatemala {
    use super::*;

    /// Validates a Guatemalan NIT.
    ///
    /// Up to twelve characters: all but the last are digits, the last is a
    /// digit or `K`. Leading zeros are dropped, which is what the reference's
    /// `lstrip('0')` does and why a short number is still legal.
    pub(in crate::id) fn validate_nit(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let number = number.trim_start_matches('0');
        if number.len() < 2 || number.len() > 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: number.len(),
            });
        }
        let (body, check) = number.split_at(number.len() - 1);
        if !body.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::OutOfRange);
        }
        let Some(check) = check.chars().next() else {
            return Err(Error::Empty);
        };
        if !check.is_ascii_digit() && check != 'K' {
            return Err(Error::InvalidCharacter(check));
        }
        // Weighted from the right, starting at 2.
        let sum: u32 = body
            .bytes()
            .rev()
            .enumerate()
            .map(|(i, b)| u32::from(b - b'0') * (i as u32 + 2))
            .sum();
        let remainder = (11 - (sum % 11)) % 11;
        let expected = if remainder == 10 {
            'K'
        } else {
            char::from_digit(remainder, 10).unwrap_or('?')
        };
        if check != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Costa Rica: CPJ, the juridical person number.
pub(super) mod costa_rica {
    use super::*;

    /// Validates a Costa Rican CPJ.
    ///
    /// Ten digits with no check digit: the first is the class of juridical
    /// person and the next three a type code that depends on the class.
    pub(in crate::id) fn validate_cpj(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        let class = digits[0];
        let kind = u32::from(digits[1]) * 100 + u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let ok = match class {
            2 => matches!(kind, 100 | 200 | 300 | 400),
            3 => (2..=14).contains(&kind) || (101..=110).contains(&kind),
            4 => kind == 0,
            5 => kind == 1,
            _ => false,
        };
        if !ok {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates a Costa Rican CPF (cédula de persona física), the number of a
    /// natural person.
    ///
    /// Ten digits whose first must be zero; the rest is the `X-XXXX-XXXX`
    /// grouping with the leading zero restored. The reference checks no
    /// checksum.
    pub(in crate::id) fn validate_cpf(s: &str) -> Result<()> {
        let number = compact(s, &[' ']);
        // A written grouping of three parts is padded so that `3-455-175`
        // becomes `0304550175` before the leading zero is restored.
        let parts: Vec<&str> = number.split('-').collect();
        let joined = if parts.len() == 3 {
            let head = format!("{:0>2}", parts[0]);
            let mid = format!("{:0>4}", parts[1]);
            let tail = format!("{:0>4}", parts[2]);
            format!("{head}{mid}{tail}")
        } else {
            number.replace('-', "")
        };
        let padded = if joined.len() == 9 {
            format!("0{joined}")
        } else {
            joined
        };
        let digits = checksum::digits_only(&padded)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        if digits[0] != 0 {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates a Costa Rican CR number (cédula jurídica, the older form).
    ///
    /// Eleven or twelve digits whose first is `1`. The reference checks the
    /// length and that leading class digit only.
    pub(in crate::id) fn validate_cr(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 11 && digits.len() != 12 {
            return Err(Error::WrongLength {
                expected: 11,
                found: digits.len(),
            });
        }
        if digits[0] != 1 {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Dominican Republic: RNC, the taxpayer registry number.
pub(super) mod dominican_republic {
    use super::*;

    /// RNCs the DGII issues that do not satisfy the check digit.
    ///
    /// The reference carries this list, so a number in it is valid regardless
    /// of the arithmetic. Dropping it would reject numbers the reference
    /// accepts.
    const WHITELIST: &[&str] = &[
        "101581601",
        "101582245",
        "101595422",
        "101595785",
        "10233317",
        "131188691",
        "401007374",
        "501341601",
        "501378067",
        "501620371",
        "501651319",
        "501651823",
        "501651845",
        "501651926",
        "501656006",
        "501658167",
        "501670785",
        "501676936",
        "501680158",
        "504654542",
        "504680029",
        "504681442",
        "505038691",
    ];

    /// Cédulas the DGII issues that do not satisfy the Luhn check.
    ///
    /// The reference carries this list, so a number in it is valid regardless
    /// of the arithmetic. Dropping it would reject numbers the reference
    /// accepts.
    const CEDULA_WHITELIST: &[&str] = &[
        "00000021249",
        "00000031417",
        "00000035692",
        "00000045342",
        "00000058035",
        "00000065377",
        "00000078587",
        "00000111941",
        "00000126295",
        "00000129963",
        "00000140874",
        "00000144491",
        "00000155482",
        "00000195576",
        "00000236621",
        "00000292212",
        "00000302347",
        "00000404655",
        "00000547495",
        "00000564933",
        "00000669773",
        "00000719400",
        "00001965804",
        "00004110056",
        "00006747587",
        "00010130085",
        "00010628559",
        "00077584000",
        "00100000169",
        "00100012146",
        "00100013114",
        "00100016495",
        "00100053841",
        "00100061611",
        "00100061945",
        "00100074627",
        "00100083860",
        "00100101767",
        "00100126468",
        "00100145737",
        "00100165504",
        "00100169706",
        "00100172940",
        "00100174666",
        "00100181057",
        "00100228718",
        "00100231017",
        "00100238382",
        "00100239662",
        "00100255349",
        "00100288143",
        "00100288929",
        "00100322649",
        "00100336027",
        "00100350928",
        "00100378440",
        "00100384268",
        "00100384523",
        "00100415853",
        "00100430989",
        "00100523399",
        "00100524531",
        "00100530588",
        "00100531007",
        "00100587320",
        "00100590683",
        "00100593378",
        "00100622461",
        "00100664086",
        "00100709215",
        "00100728113",
        "00100729795",
        "00100756082",
        "00100759932",
        "00101118022",
        "00101166065",
        "00101234090",
        "00101527366",
        "00101541404",
        "00101621981",
        "00101659661",
        "00101684656",
        "00101686299",
        "00101821735",
        "00101961125",
        "00102025201",
        "00102398239",
        "00102577448",
        "00102630192",
        "00103266558",
        "00103436936",
        "00103443802",
        "00103754365",
        "00103766231",
        "00103822440",
        "00103983004",
        "00104486903",
        "00104532086",
        "00104662561",
        "00104727362",
        "00104785104",
        "00104862525",
        "00104966313",
        "00105263314",
        "00105328185",
        "00105512386",
        "00105530894",
        "00105606543",
        "00105832408",
        "00106190966",
        "00106284933",
        "00106418989",
        "00106442522",
        "00106479922",
        "00106916538",
        "00107045499",
        "00107075090",
        "00107184305",
        "00107445493",
        "00107602067",
        "00107665688",
        "00107687383",
        "00107691942",
        "00108113363",
        "00108132448",
        "00108184024",
        "00108264871",
        "00108286792",
        "00108384121",
        "00108413431",
        "00108497822",
        "00108784684",
        "00108796883",
        "00108940225",
        "00109183462",
        "00109229090",
        "00109402756",
        "00109785951",
        "00109987435",
        "00110047715",
        "00110071113",
        "00110111536",
        "00110490843",
        "00110578459",
        "00110646203",
        "00111014782",
        "00111150559",
        "00113453700",
        "00114272360",
        "00114532330",
        "00114532355",
        "00114687216",
        "00115039795",
        "00115343847",
        "00116256005",
        "00116448241",
        "00116508511",
        "00117582001",
        "00119161853",
        "00121344165",
        "00121581750",
        "00121581800",
        "00129737056",
        "00130610001",
        "00131257003",
        "00133987848",
        "00134588056",
        "00142864013",
        "00143072001",
        "00144435001",
        "00146965001",
        "00147485003",
        "00149657590",
        "00155144906",
        "00160405001",
        "00161884001",
        "00162906003",
        "00163540003",
        "00163549012",
        "00163709018",
        "00166457056",
        "00166533003",
        "00167311001",
        "00170009162",
        "00170115579",
        "00171404771",
        "00174729003",
        "00174940001",
        "00181880003",
        "00184129003",
        "00189213001",
        "00189405093",
        "00190002567",
        "00196714003",
        "00200021994",
        "00200028716",
        "00200040516",
        "00200063601",
        "00200123640",
        "00200291381",
        "00200409772",
        "00200435544",
        "00200969260",
        "00201023001",
        "00202110760",
        "00202744522",
        "00207327056",
        "00208430205",
        "00208832003",
        "00218507031",
        "00222017001",
        "00235482001",
        "00236245013",
        "00241997013",
        "00246160013",
        "00261011013",
        "00270764013",
        "00274652001",
        "00278005023",
        "00289931003",
        "00291431001",
        "00291549003",
        "00297018001",
        "00298109001",
        "00299724003",
        "00300001538",
        "00300011700",
        "00300013835",
        "00300015531",
        "00300017875",
        "00300019575",
        "00300020806",
        "00300025568",
        "00300040413",
        "00300052890",
        "00300169535",
        "00300244009",
        "00300636564",
        "00301200901",
        "00305535206",
        "00345425001",
        "00352861001",
        "00356533003",
        "00362684023",
        "00376023023",
        "00388338093",
        "00400001552",
        "00400001614",
        "00400012957",
        "00400189811",
        "00409169001",
        "00425759001",
        "00435518003",
        "00475916056",
        "00481106001",
        "00481595003",
        "00493593003",
        "00500335596",
        "00516077003",
        "00520207699",
        "00524571001",
        "00539342005",
        "00540077717",
        "00544657001",
        "00561269169",
        "00572030001",
        "00574599001",
        "00599408003",
        "00633126023",
        "00644236001",
        "00648496171",
        "00651322001",
        "00686904003",
        "00701067521",
        "00720758056",
        "00731054054",
        "00741721056",
        "00757398001",
        "00800106971",
        "00848583056",
        "00857630012",
        "0094662667",
        "00971815056",
        "01000005580",
        "01000250733",
        "01000268998",
        "01000728704",
        "01000855890",
        "01038813907",
        "01094560111",
        "01100014261",
        "01100620962",
        "01103552230",
        "01133025660",
        "01154421047",
        "01200004166",
        "01200008613",
        "01200011252",
        "01200014133",
        "01200027863",
        "01200033420",
        "01200038298",
        "01200771767",
        "01300001142",
        "01300005424",
        "01300020331",
        "01400000282",
        "01400074875",
        "01600009531",
        "01600019983",
        "01600026316",
        "01600027894",
        "01650257001",
        "01700052445",
        "01700200811",
        "01800022457",
        "01800058439",
        "01800527104",
        "01810035037",
        "02038569001",
        "02100061022",
        "02300003061",
        "02300023225",
        "02300031758",
        "02300037618",
        "02300047220",
        "02300052220",
        "02300054193",
        "02300062066",
        "02300085158",
        "02400229955",
        "02500045676",
        "02600036132",
        "02600094954",
        "02700029905",
        "02755972001",
        "02800000129",
        "02800021761",
        "02800025877",
        "02800029588",
        "02831146001",
        "03000411295",
        "03100001162",
        "03100018730",
        "03100034839",
        "03100083297",
        "03100109611",
        "03100156525",
        "03100195659",
        "03100231390",
        "03100232921",
        "03100277078",
        "03100304632",
        "03100332296",
        "03100398552",
        "03100442457",
        "03100486248",
        "03100488033",
        "03100620176",
        "03100654224",
        "03100668294",
        "03100673050",
        "03100771674",
        "03100789636",
        "03100831768",
        "03100963776",
        "03100984652",
        "03101014877",
        "03101070888",
        "03101105802",
        "03101162278",
        "03101409196",
        "03101456639",
        "03101477254",
        "03101577963",
        "03101713684",
        "03101977306",
        "03102342076",
        "03102399233",
        "03102678700",
        "03102805428",
        "03102828522",
        "03102936385",
        "03103202719",
        "03103315310",
        "03103317617",
        "03103749672",
        "03104354892",
        "03107049671",
        "03108309308",
        "03111670001",
        "03121982479",
        "03131503831",
        "03170483480",
        "03200023002",
        "03200066940",
        "03300023841",
        "03400058730",
        "03400157849",
        "03401709701",
        "03500037890",
        "03600046116",
        "03600127038",
        "03600180637",
        "03700663589",
        "03800032522",
        "03807240010",
        "03852380001",
        "03900069856",
        "03900192284",
        "04022130495",
        "04200012900",
        "04400002002",
        "04400627868",
        "04600198229",
        "04700004024",
        "04700020933",
        "04700027064",
        "04700061076",
        "04700070460",
        "04700074827",
        "04700211635",
        "04700221469",
        "04700728184",
        "04701174268",
        "04800019561",
        "04800034846",
        "04800046910",
        "04800956889",
        "04801245892",
        "04900009932",
        "04900011690",
        "04900013913",
        "04900014592",
        "04900026260",
        "04900028443",
        "04900448230",
        "04902549001",
        "04941042001",
        "05100085656",
        "05300013029",
        "05300013204",
        "05300123494",
        "05400016031",
        "05400021759",
        "05400022042",
        "05400028496",
        "05400033166",
        "05400034790",
        "05400037495",
        "05400038776",
        "05400040523",
        "05400047674",
        "05400048248",
        "05400049237",
        "05400049834",
        "05400050196",
        "05400050304",
        "05400052300",
        "05400053627",
        "05400054156",
        "05400055485",
        "05400055770",
        "05400057300",
        "05400057684",
        "05400058964",
        "05400059956",
        "05400060743",
        "05400062459",
        "05400065376",
        "05400067703",
        "05400072273",
        "05400076481",
        "05400216948",
        "05400878578",
        "05500003079",
        "05500006796",
        "05500008806",
        "05500012039",
        "05500014375",
        "05500017761",
        "05500021118",
        "05500022399",
        "05500023407",
        "05500024135",
        "05500024190",
        "05500027749",
        "05500028350",
        "05500032681",
        "05500173451",
        "05500303477",
        "05600037761",
        "05600038251",
        "05600038964",
        "05600051191",
        "05600063115",
        "05600166034",
        "05600267737",
        "05600553831",
        "05700004693",
        "05700064077",
        "05700071202",
        "05900072869",
        "05900105969",
        "06100007818",
        "06100009131",
        "06100011935",
        "06100013662",
        "06100016486",
        "06100017058",
        "06337850001",
        "06400007916",
        "06400011981",
        "06400014372",
        "06400069279",
        "06486186001",
        "06500162568",
        "06800008448",
        "06800245196",
        "06843739551",
        "06900069184",
        "07000007872",
        "07100018031",
        "07100063262",
        "0710208838",
        "07400001254",
        "07401860112",
        "07600000691",
        "07700009346",
        "07800000968",
        "07800002361",
        "08000213172",
        "08016809001",
        "08100002398",
        "08400068380",
        "08498619001",
        "08800002823",
        "08800003986",
        "08800005068",
        "08900001310",
        "08900004344",
        "08900004849",
        "08900005064",
        "08952698001",
        "09000117963",
        "09000169133",
        "09010011235",
        "09022066011",
        "09200533048",
        "09300006239",
        "09300035357",
        "09400022178",
        "09421581768",
        "09500001177",
        "09500003211",
        "09500008222",
        "09700003030",
        "09700179110",
        "09900017864",
        "10061805811",
        "10100178199",
        "10201116357",
        "10462157001",
        "10491297001",
        "10621581792",
        "10983439110",
        "11700000658",
        "12019831001",
        "12300074628",
        "21000000000",
        "22321581834",
        "22721581818",
        "40200401324",
        "40200452735",
        "40200639953",
        "40200700675",
        "58005174058",
        "90001200901",
    ];

    /// Validates a Dominican RNC.
    ///
    /// Nine digits with a mod-11 check, except for the whitelisted numbers.
    pub(in crate::id) fn validate_rnc(s: &str) -> Result<()> {
        let number = compact(s, &['-', ' ']);
        let digits = checksum::digits_only(&number)?;
        if WHITELIST.contains(&number.as_str()) {
            return Ok(());
        }
        if digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        let weights = [7, 9, 8, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..8]
            .iter()
            .zip(weights)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        let check = (10 - (sum % 11)) % 9 + 1;
        if check != u32::from(digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Dominican cédula, the eleven-digit national identity number.
    ///
    /// Eleven digits with a Luhn check, except for the cédulas the DGII issues
    /// outside that rule, which the reference whitelists.
    pub(in crate::id) fn validate_cedula(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&number)?;
        if CEDULA_WHITELIST.contains(&number.as_str()) {
            return Ok(());
        }
        if digits.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: digits.len(),
            });
        }
        if !checksum::luhn_matches(&digits[..10], digits[10]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The document types a paper or electronic NCF may carry.
    const NCF_DOCUMENT_TYPES: &[&str] = &[
        "01", "02", "03", "04", "11", "12", "13", "14", "15", "16", "17",
    ];

    /// The document types an electronic NCF may carry.
    const ECF_DOCUMENT_TYPES: &[&str] =
        &["31", "32", "33", "34", "41", "43", "44", "45", "46", "47"];

    /// Validates a Dominican NCF, the tax receipt number.
    ///
    /// Three shapes, told apart by length: an eleven-character paper receipt
    /// beginning `B`, a thirteen-character electronic receipt beginning `E`,
    /// and a nineteen-character older form beginning `A` or `P`. The two
    /// digits after the leading letter name the document type.
    pub(in crate::id) fn validate_ncf(s: &str) -> Result<()> {
        let number = compact(s, &[' ']);
        let bytes = number.as_bytes();
        match number.len() {
            11 => {
                if bytes[0] != b'B' || !bytes[1..].iter().all(u8::is_ascii_digit) {
                    return Err(Error::InvalidPrefix);
                }
                if !NCF_DOCUMENT_TYPES.contains(&&number[1..3]) {
                    return Err(Error::OutOfRange);
                }
            }
            13 => {
                if bytes[0] != b'E' || !bytes[1..].iter().all(u8::is_ascii_digit) {
                    return Err(Error::InvalidPrefix);
                }
                if !ECF_DOCUMENT_TYPES.contains(&&number[1..3]) {
                    return Err(Error::OutOfRange);
                }
            }
            19 => {
                if !matches!(bytes[0], b'A' | b'P') || !bytes[1..].iter().all(u8::is_ascii_digit) {
                    return Err(Error::InvalidPrefix);
                }
                if !NCF_DOCUMENT_TYPES.contains(&&number[9..11]) {
                    return Err(Error::OutOfRange);
                }
            }
            other => {
                return Err(Error::WrongLength {
                    expected: 11,
                    found: other,
                });
            }
        }
        Ok(())
    }
}

/// Guinea: NIFp.
pub(super) mod guinea {
    use super::*;

    /// Validates a Guinean NIFp.
    ///
    /// Nine digits with a Luhn check.
    pub(in crate::id) fn validate_nifp(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        if !checksum::luhn_matches(&digits[..8], digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Cuba: NI, the identity number.
pub(super) mod cuba {
    use super::*;

    /// Validates a Cuban NI.
    ///
    /// Eleven digits: a date of birth, then a digit that encodes the century,
    /// then a serial whose last digit's parity is the sex. There is no check
    /// digit; the reference checks the date and nothing else.
    pub(in crate::id) fn validate_ni(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: digits.len(),
            });
        }
        let year = i32::from(digits[0]) * 10 + i32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        // The century digit is 9 for the 1800s, 0-5 for the 1900s, else 2000s.
        let year = match digits[6] {
            9 => year + 1800,
            0..=5 => year + 1900,
            _ => year + 2000,
        };
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Mauritius: NID, the national identifier.
pub(super) mod mauritius {
    use super::*;

    /// The alphabet the check character is drawn from.
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// Validates a Mauritian NID.
    ///
    /// Fourteen characters: a leading letter, twelve more, then a check
    /// character over the first thirteen. The date is the day, month and year
    /// at offsets 1, 3 and 5, and the reference reads its century as 2000.
    pub(in crate::id) fn validate_nid(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        if number.len() != 14 {
            return Err(Error::WrongLength {
                expected: 14,
                found: number.len(),
            });
        }
        let bytes = number.as_bytes();
        if !bytes[0].is_ascii_uppercase() {
            return Err(Error::InvalidCharacter(bytes[0] as char));
        }
        let mut values = [0u32; 14];
        for (i, &b) in bytes.iter().enumerate() {
            let Some(position) = ALPHABET.iter().position(|&a| a == b) else {
                return Err(Error::InvalidCharacter(b as char));
            };
            values[i] = position as u32;
        }
        // The check character is over the first thirteen, weighted from 14 down.
        let sum: u32 = values[..13]
            .iter()
            .enumerate()
            .map(|(i, &v)| (14 - i as u32) * v)
            .sum();
        let expected = ALPHABET[((17 - sum % 17) % 17) as usize];
        if bytes[13] != expected {
            return Err(Error::BadChecksum);
        }
        let day = values[1] * 10 + values[2];
        let month = values[3] * 10 + values[4];
        let year = 2000 + i32::from((values[5] * 10 + values[6]) as u16);
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icelandic_kennitala_examples() {
        // The two values the reference publishes, one of each kind.
        assert!(iceland::validate_kennitala("450401-3150").is_ok());
        assert!(iceland::validate_kennitala("1201743399").is_ok());
        assert_eq!(
            iceland::validate_kennitala("530575-0299"),
            Err(Error::BadChecksum)
        );
        // A day the calendar does not have.
        assert!(iceland::validate_kennitala("320174-3399").is_err());
    }

    #[test]
    fn icelandic_vsk_examples() {
        assert!(iceland::validate_vsk("12345").is_ok());
        assert!(iceland::validate_vsk("IS123456").is_ok());
        assert!(iceland::validate_vsk("1234").is_err());
    }

    #[test]
    fn ecuadorian_examples() {
        assert!(ecuador::validate_ci("1701234567").is_ok());
        assert!(ecuador::validate_ruc("1792060346001").is_ok());
        assert_eq!(
            ecuador::validate_ruc("1763154690001"),
            Err(Error::BadChecksum)
        );
        // A province code outside the country's numbering.
        assert!(ecuador::validate_ci("9912345678").is_err());
    }

    #[test]
    fn guatemalan_nit_examples() {
        assert!(guatemala::validate_nit("12345679").is_ok());
        // A leading zero is dropped before the check, so the two spellings of
        // the same number are both accepted.
        assert!(guatemala::validate_nit("012345679").is_ok());
        assert_eq!(guatemala::validate_nit("12345670"), Err(Error::BadChecksum));
    }

    #[test]
    fn costa_rican_cpj_examples() {
        assert!(costa_rica::validate_cpj("3-101-999999").is_ok());
        assert!(costa_rica::validate_cpj("3-534-123559").is_err());
    }

    #[test]
    fn dominican_rnc_examples() {
        assert!(dominican_republic::validate_rnc("1-01-85004-3").is_ok());
        assert_eq!(
            dominican_republic::validate_rnc("101850042"),
            Err(Error::BadChecksum)
        );
        // The whitelist holds numbers the check digit would reject.
        assert!(dominican_republic::validate_rnc("101581601").is_ok());
    }

    #[test]
    fn guinean_nifp_examples() {
        assert!(guinea::validate_nifp("123456782").is_ok());
        assert_eq!(guinea::validate_nifp("123456789"), Err(Error::BadChecksum));
    }

    #[test]
    fn cuban_ni_examples() {
        assert!(cuba::validate_ni("85010112345").is_ok());
        // Month 13 is impossible.
        assert!(cuba::validate_ni("85130112345").is_err());
    }

    #[test]
    fn mauritian_nid_examples() {
        assert!(mauritius::validate_nid("A1501920000003").is_ok());
        assert!(mauritius::validate_nid("B0203901234568").is_ok());
        assert_eq!(
            mauritius::validate_nid("A1501920000004"),
            Err(Error::BadChecksum)
        );
    }
}
