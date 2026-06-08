use std::collections::BTreeMap;
use std::fmt;

use crate::{Block, Delimiter, Document};

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
    InvalidInteger {
        value: String,
    },
    InvalidValue {
        type_name: &'static str,
        value: String,
        reason: String,
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
            Self::InvalidInteger { value } => write!(formatter, "invalid integer {value}"),
            Self::InvalidValue {
                type_name,
                value,
                reason,
            } => write!(formatter, "invalid {type_name} {value:?}: {reason}"),
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
        if let Some(text) = self.block.demote_to_string() {
            return Ok(text.to_owned());
        }
        if let Some(root_objects) = self.block.as_delimited(Delimiter::SquareBracket) {
            return root_objects
                .iter()
                .map(|block| NotaBlock::new(block).parse_string())
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join(" "));
        }
        Err(NotaDecodeError::ExpectedDelimited {
            type_name: "String",
            delimiter: "string atom or square bracket",
        })
    }

    pub fn parse_integer(&self) -> Result<u64, NotaDecodeError> {
        let value = self.parse_integer_text("Integer")?;
        value
            .parse::<u64>()
            .map_err(|_| NotaDecodeError::InvalidInteger {
                value: value.to_owned(),
            })
    }

    pub fn parse_u16(&self) -> Result<u16, NotaDecodeError> {
        let value = self.parse_integer()?;
        u16::try_from(value).map_err(|_| NotaDecodeError::InvalidInteger {
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
        let value = self.parse_integer_text("SignedInteger")?;
        value
            .parse::<i64>()
            .map_err(|_| NotaDecodeError::InvalidInteger {
                value: value.to_owned(),
            })
    }

    fn parse_integer_text(&self, type_name: &'static str) -> Result<&'block str, NotaDecodeError> {
        let value = self
            .block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom { type_name })?;
        Ok(value)
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

pub struct NotaString<'value> {
    value: &'value str,
}

impl<'value> NotaString<'value> {
    pub fn new(value: &'value str) -> Self {
        Self { value }
    }

    pub fn format(&self) -> String {
        if self.value.contains("|]") {
            format!("[{}]", self.value.replace(']', " ]"))
        } else if self
            .value
            .chars()
            .any(|character| matches!(character, '[' | ']' | '(' | ')' | '{' | '}' | ';' | '\n'))
        {
            format!("[|{}|]", self.value)
        } else {
            format!("[{}]", self.value)
        }
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
        let root_objects = self.block.as_delimited(Delimiter::Brace).ok_or(
            NotaDecodeError::ExpectedDelimited {
                type_name: "BTreeMap",
                delimiter: Delimiter::Brace.description(),
            },
        )?;
        if root_objects.len() % 2 != 0 {
            return Err(NotaDecodeError::ExpectedRootCount {
                type_name: "BTreeMap",
                expected: root_objects.len() + 1,
                found: root_objects.len(),
            });
        }
        let mut map = BTreeMap::new();
        let mut index = 0;
        while index < root_objects.len() {
            let key = parse_key(&root_objects[index])?;
            let value = parse_value(&root_objects[index + 1])?;
            map.insert(key, value);
            index += 2;
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
        let children =
            NotaBlock::new(self.block).expect_children(Delimiter::Parenthesis, "Option", 2)?;
        let tag = children[0]
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
        Ok(Some(parse(&children[1])?))
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
        let mut parts: Vec<String> = Vec::new();
        for (key, value) in self {
            parts.push(Key::to_nota(key));
            parts.push(Value::to_nota(value));
        }
        Delimiter::Brace.wrap(parts)
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
            Some(inner) => Delimiter::Parenthesis.wrap(["Some".to_owned(), Inner::to_nota(inner)]),
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
