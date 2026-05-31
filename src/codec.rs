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
}

pub struct NotaDocumentBody<'document> {
    document: &'document Document,
}

impl<'document> NotaDocumentBody<'document> {
    pub fn new(document: &'document Document) -> Self {
        Self { document }
    }

    pub fn root_objects(&self) -> &'document [Block] {
        self.document.root_objects()
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'document [Block], NotaDecodeError> {
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

pub struct NotaDocumentEncoding {
    fields: Vec<String>,
}

impl NotaDocumentEncoding {
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    pub fn to_nota(&self) -> String {
        self.fields.join("\n")
    }
}

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
        delimiter_name: &'static str,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'block [Block], NotaDecodeError> {
        match self.block {
            Block::Delimited {
                delimiter: found,
                root_objects,
                ..
            } if *found == delimiter => {
                if root_objects.len() != expected {
                    return Err(NotaDecodeError::ExpectedRootCount {
                        type_name,
                        expected,
                        found: root_objects.len(),
                    });
                }
                Ok(root_objects)
            }
            _ => Err(NotaDecodeError::ExpectedDelimited {
                type_name,
                delimiter: delimiter_name,
            }),
        }
    }

    pub fn parse_string(&self) -> Result<String, NotaDecodeError> {
        if let Some(text) = self.block.demote_to_string() {
            return Ok(text.to_owned());
        }
        match self.block {
            Block::Delimited {
                delimiter: Delimiter::SquareBracket,
                root_objects,
                ..
            } => root_objects
                .iter()
                .map(|block| NotaBlock::new(block).parse_string())
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join(" ")),
            _ => Err(NotaDecodeError::ExpectedDelimited {
                type_name: "String",
                delimiter: "string atom or square bracket",
            }),
        }
    }

    pub fn parse_integer(&self) -> Result<u64, NotaDecodeError> {
        let value = self
            .block
            .demote_to_string()
            .ok_or(NotaDecodeError::ExpectedAtom {
                type_name: "Integer",
            })?;
        value
            .parse::<u64>()
            .map_err(|_| NotaDecodeError::InvalidInteger {
                value: value.to_owned(),
            })
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
        match self.block {
            Block::Delimited {
                delimiter: Delimiter::SquareBracket,
                root_objects,
                ..
            } => root_objects.iter().map(parse).collect(),
            _ => Err(NotaDecodeError::ExpectedDelimited {
                type_name: "Vec",
                delimiter: "square bracket",
            }),
        }
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
        match self.block {
            Block::Delimited {
                delimiter: Delimiter::Brace,
                root_objects,
                ..
            } => {
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
            _ => Err(NotaDecodeError::ExpectedDelimited {
                type_name: "BTreeMap",
                delimiter: "brace",
            }),
        }
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
        let children = NotaBlock::new(self.block).expect_children(
            Delimiter::Parenthesis,
            "parenthesis",
            "Option",
            2,
        )?;
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

    pub fn format_vector<Element, Format>(elements: &[Element], format: Format) -> String
    where
        Format: FnMut(&Element) -> String,
    {
        let parts: Vec<String> = elements.iter().map(format).collect();
        format!("[{}]", parts.join(" "))
    }

    pub fn format_map<Key, Value, FormatKey, FormatValue>(
        map: &BTreeMap<Key, Value>,
        mut format_key: FormatKey,
        mut format_value: FormatValue,
    ) -> String
    where
        FormatKey: FnMut(&Key) -> String,
        FormatValue: FnMut(&Value) -> String,
    {
        let mut parts: Vec<String> = Vec::new();
        for (key, value) in map {
            parts.push(format_key(key));
            parts.push(format_value(value));
        }
        format!("{{{}}}", parts.join(" "))
    }

    pub fn format_option<Inner, Format>(value: &Option<Inner>, mut format: Format) -> String
    where
        Format: FnMut(&Inner) -> String,
    {
        match value {
            Some(inner) => format!("(Some {})", format(inner)),
            None => "None".to_owned(),
        }
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
        NotaCollection::format_vector(self, Element::to_nota)
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
        NotaCollection::format_map(self, Key::to_nota, Value::to_nota)
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
        NotaCollection::format_option(self, Inner::to_nota)
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
