use std::collections::BTreeMap;
use std::fmt;

use crate::{Block, Delimiter, Document, expectation::DottedExpectation, parser::AtomCharacter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotaDecodeError {
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

impl fmt::Display for NotaDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::ExpectedSingleRoot { found } => {
                write!(
                    formatter,
                    "expected exactly one NOTA root object, found {found}"
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

impl std::error::Error for NotaDecodeError {}

impl From<crate::NotaError> for NotaDecodeError {
    fn from(error: crate::NotaError) -> Self {
        Self::Parse(error.to_string())
    }
}

pub trait NotaDecode: Sized {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError>;
}

pub trait NotaEncode {
    fn to_nota(&self) -> String;
}

pub trait NotaBodyDecode: Sized {
    fn from_nota_body(body: &NotaBody<'_>) -> Result<Self, NotaDecodeError>;
}

pub trait NotaBodyEncode {
    fn to_nota_body(&self) -> NotaBodyEncoding;
}

pub trait NotaDocumentDecode: Sized {
    fn from_nota_document_body(body: &NotaDocumentBody<'_>) -> Result<Self, NotaDecodeError>;
}

pub trait NotaDocumentEncode {
    fn to_nota_document_body(&self) -> NotaDocumentEncoding;
}

pub trait NotaNamedDocumentFieldDecode: Sized {
    fn from_nota_named_document_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, NotaDecodeError>;
}

pub trait NotaNamedDocumentFieldEncode {
    fn to_nota_named_document_field_body(&self) -> String;
}

pub trait NotaNamedBodyFieldDecode: Sized {
    fn from_nota_named_body_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, NotaDecodeError>;
}

pub trait NotaNamedBodyFieldEncode {
    fn to_nota_named_body_field(&self) -> String;
}

impl<Value> NotaNamedBodyFieldDecode for Value
where
    Value: NotaNamedDocumentFieldDecode,
{
    fn from_nota_named_body_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, NotaDecodeError> {
        Value::from_nota_named_document_field(name, block)
    }
}

impl<Value> NotaNamedBodyFieldEncode for Value
where
    Value: NotaNamedDocumentFieldEncode,
{
    fn to_nota_named_body_field(&self) -> String {
        self.to_nota_named_document_field_body()
    }
}

pub struct NotaSource<'source> {
    source: &'source str,
}

impl<'source> NotaSource<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    pub fn parse_root(&self) -> Result<Block, NotaDecodeError> {
        let document = Document::parse(self.source)?;
        if document.holds_root_objects() != 1 {
            return Err(NotaDecodeError::ExpectedSingleRoot {
                found: document.holds_root_objects(),
            });
        }
        Ok(document
            .root_object_at(0)
            .expect("root count checked")
            .clone())
    }

    pub fn parse<Value>(&self) -> Result<Value, NotaDecodeError>
    where
        Value: NotaDecode,
    {
        let root = self.parse_root()?;
        Value::from_nota_block(&root)
    }

    pub fn parse_document_body<Value>(&self) -> Result<Value, NotaDecodeError>
    where
        Value: NotaDocumentDecode,
    {
        let document = Document::parse(self.source)?;
        let body = NotaDocumentBody::new(&document);
        Value::from_nota_document_body(&body)
    }

    pub fn parse_body<Value>(&self) -> Result<Value, NotaDecodeError>
    where
        Value: NotaBodyDecode,
    {
        let document = Document::parse(self.source)?;
        let body = NotaBody::from_document(&document);
        Value::from_nota_body(&body)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NotaBody<'body> {
    root_objects: &'body [Block],
}

impl<'body> NotaBody<'body> {
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
    ) -> Result<Self, NotaDecodeError> {
        let root_objects =
            block
                .as_delimited(delimiter)
                .ok_or(NotaDecodeError::ExpectedDelimited {
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
    ) -> Result<&'body [Block], NotaDecodeError> {
        let found = self.root_objects().len();
        if found != expected {
            return Err(NotaDecodeError::ExpectedRootCount {
                type_name,
                expected,
                found,
            });
        }
        Ok(self.root_objects())
    }
}

pub struct NotaDocumentBody<'document> {
    body: NotaBody<'document>,
}

impl<'document> NotaDocumentBody<'document> {
    pub fn new(document: &'document Document) -> Self {
        Self {
            body: NotaBody::from_document(document),
        }
    }

    pub fn from_body(body: NotaBody<'document>) -> Self {
        Self { body }
    }

    pub fn as_body(&self) -> &NotaBody<'document> {
        &self.body
    }

    pub fn root_objects(&self) -> &'document [Block] {
        self.body.root_objects()
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'document [Block], NotaDecodeError> {
        self.body.expect_fields(type_name, expected)
    }
}

pub struct NotaBodyEncoding {
    fields: Vec<String>,
}

impl NotaBodyEncoding {
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    pub fn to_nota(&self) -> String {
        self.fields.join("\n")
    }

    pub fn to_delimited_nota(&self, delimiter: Delimiter) -> String {
        delimiter.wrap(self.fields.iter().cloned())
    }
}

pub type NotaDocumentEncoding = NotaBodyEncoding;

pub struct NotaBlock<'block> {
    block: &'block Block,
}

impl<'block> NotaBlock<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn expect_children(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'block [Block], NotaDecodeError> {
        self.expect_body(delimiter, type_name)?
            .expect_fields(type_name, expected)
    }

    pub fn expect_delimited(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<&'block [Block], NotaDecodeError> {
        Ok(self.expect_body(delimiter, type_name)?.root_objects())
    }

    pub fn expect_body(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<NotaBody<'block>, NotaDecodeError> {
        NotaBody::from_delimited(self.block, delimiter, type_name)
    }

    pub fn parse_string(&self) -> Result<String, NotaDecodeError> {
        // Pipe text carries a literal, but a pipe wrapper around content that a
        // simpler canonical form could hold is non-canonical and rejected.
        if self.block.is_pipe_text() {
            let text = self
                .block
                .demote_to_string()
                .expect("pipe text demotes to its literal");
            NotaString::new(text).reject_redundant_delimiter(StringForm::PipeText)?;
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
                .map(|block| NotaBlock::new(block).parse_string())
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join(" "))?;
            NotaString::new(&text).reject_redundant_delimiter(StringForm::Parenthesis)?;
            return Ok(text);
        }
        Err(NotaDecodeError::ExpectedDelimited {
            type_name: "String",
            delimiter: "string atom, dotted application, or parenthesis",
        })
    }

    pub fn parse_integer(&self) -> Result<u64, NotaDecodeError> {
        let value = self.parse_numeric_text("Integer")?;
        value
            .parse::<u64>()
            .map_err(|_| NotaDecodeError::InvalidInteger { value })
    }

    pub fn parse_u16(&self) -> Result<u16, NotaDecodeError> {
        let value = self.parse_integer()?;
        u16::try_from(value).map_err(|_| NotaDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u8(&self) -> Result<u8, NotaDecodeError> {
        let value = self.parse_integer()?;
        u8::try_from(value).map_err(|_| NotaDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u32(&self) -> Result<u32, NotaDecodeError> {
        let value = self.parse_integer()?;
        u32::try_from(value).map_err(|_| NotaDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_signed_integer(&self) -> Result<i64, NotaDecodeError> {
        let value = self.parse_numeric_text("SignedInteger")?;
        value
            .parse::<i64>()
            .map_err(|_| NotaDecodeError::InvalidInteger { value })
    }

    pub fn parse_i32(&self) -> Result<i32, NotaDecodeError> {
        let value = self.parse_signed_integer()?;
        i32::try_from(value).map_err(|_| NotaDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_float(&self) -> Result<f64, NotaDecodeError> {
        let value = self.parse_numeric_text("Float")?;
        value
            .parse::<f64>()
            .map_err(|_| NotaDecodeError::InvalidValue {
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
    fn parse_numeric_text(&self, type_name: &'static str) -> Result<String, NotaDecodeError> {
        self.block
            .dotted_text()
            .ok_or(NotaDecodeError::ExpectedAtom { type_name })
    }

    pub fn parse_boolean(&self) -> Result<bool, NotaDecodeError> {
        let value = self
            .block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom {
                type_name: "Boolean",
            })?;
        match value {
            "True" => Ok(true),
            "False" => Ok(false),
            other => Err(NotaDecodeError::UnknownVariant {
                enum_name: "Boolean",
                variant: other.to_owned(),
            }),
        }
    }
}

/// The one canonical NOTA surface form a string's content takes. The forms are
/// mutually exclusive by construction — [`NotaString::canonical_form`] picks the
/// least-delimited form that carries the content faithfully — so both the
/// encoder and the redundant-delimiter check read a single classification rather
/// than scattering per-character conditionals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringForm {
    BareDotted,
    Parenthesis,
    PipeText,
}

pub struct NotaString<'value> {
    value: &'value str,
}

impl<'value> NotaString<'value> {
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

    /// The single canonical NOTA form for this string's content. The three forms
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
            .all(|word| NotaString::new(word).qualifies_as_bare_dotted_string())
    }

    fn reject_redundant_delimiter(&self, used: StringForm) -> Result<(), NotaDecodeError> {
        if self.canonical_form() != used {
            return Err(NotaDecodeError::NonCanonicalStringDelimiter {
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

pub struct NotaCollection<'block> {
    block: &'block Block,
}

impl<'block> NotaCollection<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn parse_vector<Element, Parse>(
        &self,
        parse: Parse,
    ) -> Result<Vec<Element>, NotaDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Element, NotaDecodeError>,
    {
        self.block
            .as_delimited(Delimiter::SquareBracket)
            .ok_or(NotaDecodeError::ExpectedDelimited {
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
    ) -> Result<BTreeMap<Key, Value>, NotaDecodeError>
    where
        Key: Ord,
        ParseKey: FnMut(&Block) -> Result<Key, NotaDecodeError>,
        ParseValue: FnMut(&Block) -> Result<Value, NotaDecodeError>,
    {
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(NotaDecodeError::ExpectedDelimited {
                    type_name: "Map",
                    delimiter: "Map.( … ) application",
                })?;
        if head.demote_to_string() != Some("Map") {
            return Err(NotaDecodeError::UnknownVariant {
                enum_name: "Map",
                variant: head.dotted_text().unwrap_or_default(),
            });
        }
        let entries = payload.as_delimited(Delimiter::Parenthesis).ok_or(
            NotaDecodeError::ExpectedDelimited {
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
    ) -> Result<Option<Inner>, NotaDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Inner, NotaDecodeError>,
    {
        if self.block.demote_to_string() == Some("None") {
            return Ok(None);
        }
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(NotaDecodeError::ExpectedDelimited {
                    type_name: "Option",
                    delimiter: "None atom or Some.payload application",
                })?;
        let tag = head
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom {
                type_name: "Option tag",
            })?;
        if tag != "Some" {
            return Err(NotaDecodeError::UnknownVariant {
                enum_name: "Option",
                variant: tag.to_owned(),
            });
        }
        Ok(Some(parse(payload)?))
    }
}

impl NotaDecode for String {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_string()
    }
}

impl NotaEncode for String {
    fn to_nota(&self) -> String {
        NotaString::new(self).format()
    }
}

impl NotaDecode for u64 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_integer()
    }
}

impl NotaEncode for u64 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for u8 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_u8()
    }
}

impl NotaEncode for u8 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for u16 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_u16()
    }
}

impl NotaEncode for u16 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for u32 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_u32()
    }
}

impl NotaEncode for u32 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for i32 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_i32()
    }
}

impl NotaEncode for i32 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for i64 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_signed_integer()
    }
}

impl NotaEncode for i64 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for f64 {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_float()
    }
}

impl NotaEncode for f64 {
    fn to_nota(&self) -> String {
        self.to_string()
    }
}

impl NotaDecode for bool {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaBlock::new(block).parse_boolean()
    }
}

impl NotaEncode for bool {
    fn to_nota(&self) -> String {
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

    pub fn from_hex(text: &str) -> Result<Self, NotaDecodeError> {
        if text.len() % 2 != 0 {
            return Err(NotaDecodeError::Parse(format!(
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

    fn hex_digit(digit: u8) -> Result<u8, NotaDecodeError> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            other => Err(NotaDecodeError::Parse(format!(
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

impl NotaDecode for ByteSequence {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let hex = String::from_nota_block(block)?;
        Self::from_hex(&hex)
    }
}

impl NotaEncode for ByteSequence {
    fn to_nota(&self) -> String {
        let mut hex = String::with_capacity(self.0.len() * 2);
        for byte in &self.0 {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex.to_nota()
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

    pub fn from_hex(text: &str) -> Result<Self, NotaDecodeError> {
        if text.len() != WIDTH * 2 {
            return Err(NotaDecodeError::Parse(format!(
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

    fn hex_digit(digit: u8) -> Result<u8, NotaDecodeError> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            other => Err(NotaDecodeError::Parse(format!(
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

impl<const WIDTH: usize> NotaDecode for FixedByteSequence<WIDTH> {
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        let hex = String::from_nota_block(block)?;
        Self::from_hex(&hex)
    }
}

impl<const WIDTH: usize> NotaEncode for FixedByteSequence<WIDTH> {
    fn to_nota(&self) -> String {
        let mut hex = String::with_capacity(WIDTH * 2);
        for byte in &self.0 {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex.to_nota()
    }
}

impl<Element> NotaDecode for Vec<Element>
where
    Element: NotaDecode,
{
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaCollection::new(block).parse_vector(Element::from_nota_block)
    }
}

impl<Element> NotaEncode for Vec<Element>
where
    Element: NotaEncode,
{
    fn to_nota(&self) -> String {
        Delimiter::SquareBracket.wrap(self.iter().map(Element::to_nota))
    }
}

impl<Element> NotaBodyDecode for Vec<Element>
where
    Element: NotaDecode,
{
    fn from_nota_body(body: &NotaBody<'_>) -> Result<Self, NotaDecodeError> {
        body.root_objects()
            .iter()
            .map(Element::from_nota_block)
            .collect()
    }
}

impl<Element> NotaBodyEncode for Vec<Element>
where
    Element: NotaEncode,
{
    fn to_nota_body(&self) -> NotaBodyEncoding {
        NotaBodyEncoding::new(self.iter().map(Element::to_nota).collect())
    }
}

impl<Key, Value> NotaDecode for BTreeMap<Key, Value>
where
    Key: NotaDecode + Ord,
    Value: NotaDecode,
{
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaCollection::new(block).parse_map(Key::from_nota_block, Value::from_nota_block)
    }
}

impl<Key, Value> NotaEncode for BTreeMap<Key, Value>
where
    Key: NotaEncode,
    Value: NotaEncode,
{
    fn to_nota(&self) -> String {
        let mut entries: Vec<String> = Vec::new();
        for (key, value) in self {
            entries.push(format!("{}.{}", Key::to_nota(key), Value::to_nota(value)));
        }
        format!("Map.{}", Delimiter::Parenthesis.wrap(entries))
    }
}

impl<Inner> NotaDecode for Option<Inner>
where
    Inner: NotaDecode,
{
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        NotaCollection::new(block).parse_option(Inner::from_nota_block)
    }
}

impl<Inner> NotaEncode for Option<Inner>
where
    Inner: NotaEncode,
{
    fn to_nota(&self) -> String {
        match self {
            Some(inner) => format!("Some.{}", Inner::to_nota(inner)),
            None => "None".to_owned(),
        }
    }
}

impl<Inner> NotaDecode for Box<Inner>
where
    Inner: NotaDecode,
{
    fn from_nota_block(block: &Block) -> Result<Self, NotaDecodeError> {
        Inner::from_nota_block(block).map(Box::new)
    }
}

impl<Inner> NotaEncode for Box<Inner>
where
    Inner: NotaEncode,
{
    fn to_nota(&self) -> String {
        Inner::to_nota(self)
    }
}
