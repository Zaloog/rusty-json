use std::fmt;
use std::fs::File;
use std::io::Read;
use std::iter::Peekable;
use std::path::Path;
use std::str::Chars;

use anyhow::{Result, bail};

#[derive(Debug)]
pub enum Token {
    BraceCurlyOpen,
    BraceCurlyClose,
    BracketSquareOpen,
    BracketSquareClose,
    Comma,
    Colon,
    String(String),
    Number(f64),
    True,
    False,
    Null,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum JsonValue {
    String(String),
    Number(f64),
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    Boolean(bool),
    Null,
}

const MAX_DEPTH: usize = 19;

#[derive(Debug)]
struct Position {
    row: i32,
    column: i32,
}
impl Position {
    pub fn new() -> Self {
        Position { row: 1, column: 1 }
    }

    pub fn next_row(&mut self) -> &Self {
        self.row += 1;
        self.reset_column()
    }
    pub fn next_column(&mut self) -> &Self {
        self.column += 1;
        self
    }
    pub fn reset_column(&mut self) -> &Self {
        self.column = 1;
        self
    }
}
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.row, self.column)
    }
}

pub fn read_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content.trim().to_string())
}

fn parse_string(chars: &mut Peekable<Chars>, position: &mut Position) -> Result<String> {
    let mut result_string = String::new();
    while let Some(char) = chars.next() {
        match char {
            '"' => {
                position.next_column();
                return Ok(result_string);
            }
            '\\' => {
                // Handle escape sequences
                position.next_column();
                if let Some(escaped_char) = chars.next() {
                    match escaped_char {
                        '"' => result_string.push('"'),
                        '\\' => result_string.push('\\'),
                        '/' => result_string.push('/'),
                        'b' => result_string.push('\u{0008}'), // backspace
                        'f' => result_string.push('\u{000C}'), // form feed
                        'n' => result_string.push('\n'),
                        'r' => result_string.push('\r'),
                        't' => result_string.push('\t'),
                        'u' => {
                            // Unicode escape sequence \uXXXX
                            let mut hex = String::new();
                            for _ in 0..4 {
                                if let Some(hex_char) = chars.next() {
                                    if hex_char.is_ascii_hexdigit() {
                                        hex.push(hex_char);
                                        position.next_column();
                                    } else {
                                        bail!("Invalid unicode escape sequence at position {}", position);
                                    }
                                } else {
                                    bail!("Incomplete unicode escape sequence at position {}", position);
                                }
                            }
                            let code_point = u32::from_str_radix(&hex, 16)
                                .map_err(|_| anyhow::anyhow!("Invalid unicode escape at position {}", position))?;
                            if let Some(unicode_char) = char::from_u32(code_point) {
                                result_string.push(unicode_char);
                            } else {
                                bail!("Invalid unicode code point at position {}", position);
                            }
                        }
                        _ => bail!("Invalid escape sequence '\\{}' at position {}", escaped_char, position),
                    }
                    position.next_column();
                } else {
                    bail!("Unterminated escape sequence at position {}", position);
                }
            }
            '\n' | '\r' | '\t' => {
                // Unescaped control characters are not allowed in JSON strings
                bail!("Unescaped control character in string at position {}", position);
            }
            c if c.is_control() => {
                // Reject all other control characters (0x00-0x1F)
                bail!("Unescaped control character (0x{:02x}) in string at position {}", c as u32, position);
            }
            _ => {
                result_string.push(char);
                position.next_column();
            }
        }
    }
    bail!("Unterminated string literal at position {}", position)
}

fn parse_keyword(chars: &mut Peekable<Chars>, position: &mut Position) -> Result<Token> {
    let mut result_keywork = String::new();
    while let Some(&char) = chars.peek() {
        if char.is_alphabetic() {
            result_keywork.push(char);
            chars.next();
            position.next_column();
        } else {
            break;
        }
    }
    match result_keywork.as_str() {
        "true" => Ok(Token::True),
        "false" => Ok(Token::False),
        "null" => Ok(Token::Null),
        _ => bail!(
            "Unknown keyword: '{}' at position {}",
            result_keywork,
            position
        ),
    }
}

fn can_accept_value(token_list: &[Token]) -> Result<()> {
    if let Some(last) = token_list.last()
        && !matches!(
            last,
            Token::Comma | Token::Colon | Token::BraceCurlyOpen | Token::BracketSquareOpen
        ) {
            bail!("Unexpected value after {:?}", last);
        }
    Ok(())
}

fn cannot_follow_comma(token_list: &[Token]) -> Result<()> {
    if let Some(last) = token_list.last()
        && matches!(last, Token::Comma) {
            bail!("Unexpected token after comma (trailing comma?)");
        }
    Ok(())
}

fn parse_numbers(chars: &mut Peekable<Chars>, position: &mut Position) -> Result<f64> {
    let mut result_number = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    let mut after_minus = false;

    while let Some(&char) = chars.peek() {
        match char {
            '0'..='9' => {
                // Check for leading zeros (e.g., 013 is invalid, but 0.13 or 0e10 is valid)
                if result_number == "0" || result_number == "-0" {
                    // After a zero, only '.', 'e', 'E', or end is allowed
                    if !matches!(chars.peek(), Some(&'.') | Some(&'e') | Some(&'E')) && char == '0' {
                        // Multiple leading zeros
                    } else if char != '0' {
                        bail!("Invalid number: leading zeros not allowed at position {}", position);
                    }
                }
                result_number.push(char);
                chars.next();
                position.next_column();
                after_minus = false;
            }
            '.' if !has_decimal && !has_exponent => {
                has_decimal = true;
                result_number.push(char);
                chars.next();
                position.next_column();
                after_minus = false;
            }
            '-' if result_number.is_empty() => {
                result_number.push(char);
                chars.next();
                position.next_column();
                after_minus = true;
            }
            '+' | '-' if has_exponent && after_minus => {
                // Sign after exponent (e.g., 1e+10, 1e-10)
                result_number.push(char);
                chars.next();
                position.next_column();
                after_minus = false;
            }
            'e' | 'E' if !has_exponent => {
                has_exponent = true;
                result_number.push(char);
                chars.next();
                position.next_column();
                after_minus = true;
            }
            _ => break,
        }
    }

    let result: f64 = result_number
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid number format: '{}'", result_number))?;
    Ok(result)
}

pub fn tokenize(json_string: String) -> Result<Vec<Token>> {
    let mut token_list = vec![];
    let mut position = Position::new();
    let mut chars = json_string.chars().peekable();
    while let Some(&char) = chars.peek() {
        match char {
            '{' => {
                token_list.push(Token::BraceCurlyOpen);
                chars.next();
                position.next_column();
            }
            '}' => {
                cannot_follow_comma(&token_list)?;
                token_list.push(Token::BraceCurlyClose);
                chars.next();
                position.next_column();
            }
            '[' => {
                token_list.push(Token::BracketSquareOpen);
                chars.next();
                position.next_column();
            }
            ']' => {
                cannot_follow_comma(&token_list)?;
                token_list.push(Token::BracketSquareClose);
                chars.next();
                position.next_column();
            }
            ',' => {
                cannot_follow_comma(&token_list)?;
                token_list.push(Token::Comma);
                chars.next();
                position.next_column();
            }
            ':' => {
                cannot_follow_comma(&token_list)?;
                token_list.push(Token::Colon);
                chars.next();
                position.next_column();
            }
            '"' => {
                can_accept_value(&token_list)?;
                chars.next();
                let string_token = parse_string(&mut chars, &mut position)?;
                token_list.push(Token::String(string_token));
            }
            't' | 'f' | 'n' => {
                can_accept_value(&token_list)?;
                let keyword_token = parse_keyword(&mut chars, &mut position)?;
                token_list.push(keyword_token);
            }
            '0'..='9' | '-' => {
                can_accept_value(&token_list)?;
                let number_token = parse_numbers(&mut chars, &mut position)?;
                token_list.push(Token::Number(number_token));
            }
            _ if char.is_whitespace() => {
                chars.next();
                if char.eq(&'\n') {
                    position.next_row();
                } else {
                    position.next_column();
                }
            }
            _ => bail!("Unexpected character: '{}' at position {}", char, position),
        }
    }

    Ok(token_list)
}

fn parse_object<I>(tokens: &mut Peekable<I>, depth: usize) -> Result<JsonValue>
where
    I: Iterator<Item = Token>,
{
    let mut object: Vec<_> = Vec::new();
    // Consume the `{`
    tokens.next();

    if depth > MAX_DEPTH {
        bail!(
            "Detected nesting level {}, max allowed {}",
            depth,
            MAX_DEPTH
        )
    }
    
    loop {
        // Check for closing brace or key
        match tokens.peek() {
            Some(Token::BraceCurlyClose) => {
                tokens.next();
                break;
            }
            Some(Token::String(_)) => {
                // Parse key
                let key = if let Some(Token::String(k)) = tokens.next() {
                    k
                } else {
                    bail!("Expected string key in object")
                };
                
                // Expect colon
                if !matches!(tokens.next(), Some(Token::Colon)) {
                    bail!("Expected ':' after object key")
                }
                
                // Parse value
                let value = parse_value(tokens, depth)?;
                object.push((key, value));
                
                // Check for comma or closing brace
                match tokens.peek() {
                    Some(Token::Comma) => {
                        tokens.next();
                        // Continue to next key-value pair
                    }
                    Some(Token::BraceCurlyClose) => {
                        // Will be handled in next iteration
                    }
                    _ => bail!("Expected ',' or '}}' in object"),
                }
            }
            _ => bail!("Expected string key or '}}' in object"),
        }
    }
    
    Ok(JsonValue::Object(object))
}
fn parse_array<I>(tokens: &mut Peekable<I>, depth: usize) -> Result<JsonValue>
where
    I: Iterator<Item = Token>,
{
    let mut array: Vec<JsonValue> = Vec::new();
    // Consume the `[`
    tokens.next();

    if depth > MAX_DEPTH {
        bail!(
            "Detected nesting level {}, max allowed {}",
            depth,
            MAX_DEPTH
        )
    }
    
    loop {
        // Check for closing bracket
        match tokens.peek() {
            Some(Token::BracketSquareClose) => {
                tokens.next();
                break;
            }
            Some(_) => {
                // Parse value
                let value = parse_value(tokens, depth)?;
                array.push(value);
                
                // Check for comma or closing bracket
                match tokens.peek() {
                    Some(Token::Comma) => {
                        tokens.next();
                        // Continue to next value
                    }
                    Some(Token::BracketSquareClose) => {
                        // Will be handled in next iteration
                    }
                    _ => bail!("Expected ',' or ']' in array"),
                }
            }
            None => bail!("Unexpected end of tokens in array"),
        }
    }
    
    Ok(JsonValue::Array(array))
}

fn parse_value<I>(tokens: &mut Peekable<I>, depth: usize) -> Result<JsonValue>
where
    I: Iterator<Item = Token>,
{
    if depth > MAX_DEPTH {
        bail!(
            "Detected nesting level {}, max allowed {}",
            depth,
            MAX_DEPTH
        )
    }
    if let Some(token) = tokens.peek() {
        match token {
            Token::BraceCurlyOpen => parse_object(tokens, depth + 1),
            Token::BracketSquareOpen => parse_array(tokens, depth + 1),
            Token::String(_) => {
                if let Some(Token::String(s)) = tokens.next() {
                    Ok(JsonValue::String(s))
                } else {
                    bail!("Expected a String")
                }
            }
            Token::Number(_) => {
                if let Some(Token::Number(n)) = tokens.next() {
                    Ok(JsonValue::Number(n))
                } else {
                    bail!("Expected a Number")
                }
            }
            Token::True => {
                tokens.next();
                Ok(JsonValue::Boolean(true))
            }
            Token::False => {
                tokens.next();
                Ok(JsonValue::Boolean(false))
            }
            Token::Null => {
                tokens.next();
                Ok(JsonValue::Null)
            }
            _ => bail!("Unknown Token detected"),
        }
    } else {
        bail!("Unexpected end of tokens")
    }
}

pub fn parse_tokens(tokens: Vec<Token>) -> Result<JsonValue> {
    let mut token_iterator = tokens.into_iter().peekable();
    
    // Check that the root value is an object or array
    match token_iterator.peek() {
        Some(Token::BraceCurlyOpen) | Some(Token::BracketSquareOpen) => {
            // Valid JSON root
        }
        Some(_) => {
            bail!("JSON must be an object or array, not a primitive value");
        }
        None => {
            bail!("Empty JSON input");
        }
    }
    
    let value = parse_value(&mut token_iterator, 0)?;
    
    // Check that there are no extra tokens after the root value
    if let Some(extra_token) = token_iterator.next() {
        bail!("Unexpected token after root value: {:?}", extra_token);
    }

    Ok(value)
}
