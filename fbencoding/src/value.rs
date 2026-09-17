use std::collections::BTreeMap;
use std::fmt;

/// The various errors encountered when decoding a Bencoded value.
#[derive(Debug, PartialEq)]
pub enum DecodeError {
    Invalid,
    /// Unable to decode the [u8] for the described reason
    ParseError(String),
    /// A poorly formatted number
    NAN(String),
}

/// # Simple Bencoding Library to use with our toy Torrent client
///
/// This allows for simple de/serialisation of [Bencode](https://en.wikipedia.org/wiki/Bencode)
/// binary encoded data.
///
/// # Examples:
/// ```
/// use fbencoding::Value;
///
/// let example = Value::decode(b"l5:hellod5:worldi32eee");
/// ```
///
/// or
/// 
/// ```
/// let binary = Value::List(vec/[Value::Int(10), Value::ByteString(b"Hello"),
/// Value::String("World!")]);
/// println/("Encoded: {:?}", binary.encode());
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum Value<'a> {
    Int(i64),
    ByteString(&'a [u8]),
    List(Vec<Value<'a>>),
    Dict(BTreeMap<&'a str, Value<'a>>),

    /// Helpers for encoding/decoding strings, which are just byte strings with UTF-8 content.
    String(&'a str),
}

/// Captures the state for decoding a [u8] into Value.
struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
}

#[allow(dead_code)]
impl<'a> Decoder<'a> {
    fn new(input: &'a [u8]) -> Decoder<'a> {
        Decoder {
            input: input,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    /// Decode the [u8] added when instantiating the Decoder into a Value.
    ///
    /// # Example:
    ///
    /// ```
    /// use crate::Decoder
    /// let raw = Decoder::new(b'li32e6:foobare')
    /// let value = Decoder.parse_value(); // Value::List(vec![
    ///                                    //               Value::Int(32),
    ///                                    //               Value::ByteString(b"foobar")
    ///                                    // ]);
    /// ```
    fn parse_value(&mut self) -> Result<Value<'a>, DecodeError> {
        match self.peek() {
            Some(b'i') => self.parse_int(),
            Some(b'l') => self.parse_list(),
            Some(b'd') => self.parse_dict(),
            Some(b'0'..=b'9') => self.parse_string(),
            _ => Err(DecodeError::ParseError(format!(
                "Unexpected token @ '{}' in '{:?}'",
                self.pos, self.input
            ))),
        }
    }

    fn parse_int(&mut self) -> Result<Value<'a>, DecodeError> {
        let start = self.pos;
        self.pos += 1; // Skip 'i'

        let is_negative = self.peek() == Some(b'-');

        let number = self.parse_number()?;

        // Validation:
        if is_negative && number == 0 {
            return Err(DecodeError::ParseError(
                "Negative zero is not allowed in bencoding".to_string(),
            ));
        }

        if self.peek() != Some(b'e') {
            return Err(DecodeError::ParseError(format!(
                "Int wasn't terminated with a 'e': '{}'",
                String::from_utf8_lossy(&self.input[start..self.pos])
            )));
        }

        self.pos += 1; // Move past 'e'
        Ok(Value::Int(number))
    }

    // Helper to parse numbers, which are used in two places
    //
    // Note: this also moves self.pos forward.
    fn parse_number(&mut self) -> Result<i64, DecodeError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }

        if self.pos == digits_start {
            return Err(DecodeError::ParseError(
                "Failed to parse integer".to_string(),
            ));
        }

        let number_string = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| DecodeError::ParseError("Invalid UTF-8 in integer".to_string()))?;
        number_string
            .parse()
            .map_err(|_| DecodeError::ParseError("Failed to parse integer".to_string()))
    }

    fn parse_list(&mut self) -> Result<Value<'a>, DecodeError> {
        self.pos += 1; // Skip 'l'
        let mut entries: Vec<Value> = vec![];
        while self.peek() != Some(b'e') {
            entries.push(self.parse_value()?);
        }
        self.pos += 1; // Skip 'e'
        Ok(Value::List(entries))
    }

    fn parse_dict(&mut self) -> Result<Value<'a>, DecodeError> {
        self.pos += 1; // Skip 'd'
        let mut dict: BTreeMap<&'a str, Value<'a>> = BTreeMap::new();
        while self.peek() != Some(b'e') {
            let key = std::str::from_utf8(self.parse_bytes()?)
                .map_err(|_| DecodeError::ParseError("Dict key is not valid UTF-8".to_string()))?;
            let value = self.parse_value()?;
            dict.insert(key, value);
        }
        self.pos += 1; // Skip 'e'
        Ok(Value::Dict(dict))
    }

    // Used directly by parse_string, and by parse_dict for keys, which need
    // the raw bytes rather than a Value wrapper.
    fn parse_bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let length = self.parse_number()? as usize;

        if self.peek() != Some(b':') {
            return Err(DecodeError::ParseError(format!(
                "Poorly formatted string, expecting ':' @ {}",
                self.pos
            )));
        }

        self.pos += 1;

        let bytes = self
            .input
            .get(self.pos..self.pos + length)
            .ok_or(DecodeError::ParseError(format!(
                "String[{}, {}] is out of bounds in '{:?}'",
                self.pos,
                self.pos + length,
                self.input
            )))?;

        self.pos += length;
        Ok(bytes)
    }

    fn parse_string(&mut self) -> Result<Value<'a>, DecodeError> {
        Ok(Value::ByteString(self.parse_bytes()?))
    }
}

impl<'a> Value<'a> {
    fn fmt_indented(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let inner_pad = "    ".repeat(indent + 1);
        match self {
            Value::Int(i) => write!(f, "Int({})", i),
            Value::ByteString(b) => {
                match std::str::from_utf8(b) {
                    Ok(s) => write!(f, "ByteString({:?})", s),
                    Err(_) => write!(f, "ByteString({:?})", b),
                }
            }
            Value::List(list) => {
                if list.is_empty() {
                    return write!(f, "List([])");
                }
                writeln!(f, "List([")?;
                for v in list {
                    write!(f, "{}", inner_pad)?;
                    v.fmt_indented(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                write!(f, "{}])", "    ".repeat(indent))
            }
            Value::Dict(dict) => {
                if dict.is_empty() {
                    return write!(f, "Dict({{}})");
                }
                writeln!(f, "Dict({{")?;
                for (k, v) in dict {
                    write!(f, "{}{:?}: ", inner_pad, k)?;
                    v.fmt_indented(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                write!(f, "{}}})", "    ".repeat(indent))
            }
            _ => write!(f, "<Unsupported Value variant for Display>"),
        }
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indented(f, 0)
    }
}

impl<'a> Value<'a> {
    /// Convert a Value instance into a [u8]. If you use a List of Dict it'll convert the set of
    /// Values into a [u8].
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Value::Int(i) => format!("i{}e", i).into_bytes(),
            Value::ByteString(b) => {
                format!("{}:{}", b.len(), String::from_utf8_lossy(b)).into_bytes()
            }
            Value::List(list) => {
                let content: Vec<u8> = list.iter().map(|v| v.encode()).flatten().collect();
                let mut result: Vec<u8> = Vec::with_capacity(content.len() + 2);
                result.push('l' as u8);
                result.extend_from_slice(&content);
                result.push('e' as u8);
                result
            }
            Value::Dict(dict) => {
                let mut result = vec![];
                result.push('d' as u8);
                for (key, value) in dict {
                    result.extend_from_slice(&Value::ByteString(key.as_bytes()).encode());
                    result.extend_from_slice(&value.encode());
                }
                result.push('e' as u8);
                result
            }

            // Helper wrappers for writing test
            Value::String(s) => Value::ByteString(s.as_bytes()).encode(),
        }
    }

    pub fn decode(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        Decoder::new(bytes).parse_value()
    }
}

fn read_number(bytes: &[u8]) -> Result<(i64, &[u8]), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::NAN("Empty number".to_string()));
    }

    let negative = bytes[0] == b'-';
    if negative && bytes.len() == 1 {
        return Err(DecodeError::NAN("Only a '-' operator provided".to_string()));
    }
    let content = if negative { &bytes[1..] } else { bytes };

    // Allowed 0, but can't be leading a zero: e.g. 012 or -01
    let is_zero = Some(&b'0') == content.get(0);
    if negative && is_zero {
        return Err(DecodeError::NAN("Negative zero is not allowed".to_string()));
    }

    let is_leading_zero = is_zero && content.get(1).map_or(false, |&b| b.is_ascii_digit());
    if is_leading_zero {
        return Err(DecodeError::NAN("Leading 0's not allowed, e.g. 012 or -01".to_string()));
    }

    if content.is_empty() || !content[0].is_ascii_digit() {
        return Err(DecodeError::NAN("Unable to parse the string into a number".to_string()));
    }

    let mut number: i64 = 0;
    let mut consumed = 0;
    for &byte in content {
        if !byte.is_ascii_digit() {
            break;
        }
        number = number * 10 + (byte - b'0') as i64;
        consumed += 1;
    }
    if negative {
        number = number * -1;
    }
    let total_consumed = if negative { consumed + 1 } else { consumed };
    Ok((number, &bytes[total_consumed..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_integer() {
        let v = Value::decode(b"i42e").unwrap();
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn test_decode_bad_integer_error() {
        let v = Value::decode(b"i42BAD_TEXTe");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "Invalid integer should return a parse error"
        );

        let v = Value::decode(b"i42");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "Missing 'e' terminator for integer"
        );

        let v = Value::decode(b"ie");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "Empty integer is not allowed in bencoding"
        );

        let v = Value::decode(b"i-0e");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "Negative zero is not allowed in bencoding"
        );
    }

    #[test]
    fn test_decode_string() {
        let v = Value::decode(b"7:bencode").unwrap();
        assert_eq!(v, Value::ByteString(b"bencode"));
    }

    #[test]
    fn test_decode_bad_strings() {
        let v = Value::decode(b"100:badcode");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "String length exceeds available bytes"
        );
    }

    #[test]
    fn encodes_integer() {
        let v = Value::Int(42);
        assert_eq!(v.encode(), b"i42e");
    }

    #[test]
    fn encodes_bytes() {
        let v = Value::String("bencode");
        assert_eq!(v.encode(), b"7:bencode");

        // Make sure our helper function works as well
        assert_eq!(v.encode(), Value::ByteString(b"bencode").encode());
    }

    #[test]
    fn encodes_list() {
        let v = Value::List(vec![Value::String("bencode"), Value::Int(42)]);
        assert_eq!(v.encode(), b"l7:bencodei42ee");
    }

    #[test]
    fn decode_list() {
        let v = Value::decode(b"l7:bencodei42ee").unwrap();
        assert_eq!(
            v,
            Value::List(vec![Value::ByteString(b"bencode"), Value::Int(42)])
        );
    }

    #[test]
    fn decode_bad_list() {
        let v = Value::decode(b"l7:bencodei42e");
        assert!(
            matches!(v, Err(DecodeError::ParseError(_))),
            "List needs to end with an 'e' character"
        );
    }

    #[test]
    fn encodes_dict() {
        // Example of a MetaInfo file
        let metainfo = Value::Dict(BTreeMap::from([
            ("announce", Value::String("https://a.com/announce")),
            (
                "info",
                Value::Dict(BTreeMap::from([("name", Value::String("file.txt"))])),
            ),
        ]));
        println!("Metainfo: {}", String::from_utf8_lossy(&metainfo.encode()));
        assert_eq!(
            metainfo.encode(),
            b"d8:announce22:https://a.com/announce4:infod4:name8:file.txtee"
        );
    }

    #[test]
    fn decode_dict() {
        let v = Value::decode(b"d8:announce22:https://a.com/announce4:infod4:name8:file.txtee")
            .unwrap();
        assert_eq!(
            v,
            Value::Dict(BTreeMap::from([
                ("announce", Value::ByteString(b"https://a.com/announce")),
                (
                    "info",
                    Value::Dict(BTreeMap::from([("name", Value::ByteString(b"file.txt")),]))
                ),
            ]))
        );
    }

    #[test]
    fn test_bytestring_formats_correctly() {
        // Printable should show as UTF-8
        let v = Value::ByteString(b"Hello, World!");
        assert_eq!(format!("{}", v), "ByteString(\"Hello, World!\")");
        let hello = [0x68, 0x65, 0x6C, 0x6C, 0x6F];
        assert_eq!(hello, *b"hello");
        assert_eq!(format!("{}", Value::ByteString(&hello)), "ByteString(\"hello\")");

        // Non-printable should show as a vector of bytes
        let v = Value::ByteString(&[104, 101, 108, 108, 111, 255]);
        assert_eq!(format!("{}", v), "ByteString([104, 101, 108, 108, 111, 255])");
    }

    #[test]
    fn test_read_number() {
        // Misuse
        assert!(read_number(b"i32e").is_err());
        assert!(read_number(b"").is_err());
        assert!(read_number(b"01").is_err());
        assert!(read_number(b"-01").is_err());
        assert!(read_number(b"-0").is_err());

        // Simple case
        assert_eq!(read_number(b"0").unwrap().0, 0);
        assert_eq!(read_number(b"1").unwrap().0, 1);
        assert_eq!(read_number(b"-1").unwrap().0, -1);
        assert_eq!(read_number(b"123").unwrap().0, 123);

        // More nuanced
        assert_eq!(read_number(b"321e").unwrap().0, 321);
    }
}
