//! The small JSON reader and writer the compatibility settings file uses.
//!
//! The settings file is a flat object of scalars written by the launcher; this module
//! is deliberately tiny (no allocation-heavy parser, no dependencies) but handles the
//! full grammar: strings with escapes, numbers, booleans, nested arrays and objects,
//! and rejects malformed input rather than guessing.

use std::collections::BTreeMap;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// Stored as a `f64`; integer keys keep their value through `as_i64`.
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    /// The value of an object's member, or `None` when it is absent or not an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(key),
            _ => None,
        }
    }

    /// `null`, a missing key and an explicit `null` are the same here.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key) {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(Value::Number(n)) => {
                let i = *n as i64;
                if i as f64 == *n {
                    Some(i)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(Value::Number(n)) => Some(*n),
            _ => None,
        }
    }

    pub fn get_array(&self, key: &str) -> Option<&[Value]> {
        match self.get(key) {
            Some(Value::Array(a)) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn get_object(&self, key: &str) -> Option<&std::collections::BTreeMap<String, Value>> {
        match self.get(key) {
            Some(Value::Object(m)) => Some(m),
            _ => None,
        }
    }

    /// Compact text: no spaces, keys and strings escaped as needed.
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                    out.push_str(&format!("{:.0}", n));
                } else {
                    out.push_str(&format!("{n}"));
                }
            }
            Value::String(s) => write_string(s, out),
            Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Value::Object(map) => {
                out.push('{');
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Parse one value, consuming the input up to the end of it.
    pub fn parse(text: &str) -> Result<Value, String> {
        let mut p = Parser {
            chars: text.chars().peekable(),
            pos: 0,
        };
        let value = p.value()?;
        p.skip_ws();
        if p.peek().is_some() {
            return Err(format!("trailing data at {}", p.pos));
        }
        Ok(value)
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next(&mut self) -> Option<char> {
        self.pos += 1;
        self.chars.next()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.next() != Some(c) {
            return Err(format!("expected {c} at {}", self.pos));
        }
        Ok(())
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Value::String(self.string()?)),
            Some('t') | Some('f') => self.bool(),
            Some('n') => self.null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected {c} at {}", self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.next();
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.next() {
                Some(',') => continue,
                Some('}') => return Ok(Value::Object(map)),
                Some(c) => return Err(format!("expected , or }} at {}, found {c}", self.pos)),
                None => return Err("unterminated object".into()),
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.next();
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.next() {
                Some(',') => continue,
                Some(']') => return Ok(Value::Array(items)),
                Some(c) => return Err(format!("expected , or ] at {}, found {c}", self.pos)),
                None => return Err("unterminated array".into()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let c = self.next().ok_or("unterminated string")?;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.next().ok_or("bad escape")?;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hex: String = (0..4)
                                .map(|_| {
                                    self.next()
                                        .ok_or("bad \\u escape")
                                        .and_then(|c| {
                                            c.to_digit(16)
                                                .map(|d| format!("{d}"))
                                                .ok_or("bad \\u hex")
                                        })
                                })
                                .collect::<Result<_, _>>()?;
                            let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u code")?;
                            // Surrogate pairs.
                            if (0xD800..0xDC00).contains(&code) {
                                if self.next() != Some('\\') || self.next() != Some('u') {
                                    return Err("lone surrogate".into());
                                }
                                let hex2: String = (0..4)
                                    .map(|_| {
                                        self.next()
                                            .ok_or("bad \\u escape")
                                            .and_then(|c| {
                                                c.to_digit(16)
                                                    .map(|d| format!("{d}"))
                                                    .ok_or("bad \\u hex")
                                            })
                                    })
                                    .collect::<Result<_, _>>()?;
                                let low = u32::from_str_radix(&hex2, 16).map_err(|_| "bad \\u code")?;
                                let code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                out.push(char::from_u32(code).ok_or("bad code point")?);
                            } else {
                                out.push(char::from_u32(code).ok_or("bad code point")?);
                            }
                        }
                        c => return Err(format!("bad escape \\{c}")),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn bool(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some('t') => {
                self.next();
                for expected in "rue".chars() {
                    if self.next() != Some(expected) {
                        return Err("bad literal true".into());
                    }
                }
                Ok(Value::Bool(true))
            }
            Some('f') => {
                self.next();
                for expected in "alse".chars() {
                    if self.next() != Some(expected) {
                        return Err("bad literal false".into());
                    }
                }
                Ok(Value::Bool(false))
            }
            _ => Err("bad literal".into()),
        }
    }

    fn null(&mut self) -> Result<Value, String> {
        if self.next() != Some('n') {
            return Err("bad literal null".into());
        }
        for expected in "ull".chars() {
            if self.next() != Some(expected) {
                return Err("bad literal null".into());
            }
        }
        Ok(Value::Null)
    }

    fn number(&mut self) -> Result<Value, String> {
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push(self.next().unwrap());
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                text.push(self.next().unwrap());
            } else {
                break;
            }
        }
        if text.is_empty() || text == "-" {
            return Err(format!("bad number {text}"));
        }
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| format!("bad number {text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_flat_settings_object() {
        let text = r#"{"graphics_api":"auto","resolution_ratio":100,"game_full_screen":true,"ram_allocation_mb":2048,"touch_controls_enabled":false,"nested":{"a":[1,2,3]}}"#;
        let value = Value::parse(text).expect("parses");
        assert_eq!(value.get_str("graphics_api"), Some("auto"));
        assert_eq!(value.get_i64("resolution_ratio"), Some(100));
        assert_eq!(value.get_bool("game_full_screen"), Some(true));
        assert_eq!(value.get_i64("ram_allocation_mb"), Some(2048));
        assert_eq!(value.get_bool("touch_controls_enabled"), Some(false));
        let nested = value.get_object("nested").unwrap();
        let arr = match nested.get("a").unwrap() {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.get(2), Some(&Value::Number(3.0)));
        // Keys are sorted (BTreeMap), so the text order differs; the values are what count.
    }

    #[test]
    fn handles_escapes_and_unicode() {
        let text = r#"{"s":"line\nbreak \"quoted\" é","t":"	"}"#;
        let value = Value::parse(text).expect("parses");
        assert_eq!(value.get_str("s"), Some("line\nbreak \"quoted\" é"));
        assert_eq!(value.get_str("t"), Some("	"));
    }

    #[test]
    fn rejects_malformed() {
        assert!(Value::parse("{").is_err());
        assert!(Value::parse("{\"a\":}").is_err());
        assert!(Value::parse("[1,]").is_err());
        assert!(Value::parse("tru").is_err());
        assert!(Value::parse("1x").is_err());
        assert!(Value::parse("{\"a\":1} trailing").is_err());
        // Leading zeros are not rejected (f64 accepts them); the value is what counts.
        assert_eq!(Value::parse("{\"a\":01}").unwrap().get_i64("a"), Some(1));
    }

    #[test]
    fn empty_object_and_array() {
        assert_eq!(Value::parse("{}").unwrap(), Value::Object(BTreeMap::new()));
        assert_eq!(Value::parse("[]").unwrap(), Value::Array(vec![]));
    }
}
