//! Parsing of quantity text, such as `"3.5 kg"` or `"60 km/h"`.

use crate::Dimension;
use crate::quantity::Quantity;
use crate::units::Unit;
use crate::units::{UnknownUnit, unit};
use std::error::Error;
use std::fmt;

/// Why a piece of text could not be read as a quantity.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseError {
    /// The text is empty, or only whitespace.
    Empty,
    /// A number could not be read, or is missing where one is required.
    Number(String),
    /// A unit name in the text is unknown.
    Unit(UnknownUnit),
    /// The text ends in the middle of an expression.
    UnexpectedEnd,
    /// A character or token has no meaning here.
    Unexpected(String),
    /// An exponent is not a whole number in the `i8` range.
    Exponent(String),
    /// An affine unit such as `degC` appears inside a product or quotient,
    /// where its offset has no meaning.
    AffineUnitInProduct,
    /// A division by a zero value.
    DivideByZero,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("the text is empty"),
            Self::Number(text) => write!(formatter, "{text:?} is not a number"),
            Self::Unit(error) => write!(formatter, "{error}"),
            Self::UnexpectedEnd => formatter.write_str("the expression ends too early"),
            Self::Unexpected(token) => write!(formatter, "unexpected {token:?}"),
            Self::Exponent(text) => write!(formatter, "bad exponent {text:?}"),
            Self::AffineUnitInProduct => formatter
                .write_str("an affine unit such as degC cannot appear in a product or quotient"),
            Self::DivideByZero => formatter.write_str("division by zero"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unit(error) => Some(error),
            _ => None,
        }
    }
}

/// Parse a quantity written as text.
///
/// The grammar is a number, optional, followed by a unit expression built from
/// names, `*`, `/`, `·`, juxtaposition and `^` or `**` powers. A bare number is
/// a dimensionless quantity, and a bare unit expression is a quantity of `1`.
///
/// ```
/// use runits::parse;
///
/// assert_eq!(parse("12").unwrap().value(), 12.0);
/// assert!((parse("3 kg*m/s^2").unwrap().to("N").unwrap().value() - 3.0).abs() < 1e-12);
/// ```
pub fn parse(text: &str) -> Result<Quantity, ParseError> {
    let mut parser = Parser::new(text)?;
    let value = parser.number()?.unwrap_or(1.0);
    // A bare number is dimensionless, so there is no unit expression to read.
    let unit = if parser.at_end() {
        Unit::from_parts(1.0, Dimension::NONE)
    } else {
        parser.expression()?
    };
    if !parser.at_end() {
        return Err(ParseError::Unexpected(format!(
            "{:?}",
            parser.tokens[parser.position]
        )));
    }
    Ok(Quantity::new(value, unit))
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Star,
    Slash,
    Caret,
    Open,
    Close,
}

impl Parser {
    fn new(text: &str) -> Result<Self, ParseError> {
        let tokens = tokenize(text)?;
        if tokens.is_empty() {
            return Err(ParseError::Empty);
        }
        Ok(Self {
            tokens,
            position: 0,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// The leading number, if the text starts with one.
    fn number(&mut self) -> Result<Option<f64>, ParseError> {
        match self.peek() {
            Some(Token::Number(value)) => {
                let value = *value;
                self.position += 1;
                Ok(Some(value))
            }
            _ => Ok(None),
        }
    }

    /// A product or quotient of unit factors.
    fn expression(&mut self) -> Result<Unit, ParseError> {
        // A leading `/`, as in `1 / mol`, divides into an implicit one.
        let mut left = if matches!(self.peek(), Some(Token::Slash)) {
            Unit::from_parts(1.0, Dimension::NONE)
        } else {
            self.factor()?
        };
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.position += 1;
                    let right = self.factor()?;
                    left = multiply(left, right)?;
                }
                Some(Token::Slash) => {
                    self.position += 1;
                    let right = self.factor()?;
                    left = divide(left, right)?;
                }
                // Juxtaposition, as in `kg m` or `N m`.
                Some(Token::Name(_) | Token::Open) => {
                    let right = self.factor()?;
                    left = multiply(left, right)?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// A single unit, with an optional power.
    fn factor(&mut self) -> Result<Unit, ParseError> {
        let mut base = match self.next() {
            Some(Token::Name(name)) => unit(&name).map_err(ParseError::Unit)?,
            Some(Token::Open) => {
                let inner = self.expression()?;
                match self.next() {
                    Some(Token::Close) => inner,
                    _ => return Err(ParseError::UnexpectedEnd),
                }
            }
            Some(Token::Number(value)) => return Ok(Unit::from_parts(value, Dimension::NONE)),
            Some(token) => return Err(ParseError::Unexpected(format!("{token:?}"))),
            None => return Err(ParseError::UnexpectedEnd),
        };
        if let Some(Token::Caret) = self.peek() {
            self.position += 1;
            let exponent = match self.next() {
                Some(Token::Number(value)) if value.fract() == 0.0 => {
                    if !(f64::from(i8::MIN)..=f64::from(i8::MAX)).contains(&value) {
                        return Err(ParseError::Exponent(value.to_string()));
                    }
                    value as i32
                }
                other => return Err(ParseError::Exponent(format!("{other:?}"))),
            };
            let dimension = base
                .dimension()
                .pow(exponent)
                .ok_or_else(|| ParseError::Exponent(exponent.to_string()))?;
            base = Unit::from_parts(base.factor().powi(exponent), dimension);
        }
        Ok(base)
    }
}

fn multiply(left: Unit, right: Unit) -> Result<Unit, ParseError> {
    if left.is_affine() || right.is_affine() {
        return Err(ParseError::AffineUnitInProduct);
    }
    let dimension = left
        .dimension()
        .times(right.dimension())
        .ok_or(ParseError::UnexpectedEnd)?;
    Ok(Unit::from_parts(left.factor() * right.factor(), dimension))
}

fn divide(left: Unit, right: Unit) -> Result<Unit, ParseError> {
    if right.factor() == 0.0 {
        return Err(ParseError::DivideByZero);
    }
    if left.is_affine() || right.is_affine() {
        return Err(ParseError::AffineUnitInProduct);
    }
    let dimension = left
        .dimension()
        .divided_by(right.dimension())
        .ok_or(ParseError::UnexpectedEnd)?;
    Ok(Unit::from_parts(left.factor() / right.factor(), dimension))
}

/// Split text into numbers, names and the operators between them.
///
/// A name may hold `_`, `%`, `°` and any non-ASCII letter, so `deg_C`, `%` and
/// `°C` all survive as single tokens. No name in the table contains `-`, so a
/// `-` always starts a negative exponent, as in `mol^-1`.
fn tokenize(text: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let characters: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        match character {
            ' ' | '\t' | '\n' | '\r' => index += 1,
            '*' => {
                if characters.get(index + 1) == Some(&'*') {
                    tokens.push(Token::Caret);
                    index += 2;
                } else {
                    tokens.push(Token::Star);
                    index += 1;
                }
            }
            '·' | '×' => {
                tokens.push(Token::Star);
                index += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                index += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                index += 1;
            }
            '(' => {
                tokens.push(Token::Open);
                index += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                index += 1;
            }
            '-' | '+' => {
                // Only meaningful as the sign of an exponent, so `mol^-1` reads
                // as a name, a caret and a number.
                let start = index;
                index += 1;
                index = number_end(&characters, index);
                if index == start + 1 {
                    return Err(ParseError::Unexpected(character.to_string()));
                }
                let text: String = characters[start..index].iter().collect();
                let value = text
                    .parse::<f64>()
                    .map_err(|_| ParseError::Number(text.clone()))?;
                tokens.push(Token::Number(value));
            }
            _ if character.is_ascii_digit() || character == '.' => {
                let start = index;
                index = number_end(&characters, index);
                let text: String = characters[start..index].iter().collect();
                let value = text
                    .parse::<f64>()
                    .map_err(|_| ParseError::Number(text.clone()))?;
                tokens.push(Token::Number(value));
            }
            _ => {
                let start = index;
                while index < characters.len() && is_name_char(characters[index]) {
                    index += 1;
                }
                if start == index {
                    return Err(ParseError::Unexpected(character.to_string()));
                }
                tokens.push(Token::Name(characters[start..index].iter().collect()));
            }
        }
    }
    Ok(tokens)
}

/// The end of a number that starts at `index`: digits, an optional fraction and
/// an optional `e` exponent.
fn number_end(characters: &[char], mut index: usize) -> usize {
    while index < characters.len()
        && (characters[index].is_ascii_digit() || characters[index] == '.')
    {
        index += 1;
    }
    if index < characters.len() && matches!(characters[index], 'e' | 'E') {
        let mut lookahead = index + 1;
        if matches!(characters.get(lookahead), Some('+' | '-')) {
            lookahead += 1;
        }
        if characters.get(lookahead).is_some_and(char::is_ascii_digit) {
            index = lookahead;
            while index < characters.len() && characters[index].is_ascii_digit() {
                index += 1;
            }
        }
    }
    index
}

/// Whether a character can appear inside a unit name.
///
/// A digit is excluded, so `m2` is not a name; use `m^2` or `m**2`. The
/// non-ASCII cases are the symbols the table actually uses.
fn is_name_char(character: char) -> bool {
    character.is_alphabetic()
        || matches!(
            character,
            '_' | '%' | '°' | 'µ' | 'μ' | 'Ω' | '∞' | 'Å' | 'å'
        )
}
