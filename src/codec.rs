use std::collections::BTreeMap;
use std::fmt;

use crate::{Block, Delimiter, Document, expectation::DottedExpectation, parser::AtomCharacter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DotosDecodeError {
    Parse(String),
    ExpectedSingleRoot {
        found: usize,
    },
    ExpectedDelimited {
        type_name: &'static str,
        delimiter: &'static str,
    },
    ExpectedRootCount {
        type_name: &'static str,
        expected: usize,
        found: usize,
    },
    ExpectedAtom {
        type_name: &'static str,
    },
    UnknownVariant {
        enum_name: &'static str,
        variant: String,
    },
    NonCanonicalStringDelimiter {
        value: String,
        canonical: String,
    },
    InvalidInteger {
        value: String,
    },
    InvalidValue {
        type_name: &'static str,
        value: String,
        reason: String,
    },
    ExpectedDottedEntry {
        expectation: &'static str,
    },
    DottedEntryCaseMismatch {
        expectation: &'static str,
        prefix: String,
    },
    DottedEntryMissingValue {
        expectation: &'static str,
    },
}

impl fmt::Display for DotosDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::ExpectedSingleRoot { found } => {
                write!(
                    formatter,
                    "expected exactly one DOTOS root object, found {found}"
                )
            }
            Self::ExpectedDelimited {
                type_name,
                delimiter,
            } => {
                write!(formatter, "expected {type_name} to be a {delimiter} block")
            }
            Self::ExpectedRootCount {
                type_name,
                expected,
                found,
            } => {
                write!(
                    formatter,
                    "expected {type_name} to hold {expected} root objects, found {found}"
                )
            }
            Self::ExpectedAtom { type_name } => write!(formatter, "expected {type_name} atom"),
            Self::UnknownVariant { enum_name, variant } => {
                write!(formatter, "unknown {enum_name} variant {variant}")
            }
            Self::NonCanonicalStringDelimiter { value, canonical } => write!(
                formatter,
                "non-canonical string delimiter for {value:?}: use {canonical}"
            ),
            Self::InvalidInteger { value } => write!(formatter, "invalid integer {value}"),
            Self::InvalidValue {
                type_name,
                value,
                reason,
            } => write!(formatter, "invalid {type_name} {value:?}: {reason}"),
            Self::ExpectedDottedEntry { expectation } => write!(
                formatter,
                "expected a {expectation} entry written as key.value"
            ),
            Self::DottedEntryCaseMismatch {
                expectation,
                prefix,
            } => write!(
                formatter,
                "{expectation} head {prefix:?} has the wrong case for this position"
            ),
            Self::DottedEntryMissingValue { expectation } => write!(
                formatter,
                "{expectation} entry ends at the period with no following value"
            ),
        }
    }
}

impl std::error::Error for DotosDecodeError {}

impl From<crate::DotosError> for DotosDecodeError {
    fn from(error: crate::DotosError) -> Self {
        Self::Parse(error.to_string())
    }
}

pub trait DotosDecode: Sized {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError>;
}

pub trait DotosEncode {
    fn to_dotos(&self) -> String;
}

pub trait DotosBodyDecode: Sized {
    fn from_dotos_body(body: &DotosBody<'_>) -> Result<Self, DotosDecodeError>;
}

pub trait DotosBodyEncode {
    fn to_dotos_body(&self) -> DotosBodyEncoding;
}

pub trait DotosDocumentDecode: Sized {
    fn from_dotos_document_body(body: &DotosDocumentBody<'_>) -> Result<Self, DotosDecodeError>;
}

pub trait DotosDocumentEncode {
    fn to_dotos_document_body(&self) -> DotosDocumentEncoding;
}

pub trait DotosNamedDocumentFieldDecode: Sized {
    fn from_dotos_named_document_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, DotosDecodeError>;
}

pub trait DotosNamedDocumentFieldEncode {
    fn to_dotos_named_document_field_body(&self) -> String;
}

pub trait DotosNamedBodyFieldDecode: Sized {
    fn from_dotos_named_body_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, DotosDecodeError>;
}

pub trait DotosNamedBodyFieldEncode {
    fn to_dotos_named_body_field(&self) -> String;
}

impl<Value> DotosNamedBodyFieldDecode for Value
where
    Value: DotosNamedDocumentFieldDecode,
{
    fn from_dotos_named_body_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, DotosDecodeError> {
        Value::from_dotos_named_document_field(name, block)
    }
}

impl<Value> DotosNamedBodyFieldEncode for Value
where
    Value: DotosNamedDocumentFieldEncode,
{
    fn to_dotos_named_body_field(&self) -> String {
        self.to_dotos_named_document_field_body()
    }
}

pub struct DotosSource<'source> {
    source: &'source str,
}

impl<'source> DotosSource<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    pub fn parse_root(&self) -> Result<Block, DotosDecodeError> {
        let document = Document::parse(self.source)?;
        if document.holds_root_objects() != 1 {
            return Err(DotosDecodeError::ExpectedSingleRoot {
                found: document.holds_root_objects(),
            });
        }
        Ok(document
            .root_object_at(0)
            .expect("root count checked")
            .clone())
    }

    pub fn parse<Value>(&self) -> Result<Value, DotosDecodeError>
    where
        Value: DotosDecode,
    {
        let root = self.parse_root()?;
        Value::from_dotos_block(&root)
    }

    pub fn parse_document_body<Value>(&self) -> Result<Value, DotosDecodeError>
    where
        Value: DotosDocumentDecode,
    {
        let document = Document::parse(self.source)?;
        let body = DotosDocumentBody::new(&document);
        Value::from_dotos_document_body(&body)
    }

    pub fn parse_body<Value>(&self) -> Result<Value, DotosDecodeError>
    where
        Value: DotosBodyDecode,
    {
        let document = Document::parse(self.source)?;
        let body = DotosBody::from_document(&document);
        Value::from_dotos_body(&body)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DotosBody<'body> {
    root_objects: &'body [Block],
}

impl<'body> DotosBody<'body> {
    pub fn new(root_objects: &'body [Block]) -> Self {
        Self { root_objects }
    }

    pub fn from_document(document: &'body Document) -> Self {
        Self::new(document.root_objects())
    }

    pub fn from_delimited(
        block: &'body Block,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<Self, DotosDecodeError> {
        let root_objects =
            block
                .as_delimited(delimiter)
                .ok_or(DotosDecodeError::ExpectedDelimited {
                    type_name,
                    delimiter: delimiter.description(),
                })?;
        Ok(Self::new(root_objects))
    }

    pub fn root_objects(&self) -> &'body [Block] {
        self.root_objects
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'body [Block], DotosDecodeError> {
        let found = self.root_objects().len();
        if found != expected {
            return Err(DotosDecodeError::ExpectedRootCount {
                type_name,
                expected,
                found,
            });
        }
        Ok(self.root_objects())
    }
}

pub struct DotosDocumentBody<'document> {
    body: DotosBody<'document>,
}

impl<'document> DotosDocumentBody<'document> {
    pub fn new(document: &'document Document) -> Self {
        Self {
            body: DotosBody::from_document(document),
        }
    }

    pub fn from_body(body: DotosBody<'document>) -> Self {
        Self { body }
    }

    pub fn as_body(&self) -> &DotosBody<'document> {
        &self.body
    }

    pub fn root_objects(&self) -> &'document [Block] {
        self.body.root_objects()
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'document [Block], DotosDecodeError> {
        self.body.expect_fields(type_name, expected)
    }
}

pub struct DotosBodyEncoding {
    fields: Vec<String>,
}

impl DotosBodyEncoding {
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    pub fn to_dotos(&self) -> String {
        self.fields.join("\n")
    }

    pub fn to_delimited_dotos(&self, delimiter: Delimiter) -> String {
        delimiter.wrap(self.fields.iter().cloned())
    }
}

pub type DotosDocumentEncoding = DotosBodyEncoding;

pub struct DotosBlock<'block> {
    block: &'block Block,
}

impl<'block> DotosBlock<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn expect_children(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'block [Block], DotosDecodeError> {
        self.expect_body(delimiter, type_name)?
            .expect_fields(type_name, expected)
    }

    pub fn expect_delimited(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<&'block [Block], DotosDecodeError> {
        Ok(self.expect_body(delimiter, type_name)?.root_objects())
    }

    pub fn expect_body(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<DotosBody<'block>, DotosDecodeError> {
        DotosBody::from_delimited(self.block, delimiter, type_name)
    }

    pub fn parse_string(&self) -> Result<String, DotosDecodeError> {
        // Pipe text carries a literal, but a pipe wrapper around content that a
        // simpler canonical form could hold is non-canonical and rejected.
        if self.block.is_pipe_text() {
            let text = self
                .block
                .demote_to_string()
                .expect("pipe text demotes to its literal");
            DotosString::new(text).reject_redundant_delimiter(StringForm::PipeText)?;
            return Ok(text.to_owned());
        }
        // A bare atom or a dotted chain of atoms is the string's flat text: an
        // expected `String` reclaims the text the structural period split into a
        // raw dot-application, exactly as the numeric readers reclaim a
        // fractional literal. The rejoin is case-blind — `file.txt`, `Foo.bar`,
        // and a multi-dot host such as `nix.prometheus.goldragon.criome` all
        // rejoin to their bare content regardless of atom case.
        if let Some(text) = self.block.dotted_text() {
            return Ok(text);
        }
        // A parenthesis holds space-joined children, each itself a string.
        if let Some(root_objects) = self.block.as_delimited(Delimiter::Parenthesis) {
            let text = root_objects
                .iter()
                .map(|block| DotosBlock::new(block).parse_string())
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join(" "))?;
            DotosString::new(&text).reject_redundant_delimiter(StringForm::Parenthesis)?;
            return Ok(text);
        }
        Err(DotosDecodeError::ExpectedDelimited {
            type_name: "String",
            delimiter: "string atom, dotted application, or parenthesis",
        })
    }

    pub fn parse_integer(&self) -> Result<u64, DotosDecodeError> {
        let value = self.parse_numeric_text("Integer")?;
        value
            .parse::<u64>()
            .map_err(|_| DotosDecodeError::InvalidInteger { value })
    }

    pub fn parse_u16(&self) -> Result<u16, DotosDecodeError> {
        let value = self.parse_integer()?;
        u16::try_from(value).map_err(|_| DotosDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u8(&self) -> Result<u8, DotosDecodeError> {
        let value = self.parse_integer()?;
        u8::try_from(value).map_err(|_| DotosDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u32(&self) -> Result<u32, DotosDecodeError> {
        let value = self.parse_integer()?;
        u32::try_from(value).map_err(|_| DotosDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_signed_integer(&self) -> Result<i64, DotosDecodeError> {
        let value = self.parse_numeric_text("SignedInteger")?;
        value
            .parse::<i64>()
            .map_err(|_| DotosDecodeError::InvalidInteger { value })
    }

    pub fn parse_i32(&self) -> Result<i32, DotosDecodeError> {
        let value = self.parse_signed_integer()?;
        i32::try_from(value).map_err(|_| DotosDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_float(&self) -> Result<f64, DotosDecodeError> {
        let value = self.parse_numeric_text("Float")?;
        value
            .parse::<f64>()
            .map_err(|_| DotosDecodeError::InvalidValue {
                type_name: "Float",
                value,
                reason: "expected a finite or non-finite Rust f64 literal".to_owned(),
            })
    }

    /// The flat literal text of a numeric block. A number that carries a
    /// fractional period — `-122.3` — is a dot-application at the raw layer, so
    /// its literal is reconstructed from the dotted segments rather than read
    /// as a single atom; a period-free integer is a bare atom whose text is the
    /// same reconstruction of one segment.
    fn parse_numeric_text(&self, type_name: &'static str) -> Result<String, DotosDecodeError> {
        self.block
            .dotted_text()
            .ok_or(DotosDecodeError::ExpectedAtom { type_name })
    }

    pub fn parse_boolean(&self) -> Result<bool, DotosDecodeError> {
        let value = self
            .block
            .demote_to_string()
            .ok_or(DotosDecodeError::ExpectedAtom {
                type_name: "Boolean",
            })?;
        match value {
            "True" => Ok(true),
            "False" => Ok(false),
            other => Err(DotosDecodeError::UnknownVariant {
                enum_name: "Boolean",
                variant: other.to_owned(),
            }),
        }
    }
}

/// The one canonical DOTOS surface form a string's content takes. The forms are
/// mutually exclusive by construction — [`DotosString::canonical_form`] picks the
/// least-delimited form that carries the content faithfully — so both the
/// encoder and the redundant-delimiter check read a single classification rather
/// than scattering per-character conditionals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringForm {
    BareDotted,
    Parenthesis,
    PipeText,
}

pub struct DotosString<'value> {
    value: &'value str,
}

impl<'value> DotosString<'value> {
    pub fn new(value: &'value str) -> Self {
        Self { value }
    }

    pub fn format(&self) -> String {
        match self.canonical_form() {
            StringForm::BareDotted => self.value.to_owned(),
            StringForm::Parenthesis => format!("({})", self.value),
            StringForm::PipeText => format!("(|{}|)", self.escape_pipe_text()),
        }
    }

    /// The single canonical DOTOS form for this string's content. The three forms
    /// are ordered by how little delimiter they spend, and the first one that can
    /// carry the content faithfully wins, so each form narrows to its honest role:
    ///
    /// - [`StringForm::BareDotted`] — content that is a period-joined chain of
    ///   bare atoms, so the raw parser rebuilds a dot-application whose flat
    ///   dotted text is exactly the content: `schema`, `file.txt`,
    ///   `nix.prometheus.goldragon.criome`. A period is a structural operator at
    ///   the raw layer, but an expected `String` reclaims the split text, so a
    ///   period-bearing string no longer needs an escape.
    /// - [`StringForm::Parenthesis`] — content that is single-space-separated
    ///   words, each itself bare-dotted, so the space-joined `( … )` form
    ///   rebuilds it: `alpha beta`, `version 1.2`.
    /// - [`StringForm::PipeText`] — everything else: delimiter glyphs, newlines
    ///   and indentation, comment markers, pipe-close markers, irregular
    ///   whitespace, or the empty string. Only the literal-preserving
    ///   `( | … | )` form carries these, with close markers escaped.
    fn canonical_form(&self) -> StringForm {
        if self.qualifies_as_bare_dotted_string() {
            StringForm::BareDotted
        } else if self.qualifies_as_parenthesized_string() {
            StringForm::Parenthesis
        } else {
            StringForm::PipeText
        }
    }

    /// Whether the content is a non-empty period-joined chain of bare atoms. Each
    /// period-separated segment must be a non-empty run of bare-atom characters,
    /// so the raw parser rebuilds a dot-application (or a lone atom) whose flat
    /// dotted text equals the content. A comment marker breaks atom scanning, so
    /// content carrying `;;` is never bare.
    fn qualifies_as_bare_dotted_string(&self) -> bool {
        if self.value.is_empty() || self.value.contains(";;") {
            return false;
        }
        self.value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| AtomCharacter::new(character).is_bare_string())
        })
    }

    /// Whether the content is single-space-separated words that each qualify as
    /// bare-dotted. The space-joined `( … )` form skips whitespace adjacent to
    /// object boundaries, so only single ASCII spaces between non-empty words
    /// survive a round trip: a leading or trailing space, a doubled space, or any
    /// non-space whitespace forces the literal-preserving pipe form instead.
    fn qualifies_as_parenthesized_string(&self) -> bool {
        if !self.value.contains(' ') {
            return false;
        }
        if self
            .value
            .chars()
            .any(|character| character.is_whitespace() && character != ' ')
        {
            return false;
        }
        self.value
            .split(' ')
            .all(|word| DotosString::new(word).qualifies_as_bare_dotted_string())
    }

    fn reject_redundant_delimiter(&self, used: StringForm) -> Result<(), DotosDecodeError> {
        if self.canonical_form() != used {
            return Err(DotosDecodeError::NonCanonicalStringDelimiter {
                value: self.value.to_owned(),
                canonical: self.format(),
            });
        }
        Ok(())
    }

    fn escape_pipe_text(&self) -> String {
        let mut escaped = String::new();
        let mut characters = self.value.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\\' {
                escaped.push('\\');
                escaped.push('\\');
            } else if character == '|' && characters.peek() == Some(&')') {
                escaped.push('\\');
                escaped.push('|');
            } else {
                escaped.push(character);
            }
        }
        escaped
    }
}

pub struct DotosCollection<'block> {
    block: &'block Block,
}

impl<'block> DotosCollection<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn parse_vector<Element, Parse>(
        &self,
        parse: Parse,
    ) -> Result<Vec<Element>, DotosDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Element, DotosDecodeError>,
    {
        self.block
            .as_delimited(Delimiter::SquareBracket)
            .ok_or(DotosDecodeError::ExpectedDelimited {
                type_name: "Vec",
                delimiter: Delimiter::SquareBracket.description(),
            })?
            .iter()
            .map(parse)
            .collect()
    }

    pub fn parse_map<Key, Value, ParseKey, ParseValue>(
        &self,
        mut parse_key: ParseKey,
        mut parse_value: ParseValue,
    ) -> Result<BTreeMap<Key, Value>, DotosDecodeError>
    where
        Key: Ord,
        ParseKey: FnMut(&Block) -> Result<Key, DotosDecodeError>,
        ParseValue: FnMut(&Block) -> Result<Value, DotosDecodeError>,
    {
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(DotosDecodeError::ExpectedDelimited {
                    type_name: "Map",
                    delimiter: "Map.( … ) application",
                })?;
        if head.demote_to_string() != Some("Map") {
            return Err(DotosDecodeError::UnknownVariant {
                enum_name: "Map",
                variant: head.dotted_text().unwrap_or_default(),
            });
        }
        let entries = payload.as_delimited(Delimiter::Parenthesis).ok_or(
            DotosDecodeError::ExpectedDelimited {
                type_name: "Map",
                delimiter: Delimiter::Parenthesis.description(),
            },
        )?;
        let mut map = BTreeMap::new();
        for entry_block in entries {
            let entry =
                DottedExpectation::Uncapitalized.read_entry(std::slice::from_ref(entry_block))?;
            let key = parse_key(entry.key())?;
            let value = parse_value(entry.value())?;
            map.insert(key, value);
        }
        Ok(map)
    }

    pub fn parse_option<Inner, Parse>(
        &self,
        mut parse: Parse,
    ) -> Result<Option<Inner>, DotosDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Inner, DotosDecodeError>,
    {
        if self.block.demote_to_string() == Some("None") {
            return Ok(None);
        }
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(DotosDecodeError::ExpectedDelimited {
                    type_name: "Option",
                    delimiter: "None atom or Some.payload application",
                })?;
        let tag = head
            .demote_to_string()
            .ok_or(DotosDecodeError::ExpectedAtom {
                type_name: "Option tag",
            })?;
        if tag != "Some" {
            return Err(DotosDecodeError::UnknownVariant {
                enum_name: "Option",
                variant: tag.to_owned(),
            });
        }
        Ok(Some(parse(payload)?))
    }
}

impl DotosDecode for String {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_string()
    }
}

impl DotosEncode for String {
    fn to_dotos(&self) -> String {
        DotosString::new(self).format()
    }
}

impl DotosDecode for u64 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_integer()
    }
}

impl DotosEncode for u64 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for u8 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_u8()
    }
}

impl DotosEncode for u8 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for u16 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_u16()
    }
}

impl DotosEncode for u16 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for u32 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_u32()
    }
}

impl DotosEncode for u32 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for i32 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_i32()
    }
}

impl DotosEncode for i32 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for i64 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_signed_integer()
    }
}

impl DotosEncode for i64 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for f64 {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_float()
    }
}

impl DotosEncode for f64 {
    fn to_dotos(&self) -> String {
        self.to_string()
    }
}

impl DotosDecode for bool {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosBlock::new(block).parse_boolean()
    }
}

impl DotosEncode for bool {
    fn to_dotos(&self) -> String {
        if *self {
            "True".to_owned()
        } else {
            "False".to_owned()
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct ByteSequence(Vec<u8>);

impl ByteSequence {
    pub fn new(payload: Vec<u8>) -> Self {
        Self(payload)
    }

    pub fn payload(&self) -> &[u8] {
        &self.0
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.0
    }

    pub fn from_hex(text: &str) -> Result<Self, DotosDecodeError> {
        if text.len() % 2 != 0 {
            return Err(DotosDecodeError::Parse(format!(
                "byte sequence hex literal has odd length: {text}"
            )));
        }
        let mut bytes = Vec::with_capacity(text.len() / 2);
        for pair in text.as_bytes().chunks_exact(2) {
            let high = Self::hex_digit(pair[0])?;
            let low = Self::hex_digit(pair[1])?;
            bytes.push((high << 4) | low);
        }
        Ok(Self(bytes))
    }

    fn hex_digit(digit: u8) -> Result<u8, DotosDecodeError> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            other => Err(DotosDecodeError::Parse(format!(
                "byte sequence hex literal has a non-hex digit: {other}"
            ))),
        }
    }
}

impl From<Vec<u8>> for ByteSequence {
    fn from(payload: Vec<u8>) -> Self {
        Self::new(payload)
    }
}

impl DotosDecode for ByteSequence {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        let hex = String::from_dotos_block(block)?;
        Self::from_hex(&hex)
    }
}

impl DotosEncode for ByteSequence {
    fn to_dotos(&self) -> String {
        let mut hex = String::with_capacity(self.0.len() * 2);
        for byte in &self.0 {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex.to_dotos()
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct FixedByteSequence<const WIDTH: usize>([u8; WIDTH]);

impl<const WIDTH: usize> FixedByteSequence<WIDTH> {
    pub fn new(payload: [u8; WIDTH]) -> Self {
        Self(payload)
    }

    pub fn payload(&self) -> &[u8; WIDTH] {
        &self.0
    }

    pub fn into_payload(self) -> [u8; WIDTH] {
        self.0
    }

    pub fn from_hex(text: &str) -> Result<Self, DotosDecodeError> {
        if text.len() != WIDTH * 2 {
            return Err(DotosDecodeError::Parse(format!(
                "fixed byte sequence expected {} hex digits, found {}",
                WIDTH * 2,
                text.len()
            )));
        }
        let mut bytes = [0u8; WIDTH];
        for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (Self::hex_digit(pair[0])? << 4) | Self::hex_digit(pair[1])?;
        }
        Ok(Self(bytes))
    }

    fn hex_digit(digit: u8) -> Result<u8, DotosDecodeError> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            other => Err(DotosDecodeError::Parse(format!(
                "fixed byte sequence hex literal has a non-hex digit: {other}"
            ))),
        }
    }
}

impl<const WIDTH: usize> From<[u8; WIDTH]> for FixedByteSequence<WIDTH> {
    fn from(payload: [u8; WIDTH]) -> Self {
        Self::new(payload)
    }
}

impl<const WIDTH: usize> DotosDecode for FixedByteSequence<WIDTH> {
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        let hex = String::from_dotos_block(block)?;
        Self::from_hex(&hex)
    }
}

impl<const WIDTH: usize> DotosEncode for FixedByteSequence<WIDTH> {
    fn to_dotos(&self) -> String {
        let mut hex = String::with_capacity(WIDTH * 2);
        for byte in &self.0 {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex.to_dotos()
    }
}

impl<Element> DotosDecode for Vec<Element>
where
    Element: DotosDecode,
{
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosCollection::new(block).parse_vector(Element::from_dotos_block)
    }
}

impl<Element> DotosEncode for Vec<Element>
where
    Element: DotosEncode,
{
    fn to_dotos(&self) -> String {
        Delimiter::SquareBracket.wrap(self.iter().map(Element::to_dotos))
    }
}

impl<Element> DotosBodyDecode for Vec<Element>
where
    Element: DotosDecode,
{
    fn from_dotos_body(body: &DotosBody<'_>) -> Result<Self, DotosDecodeError> {
        body.root_objects()
            .iter()
            .map(Element::from_dotos_block)
            .collect()
    }
}

impl<Element> DotosBodyEncode for Vec<Element>
where
    Element: DotosEncode,
{
    fn to_dotos_body(&self) -> DotosBodyEncoding {
        DotosBodyEncoding::new(self.iter().map(Element::to_dotos).collect())
    }
}

impl<Key, Value> DotosDecode for BTreeMap<Key, Value>
where
    Key: DotosDecode + Ord,
    Value: DotosDecode,
{
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosCollection::new(block).parse_map(Key::from_dotos_block, Value::from_dotos_block)
    }
}

impl<Key, Value> DotosEncode for BTreeMap<Key, Value>
where
    Key: DotosEncode,
    Value: DotosEncode,
{
    fn to_dotos(&self) -> String {
        let mut entries: Vec<String> = Vec::new();
        for (key, value) in self {
            entries.push(format!("{}.{}", Key::to_dotos(key), Value::to_dotos(value)));
        }
        format!("Map.{}", Delimiter::Parenthesis.wrap(entries))
    }
}

impl<Inner> DotosDecode for Option<Inner>
where
    Inner: DotosDecode,
{
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        DotosCollection::new(block).parse_option(Inner::from_dotos_block)
    }
}

impl<Inner> DotosEncode for Option<Inner>
where
    Inner: DotosEncode,
{
    fn to_dotos(&self) -> String {
        match self {
            Some(inner) => format!("Some.{}", Inner::to_dotos(inner)),
            None => "None".to_owned(),
        }
    }
}

impl<Inner> DotosDecode for Box<Inner>
where
    Inner: DotosDecode,
{
    fn from_dotos_block(block: &Block) -> Result<Self, DotosDecodeError> {
        Inner::from_dotos_block(block).map(Box::new)
    }
}

impl<Inner> DotosEncode for Box<Inner>
where
    Inner: DotosEncode,
{
    fn to_dotos(&self) -> String {
        Inner::to_dotos(self)
    }
}
