#![feature(custom_derive, plugin, test)]
#![plugin(serde_macros)]

extern crate test;
extern crate serde;

use std::collections::BTreeMap;
use std::iter;
use std::vec;

use serde::de::{self, Deserialize, Deserializer, Visitor};

#[derive(Debug)]
enum Token<'a> {
    Bool(bool),
    Isize(isize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Usize(usize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Char(char),
    Str(&'a str),
    String(String),

    Option(bool),

    Unit,
    NamedUnit(&'a str),

    SeqStart(usize),
    NamedSeqStart(&'a str, usize),
    SeqSep(bool),
    SeqEnd,

    MapStart(usize),
    NamedMapStart(&'a str, usize),
    MapSep(bool),
    MapEnd,

    EnumStart(&'a str),
    EnumEnd,
}

struct TokenDeserializer<'a> {
    tokens: iter::Peekable<vec::IntoIter<Token<'a>>>,
}

impl<'a> TokenDeserializer<'a> {
    fn new(tokens: Vec<Token<'a>>) -> TokenDeserializer<'a> {
        TokenDeserializer {
            tokens: tokens.into_iter().peekable(),
        }
    }
}

#[derive(Copy, PartialEq, Debug)]
enum Error {
    SyntaxError,
    EndOfStreamError,
    MissingFieldError(&'static str),
}

impl de::Error for Error {
    fn syntax_error() -> Error { Error::SyntaxError }

    fn end_of_stream_error() -> Error { Error::EndOfStreamError }

    fn missing_field_error(field: &'static str) -> Error {
        Error::MissingFieldError(field)
    }
}

impl<'a> Deserializer for TokenDeserializer<'a> {
    type Error = Error;

    fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: Visitor,
    {
        match self.tokens.next() {
            Some(Token::Bool(v)) => visitor.visit_bool(v),
            Some(Token::Isize(v)) => visitor.visit_isize(v),
            Some(Token::I8(v)) => visitor.visit_i8(v),
            Some(Token::I16(v)) => visitor.visit_i16(v),
            Some(Token::I32(v)) => visitor.visit_i32(v),
            Some(Token::I64(v)) => visitor.visit_i64(v),
            Some(Token::Usize(v)) => visitor.visit_usize(v),
            Some(Token::U8(v)) => visitor.visit_u8(v),
            Some(Token::U16(v)) => visitor.visit_u16(v),
            Some(Token::U32(v)) => visitor.visit_u32(v),
            Some(Token::U64(v)) => visitor.visit_u64(v),
            Some(Token::F32(v)) => visitor.visit_f32(v),
            Some(Token::F64(v)) => visitor.visit_f64(v),
            Some(Token::Char(v)) => visitor.visit_char(v),
            Some(Token::Str(v)) => visitor.visit_str(v),
            Some(Token::String(v)) => visitor.visit_string(v),
            Some(Token::Option(false)) => visitor.visit_none(),
            Some(Token::Option(true)) => visitor.visit_some(self),
            Some(Token::Unit) => visitor.visit_unit(),
            Some(Token::NamedUnit(name)) => visitor.visit_named_unit(name),
            Some(Token::SeqStart(len)) => {
                visitor.visit_seq(TokenDeserializerSeqVisitor {
                    de: self,
                    len: len,
                    first: true,
                })
            }
            Some(Token::NamedSeqStart(name, len)) => {
                visitor.visit_named_seq(name, TokenDeserializerSeqVisitor {
                    de: self,
                    len: len,
                    first: true,
                })
            }
            Some(Token::MapStart(len)) => {
                visitor.visit_map(TokenDeserializerMapVisitor {
                    de: self,
                    len: len,
                    first: true,
                })
            }
            Some(Token::NamedMapStart(name, len)) => {
                visitor.visit_named_map(name, TokenDeserializerMapVisitor {
                    de: self,
                    len: len,
                    first: true,
                })
            }
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    /// Hook into `Option` deserializing so we can treat `Unit` as a
    /// `None`, or a regular value as `Some(value)`.
    fn visit_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: Visitor,
    {
        match self.tokens.peek() {
            Some(&Token::Option(false)) => {
                self.tokens.next();
                visitor.visit_none()
            }
            Some(&Token::Option(true)) => {
                self.tokens.next();
                visitor.visit_some(self)
            }
            Some(&Token::Unit) => {
                self.tokens.next();
                visitor.visit_none()
            }
            Some(_) => visitor.visit_some(self),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn visit_enum<V>(&mut self, name: &str, mut visitor: V) -> Result<V::Value, Error>
        where V: de::EnumVisitor,
    {
        match self.tokens.next() {
            Some(Token::EnumStart(n)) => {
                if name == n {
                    visitor.visit(TokenDeserializerVariantVisitor {
                        de: self,
                    })
                } else {
                    Err(Error::SyntaxError)
                }
            }
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }
}

//////////////////////////////////////////////////////////////////////////

struct TokenDeserializerSeqVisitor<'a, 'b: 'a> {
    de: &'a mut TokenDeserializer<'b>,
    len: usize,
    first: bool,
}

impl<'a, 'b> de::SeqVisitor for TokenDeserializerSeqVisitor<'a, 'b> {
    type Error = Error;

    fn visit<T>(&mut self) -> Result<Option<T>, Error>
        where T: Deserialize,
    {
        let first = self.first;
        self.first = false;

        match self.de.tokens.next() {
            Some(Token::SeqSep(first_)) if first_ == first => {
                self.len -= 1;
                Ok(Some(try!(Deserialize::deserialize(self.de))))
            }
            Some(Token::SeqEnd) => Ok(None),
            Some(_) => {
                Err(Error::SyntaxError)
            }
            None => Err(Error::EndOfStreamError),
        }
    }

    fn end(&mut self) -> Result<(), Error> {
        match self.de.tokens.next() {
            Some(Token::SeqEnd) => Ok(()),
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

//////////////////////////////////////////////////////////////////////////

struct TokenDeserializerMapVisitor<'a, 'b: 'a> {
    de: &'a mut TokenDeserializer<'b>,
    len: usize,
    first: bool,
}

impl<'a, 'b> de::MapVisitor for TokenDeserializerMapVisitor<'a, 'b> {
    type Error = Error;

    fn visit_key<K>(&mut self) -> Result<Option<K>, Error>
        where K: Deserialize,
    {
        let first = self.first;
        self.first = false;

        match self.de.tokens.next() {
            Some(Token::MapSep(first_)) if first_ == first => {
                Ok(Some(try!(Deserialize::deserialize(self.de))))
            }
            Some(Token::MapEnd) => Ok(None),
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn visit_value<V>(&mut self) -> Result<V, Error>
        where V: Deserialize,
    {
        Ok(try!(Deserialize::deserialize(self.de)))
    }

    fn end(&mut self) -> Result<(), Error> {
        match self.de.tokens.next() {
            Some(Token::MapEnd) => Ok(()),
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

//////////////////////////////////////////////////////////////////////////

struct TokenDeserializerVariantVisitor<'a, 'b: 'a> {
    de: &'a mut TokenDeserializer<'b>,
}

impl<'a, 'b> de::VariantVisitor for TokenDeserializerVariantVisitor<'a, 'b> {
    type Error = Error;

    fn visit_kind<V>(&mut self) -> Result<V, Error>
        where V: de::Deserialize,
    {
        de::Deserialize::deserialize(self.de)
    }

    fn visit_unit(&mut self) -> Result<(), Error> {
        let value = try!(Deserialize::deserialize(self.de));

        match self.de.tokens.next() {
            Some(Token::EnumEnd) => Ok(value),
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::EnumSeqVisitor,
    {
        let token = self.de.tokens.next();
        match token {
            Some(Token::SeqStart(len)) => {
                let value = try!(visitor.visit(TokenDeserializerSeqVisitor {
                    de: self.de,
                    len: len,
                    first: true,
                }));

                match self.de.tokens.next() {
                    Some(Token::EnumEnd) => Ok(value),
                    Some(_) => Err(Error::SyntaxError),
                    None => Err(Error::EndOfStreamError),
                }
            }
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }

    fn visit_map<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::EnumMapVisitor,
    {
        match self.de.tokens.next() {
            Some(Token::MapStart(len)) => {
                let value = try!(visitor.visit(TokenDeserializerMapVisitor {
                    de: self.de,
                    len: len,
                    first: true,
                }));

                match self.de.tokens.next() {
                    Some(Token::EnumEnd) => Ok(value),
                    Some(_) => Err(Error::SyntaxError),
                    None => Err(Error::EndOfStreamError),
                }
            }
            Some(_) => Err(Error::SyntaxError),
            None => Err(Error::EndOfStreamError),
        }
    }
}

//////////////////////////////////////////////////////////////////////////

#[derive(Copy, PartialEq, Debug)]
#[derive_deserialize]
struct NamedUnit;

#[derive(PartialEq, Debug)]
#[derive_deserialize]
struct NamedSeq(i32, i32, i32);

#[derive(PartialEq, Debug)]
#[derive_deserialize]
struct NamedMap {
    a: i32,
    b: i32,
    c: i32,
}

#[derive(PartialEq, Debug)]
#[derive_deserialize]
enum Enum {
    Unit,
    Seq(i32, i32, i32),
    Map { a: i32, b: i32, c: i32 }
}

//////////////////////////////////////////////////////////////////////////

macro_rules! btreemap {
    () => {
        BTreeMap::new()
    };
    ($($key:expr => $value:expr),+) => {
        {
            let mut map = BTreeMap::new();
            $(map.insert($key, $value);)+
            map
        }
    }
}

macro_rules! declare_test {
    ($name:ident { $($value:expr => $tokens:expr,)+ }) => {
        #[test]
        fn $name() {
            $(
                let mut de = TokenDeserializer::new($tokens);
                let value: Result<_, Error> = Deserialize::deserialize(&mut de);
                assert_eq!(value, Ok($value));
            )+
        }
    }
}

macro_rules! declare_tests {
    ($($name:ident { $($value:expr => $tokens:expr,)+ })+) => {
        $(
            declare_test!($name { $($value => $tokens,)+ });
        )+
    }
}

//////////////////////////////////////////////////////////////////////////

declare_tests! {
    test_bool {
        true => vec![Token::Bool(true)],
        false => vec![Token::Bool(false)],
    }
    test_isize {
        0isize => vec![Token::Isize(0)],
        0isize => vec![Token::I8(0)],
        0isize => vec![Token::I16(0)],
        0isize => vec![Token::I32(0)],
        0isize => vec![Token::I64(0)],
        0isize => vec![Token::Usize(0)],
        0isize => vec![Token::U8(0)],
        0isize => vec![Token::U16(0)],
        0isize => vec![Token::U32(0)],
        0isize => vec![Token::U64(0)],
        0isize => vec![Token::F32(0.)],
        0isize => vec![Token::F64(0.)],
    }
    test_ints {
        0isize => vec![Token::Isize(0)],
        0i8 => vec![Token::I8(0)],
        0i16 => vec![Token::I16(0)],
        0i32 => vec![Token::I32(0)],
        0i64 => vec![Token::I64(0)],
    }
    test_uints {
        0usize => vec![Token::Usize(0)],
        0u8 => vec![Token::U8(0)],
        0u16 => vec![Token::U16(0)],
        0u32 => vec![Token::U32(0)],
        0u64 => vec![Token::U64(0)],
    }
    test_floats {
        0f32 => vec![Token::F32(0.)],
        0f64 => vec![Token::F64(0.)],
    }
    test_char {
        'a' => vec![Token::Char('a')],
        'a' => vec![Token::Str("a")],
        'a' => vec![Token::String("a".to_string())],
    }
    test_string {
        "abc".to_string() => vec![Token::Str("abc")],
        "abc".to_string() => vec![Token::String("abc".to_string())],
        "a".to_string() => vec![Token::Char('a')],
    }
    test_option {
        None::<i32> => vec![Token::Unit],
        None::<i32> => vec![Token::Option(false)],
        Some(1) => vec![Token::I32(1)],
        Some(1) => vec![
            Token::Option(true),
            Token::I32(1),
        ],
    }
    test_unit {
        () => vec![Token::Unit],
        () => vec![
            Token::SeqStart(0),
            Token::SeqEnd,
        ],
        () => vec![
            Token::NamedSeqStart("Anything", 0),
            Token::SeqEnd,
        ],
    }
    test_named_unit {
        NamedUnit => vec![Token::Unit],
        NamedUnit => vec![Token::NamedUnit("NamedUnit")],
        NamedUnit => vec![
            Token::SeqStart(0),
            Token::SeqEnd,
        ],
    }
    test_named_seq {
        NamedSeq(1, 2, 3) => vec![
            Token::SeqStart(3),
                Token::SeqSep(true),
                Token::I32(1),

                Token::SeqSep(false),
                Token::I32(2),

                Token::SeqSep(false),
                Token::I32(3),
            Token::SeqEnd,
        ],
        NamedSeq(1, 2, 3) => vec![
            Token::NamedSeqStart("NamedSeq", 3),
                Token::SeqSep(true),
                Token::I32(1),

                Token::SeqSep(false),
                Token::I32(2),

                Token::SeqSep(false),
                Token::I32(3),
            Token::SeqEnd,
        ],
    }
    test_vec {
        Vec::<isize>::new() => vec![
            Token::SeqStart(0),
            Token::SeqEnd,
        ],
        vec![vec![], vec![1], vec![2, 3]] => vec![
            Token::SeqStart(3),
                Token::SeqSep(true),
                Token::SeqStart(0),
                Token::SeqEnd,

                Token::SeqSep(false),
                Token::SeqStart(1),
                    Token::SeqSep(true),
                    Token::I32(1),
                Token::SeqEnd,

                Token::SeqSep(false),
                Token::SeqStart(2),
                    Token::SeqSep(true),
                    Token::I32(2),

                    Token::SeqSep(false),
                    Token::I32(3),
                Token::SeqEnd,
            Token::SeqEnd,
        ],
    }
    test_tuple {
        (1,) => vec![
            Token::SeqStart(1),
                Token::SeqSep(true),
                Token::I32(1),
            Token::SeqEnd,
        ],
        (1, 2, 3) => vec![
            Token::SeqStart(3),
                Token::SeqSep(true),
                Token::I32(1),

                Token::SeqSep(false),
                Token::I32(2),

                Token::SeqSep(false),
                Token::I32(3),
            Token::SeqEnd,
        ],
    }
    test_btreemap {
        btreemap![1 => 2] => vec![
            Token::MapStart(1),
                Token::MapSep(true),
                Token::I32(1),
                Token::I32(2),
            Token::MapEnd,
        ],
        btreemap![1 => 2, 3 => 4] => vec![
            Token::MapStart(2),
                Token::MapSep(true),
                Token::I32(1),
                Token::I32(2),

                Token::MapSep(false),
                Token::I32(3),
                Token::I32(4),
            Token::MapEnd,
        ],
        btreemap![1 => btreemap![], 2 => btreemap![3 => 4, 5 => 6]] => vec![
            Token::MapStart(2),
                Token::MapSep(true),
                Token::I32(1),
                Token::MapStart(0),
                Token::MapEnd,

                Token::MapSep(false),
                Token::I32(2),
                Token::MapStart(2),
                    Token::MapSep(true),
                    Token::I32(3),
                    Token::I32(4),

                    Token::MapSep(false),
                    Token::I32(5),
                    Token::I32(6),
                Token::MapEnd,
            Token::MapEnd,
        ],
    }
    test_named_map {
        NamedMap { a: 1, b: 2, c: 3 } => vec![
            Token::MapStart(3),
                Token::MapSep(true),
                Token::Str("a"),
                Token::I32(1),

                Token::MapSep(false),
                Token::Str("b"),
                Token::I32(2),

                Token::MapSep(false),
                Token::Str("c"),
                Token::I32(3),
            Token::MapEnd,
        ],
        NamedMap { a: 1, b: 2, c: 3 } => vec![
            Token::NamedMapStart("NamedMap", 3),
                Token::MapSep(true),
                Token::Str("a"),
                Token::I32(1),

                Token::MapSep(false),
                Token::Str("b"),
                Token::I32(2),

                Token::MapSep(false),
                Token::Str("c"),
                Token::I32(3),
            Token::MapEnd,
        ],
    }
    test_enum_unit {
        Enum::Unit => vec![
            Token::EnumStart("Enum"),
                Token::Str("Unit"),
                Token::Unit,
            Token::EnumEnd,
        ],
    }
    test_enum_seq {
        Enum::Seq(1, 2, 3) => vec![
            Token::EnumStart("Enum"),
                Token::Str("Seq"),
                Token::SeqStart(3),
                    Token::SeqSep(true),
                    Token::I32(1),

                    Token::SeqSep(false),
                    Token::I32(2),

                    Token::SeqSep(false),
                    Token::I32(3),
                Token::SeqEnd,
            Token::EnumEnd,
        ],
    }
    test_enum_map {
        Enum::Map { a: 1, b: 2, c: 3 } => vec![
            Token::EnumStart("Enum"),
                Token::Str("Map"),
                Token::MapStart(3),
                    Token::MapSep(true),
                    Token::Str("a"),
                    Token::I32(1),

                    Token::MapSep(false),
                    Token::Str("b"),
                    Token::I32(2),

                    Token::MapSep(false),
                    Token::Str("c"),
                    Token::I32(3),
                Token::MapEnd,
            Token::EnumEnd,
        ],
    }
}
