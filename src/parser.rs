use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePosition {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    source: String,
    root_objects: Vec<Block>,
}

impl Document {
    pub fn parse(source: impl Into<String>) -> Result<Self, NotaError> {
        let source = source.into();
        let mut parser = Parser::new(&source);
        let root_objects = parser.parse_document()?;
        Ok(Self {
            source,
            root_objects,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn root_objects(&self) -> &[Block] {
        &self.root_objects
    }

    pub fn holds_root_objects(&self) -> usize {
        self.root_objects.len()
    }

    pub fn root_object_at(&self, index: usize) -> Option<&Block> {
        self.root_objects.get(index)
    }

    pub fn structure_header(&self) -> StructureHeader {
        let mut builder = StructureHeaderBuilder::new();
        builder.push_document(self);
        builder.finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    Delimited {
        delimiter: Delimiter,
        span: SourceSpan,
        root_objects: Vec<Block>,
    },
    PipeText(PipeText),
    Atom(Atom),
}

impl Block {
    pub fn source_span(&self) -> SourceSpan {
        match self {
            Self::Delimited { span, .. } => *span,
            Self::PipeText(pipe_text) => pipe_text.span,
            Self::Atom(atom) => atom.span,
        }
    }

    pub fn reemit<'source>(&self, source: &'source str) -> &'source str {
        let span = self.source_span();
        &source[span.start.byte_offset..span.end.byte_offset]
    }

    pub fn is_parenthesis(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::Parenthesis,
                ..
            }
        )
    }

    pub fn is_square_bracket(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::SquareBracket,
                ..
            }
        )
    }

    pub fn is_brace(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::Brace,
                ..
            }
        )
    }

    pub fn is_pipe_parenthesis(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::PipeParenthesis,
                ..
            }
        )
    }

    pub fn is_pipe_brace(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::PipeBrace,
                ..
            }
        )
    }

    pub fn is_pipe_text(&self) -> bool {
        matches!(self, Self::PipeText(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self, Self::Atom(_))
    }

    pub fn is_delimited_with(&self, delimiter: Delimiter) -> bool {
        matches!(self, Self::Delimited { delimiter: found, .. } if *found == delimiter)
    }

    pub fn as_delimited(&self, delimiter: Delimiter) -> Option<&[Block]> {
        match self {
            Self::Delimited {
                delimiter: found,
                root_objects,
                ..
            } if *found == delimiter => Some(root_objects),
            Self::Delimited { .. } | Self::PipeText(_) | Self::Atom(_) => None,
        }
    }

    pub fn holds_root_objects(&self) -> usize {
        match self {
            Self::Delimited { root_objects, .. } => root_objects.len(),
            Self::PipeText(_) | Self::Atom(_) => 0,
        }
    }

    pub fn holds_single_root_object(&self) -> bool {
        self.holds_root_objects() == 1
    }

    pub fn holds_two_root_objects(&self) -> bool {
        self.holds_root_objects() == 2
    }

    pub fn root_object_at(&self, index: usize) -> Option<&Block> {
        match self {
            Self::Delimited { root_objects, .. } => root_objects.get(index),
            Self::PipeText(_) | Self::Atom(_) => None,
        }
    }

    pub fn root_objects(&self) -> &[Block] {
        match self {
            Self::Delimited { root_objects, .. } => root_objects,
            Self::PipeText(_) | Self::Atom(_) => &[],
        }
    }

    pub fn atom(&self) -> Option<&Atom> {
        match self {
            Self::Atom(atom) => Some(atom),
            Self::Delimited { .. } | Self::PipeText(_) => None,
        }
    }

    pub fn qualifies_as_symbol(&self) -> bool {
        self.atom().is_some_and(Atom::qualifies_as_symbol)
    }

    pub fn qualifies_as_pascal_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_pascal_case_symbol)
    }

    pub fn qualifies_as_camel_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_camel_case_symbol)
    }

    pub fn qualifies_as_kebab_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_kebab_case_symbol)
    }

    pub fn demote_to_string(&self) -> Option<&str> {
        match self {
            Self::Atom(atom) => Some(atom.text.as_str()),
            Self::PipeText(pipe_text) => Some(pipe_text.text.as_str()),
            Self::Delimited { .. } => None,
        }
    }

    pub fn structure_shape(&self) -> StructureShape {
        match self {
            Self::Delimited {
                delimiter: Delimiter::Parenthesis,
                ..
            } => StructureShape::Parenthesis,
            Self::Delimited {
                delimiter: Delimiter::SquareBracket,
                ..
            } => StructureShape::SquareBracket,
            Self::Delimited {
                delimiter: Delimiter::Brace,
                ..
            } => StructureShape::Brace,
            Self::Delimited {
                delimiter: Delimiter::PipeParenthesis,
                ..
            } => StructureShape::PipeParenthesis,
            Self::Delimited {
                delimiter: Delimiter::PipeBrace,
                ..
            } => StructureShape::PipeBrace,
            Self::PipeText(_) => StructureShape::PipeText,
            Self::Atom(_) => StructureShape::Atom,
        }
    }

    pub fn structure_slot(&self) -> StructureSlot {
        StructureSlot::new(self.structure_shape(), self.holds_root_objects())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delimiter {
    Parenthesis,
    SquareBracket,
    Brace,
    PipeParenthesis,
    PipeBrace,
}

impl Delimiter {
    pub fn opening_text(self) -> &'static str {
        match self {
            Self::Parenthesis => "(",
            Self::SquareBracket => "[",
            Self::Brace => "{",
            Self::PipeParenthesis => "(|",
            Self::PipeBrace => "{|",
        }
    }

    pub fn closing_text(self) -> &'static str {
        match self {
            Self::Parenthesis => ")",
            Self::SquareBracket => "]",
            Self::Brace => "}",
            Self::PipeParenthesis => "|)",
            Self::PipeBrace => "|}",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::PipeParenthesis => "pipe parenthesis",
            Self::PipeBrace => "pipe brace",
        }
    }

    pub fn wrap(self, children: impl IntoIterator<Item = String>) -> String {
        let children = children.into_iter().collect::<Vec<_>>();
        format!(
            "{}{}{}",
            self.opening_text(),
            children.join(" "),
            self.closing_text()
        )
    }

    fn closing(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::SquareBracket => ']',
            Self::Brace => '}',
            Self::PipeParenthesis => ')',
            Self::PipeBrace => '}',
        }
    }

    fn from_opening(character: char) -> Option<Self> {
        match character {
            '(' => Some(Self::Parenthesis),
            '[' => Some(Self::SquareBracket),
            '{' => Some(Self::Brace),
            _ => None,
        }
    }

    fn from_closing(character: char) -> Option<Self> {
        match character {
            ')' => Some(Self::Parenthesis),
            ']' => Some(Self::SquareBracket),
            '}' => Some(Self::Brace),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureHeader {
    slots: Vec<StructureSlot>,
}

impl StructureHeader {
    pub const MAXIMUM_SLOTS: usize = 8;

    pub fn slots(&self) -> &[StructureSlot] {
        &self.slots
    }

    pub fn packed_word(&self) -> u64 {
        let mut word = 0_u64;
        for (index, slot) in self.slots.iter().enumerate().take(Self::MAXIMUM_SLOTS) {
            word |= u64::from(slot.packed_byte()) << (index * 8);
        }
        word
    }

    pub fn from_packed_word(word: u64) -> Self {
        let mut slots = Vec::new();
        for index in 0..Self::MAXIMUM_SLOTS {
            let byte = ((word >> (index * 8)) & 0xff) as u8;
            if byte == 0 && index > 0 {
                break;
            }
            slots.push(StructureSlot::from_packed_byte(byte));
        }
        Self { slots }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructureSlot {
    shape: StructureShape,
    child_count: u8,
}

impl StructureSlot {
    pub fn new(shape: StructureShape, child_count: usize) -> Self {
        if child_count > Self::MAXIMUM_CHILD_COUNT {
            return Self::overflow();
        }
        Self {
            shape,
            child_count: child_count as u8,
        }
    }

    const MAXIMUM_CHILD_COUNT: usize = 15;

    pub fn overflow() -> Self {
        Self {
            shape: StructureShape::Unknown,
            child_count: Self::MAXIMUM_CHILD_COUNT as u8,
        }
    }

    pub fn shape(&self) -> StructureShape {
        self.shape
    }

    pub fn child_count(&self) -> u8 {
        self.child_count
    }

    pub fn packed_byte(&self) -> u8 {
        (self.shape.code() << 4) | self.child_count
    }

    pub fn from_packed_byte(byte: u8) -> Self {
        Self {
            shape: StructureShape::from_code(byte >> 4),
            child_count: byte & 0x0f,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructureShape {
    Document,
    Atom,
    Parenthesis,
    SquareBracket,
    Brace,
    PipeText,
    PipeParenthesis,
    PipeBrace,
    Unknown,
}

impl StructureShape {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Atom => "atom",
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::PipeText => "pipe text",
            Self::PipeParenthesis => "pipe parenthesis",
            Self::PipeBrace => "pipe brace",
            Self::Unknown => "unknown",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Document => 0,
            Self::Atom => 1,
            Self::Parenthesis => 2,
            Self::SquareBracket => 3,
            Self::Brace => 4,
            Self::PipeText => 5,
            Self::PipeParenthesis => 6,
            Self::PipeBrace => 7,
            Self::Unknown => 15,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Document,
            1 => Self::Atom,
            2 => Self::Parenthesis,
            3 => Self::SquareBracket,
            4 => Self::Brace,
            5 => Self::PipeText,
            6 => Self::PipeParenthesis,
            7 => Self::PipeBrace,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
struct StructureHeaderBuilder {
    slots: Vec<StructureSlot>,
}

impl StructureHeaderBuilder {
    fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn push_document(&mut self, document: &Document) {
        self.push_slot(StructureSlot::new(
            StructureShape::Document,
            document.holds_root_objects(),
        ));
        for root_object in document.root_objects() {
            self.push_block(root_object, 1);
        }
    }

    fn push_block(&mut self, block: &Block, depth: usize) {
        if depth > 2 {
            return;
        }
        if !self.push_slot(block.structure_slot()) {
            return;
        }
        if depth == 2 {
            return;
        }
        for child in block.root_objects() {
            self.push_block(child, depth + 1);
        }
    }

    fn push_slot(&mut self, slot: StructureSlot) -> bool {
        if self.slots.len() < StructureHeader::MAXIMUM_SLOTS {
            self.slots.push(slot);
            true
        } else {
            if let Some(last) = self.slots.last_mut() {
                *last = StructureSlot::overflow();
            }
            false
        }
    }

    fn finish(self) -> StructureHeader {
        StructureHeader { slots: self.slots }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeText {
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Atom {
    text: String,
    classification: AtomClassification,
    span: SourceSpan,
}

impl Atom {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn classification(&self) -> AtomClassification {
        self.classification
    }

    pub fn source_span(&self) -> SourceSpan {
        self.span
    }

    pub fn qualifies_as_symbol(&self) -> bool {
        self.classification == AtomClassification::SymbolCandidate
    }

    pub fn qualifies_as_pascal_case_symbol(&self) -> bool {
        self.qualifies_as_symbol()
            && self
                .text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_uppercase())
            && !self.text.contains('-')
    }

    pub fn qualifies_as_camel_case_symbol(&self) -> bool {
        self.qualifies_as_symbol()
            && self
                .text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_lowercase())
            && !self.text.contains('-')
    }

    pub fn qualifies_as_kebab_case_symbol(&self) -> bool {
        self.qualifies_as_symbol() && self.text.contains('-')
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomClassification {
    SymbolCandidate,
    IntegerCandidate,
    DecimalCandidate,
    TextCandidate,
}

impl AtomClassification {
    pub fn classify(text: &str) -> Self {
        if text.parse::<i64>().is_ok() {
            Self::IntegerCandidate
        } else if text.parse::<f64>().is_ok() && text.contains('.') {
            Self::DecimalCandidate
        } else if text
            .chars()
            .all(|character| AtomCharacter::new(character).is_symbol())
            && text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        {
            Self::SymbolCandidate
        } else {
            Self::TextCandidate
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotaError {
    UnexpectedClose {
        found: char,
        position: SourcePosition,
    },
    UnclosedDelimiter {
        delimiter: Delimiter,
        position: SourcePosition,
    },
    UnclosedPipeText {
        position: SourcePosition,
    },
}

impl fmt::Display for NotaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedClose { found, position } => write!(
                formatter,
                "unexpected closing delimiter `{found}` at {}:{}",
                position.line, position.column
            ),
            Self::UnclosedDelimiter {
                delimiter,
                position,
            } => write!(
                formatter,
                "unclosed `{}` delimiter opened at {}:{}",
                delimiter.opening_text(),
                position.line,
                position.column
            ),
            Self::UnclosedPipeText { position } => write!(
                formatter,
                "unclosed `[|` pipe text opened at {}:{}",
                position.line, position.column
            ),
        }
    }
}

impl std::error::Error for NotaError {}

struct Parser<'source> {
    source: &'source str,
    cursor: Cursor,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            cursor: Cursor::default(),
        }
    }

    fn parse_document(&mut self) -> Result<Vec<Block>, NotaError> {
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Ok(root_objects);
            };
            if Delimiter::from_closing(character).is_some() {
                return Err(NotaError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    fn parse_object(&mut self) -> Result<Block, NotaError> {
        match self.peek() {
            Some('(') if self.peek_next() == Some('|') => {
                self.parse_pipe_delimited(Delimiter::PipeParenthesis)
            }
            Some('(') => self.parse_delimited(Delimiter::Parenthesis),
            Some('[') if self.peek_next() == Some('|') => self.parse_pipe_text(),
            Some('[') => self.parse_delimited(Delimiter::SquareBracket),
            Some('{') if self.peek_next() == Some('|') => {
                self.parse_pipe_delimited(Delimiter::PipeBrace)
            }
            Some('{') => self.parse_delimited(Delimiter::Brace),
            Some(_) => Ok(self.parse_atom()),
            None => Ok(self.parse_atom()),
        }
    }

    fn parse_pipe_delimited(&mut self, delimiter: Delimiter) -> Result<Block, NotaError> {
        let start = self.cursor.position();
        self.bump();
        self.bump();
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Err(NotaError::UnclosedDelimiter {
                    delimiter,
                    position: start,
                });
            };
            if character == '|' && self.peek_next() == Some(delimiter.closing()) {
                self.bump();
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::Delimited {
                    delimiter,
                    span: SourceSpan { start, end },
                    root_objects,
                });
            }
            if Delimiter::from_closing(character).is_some() {
                return Err(NotaError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    fn parse_delimited(&mut self, delimiter: Delimiter) -> Result<Block, NotaError> {
        let start = self.cursor.position();
        self.bump();
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Err(NotaError::UnclosedDelimiter {
                    delimiter,
                    position: start,
                });
            };
            if character == delimiter.closing() {
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::Delimited {
                    delimiter,
                    span: SourceSpan { start, end },
                    root_objects,
                });
            }
            if Delimiter::from_closing(character).is_some() {
                return Err(NotaError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    fn parse_pipe_text(&mut self) -> Result<Block, NotaError> {
        let start = self.cursor.position();
        self.bump();
        self.bump();
        let mut text = String::new();
        while let Some(character) = self.peek() {
            if character == '\\' {
                self.bump();
                if let Some(escaped) = self.peek() {
                    text.push(escaped);
                    self.bump();
                } else {
                    text.push('\\');
                }
            } else if character == '|' && self.peek_next() == Some(']') {
                self.bump();
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::PipeText(PipeText {
                    text,
                    span: SourceSpan { start, end },
                }));
            } else {
                text.push(character);
                self.bump();
            }
        }
        Err(NotaError::UnclosedPipeText { position: start })
    }

    fn parse_atom(&mut self) -> Block {
        let start = self.cursor.position();
        while let Some(character) = self.peek() {
            if character.is_whitespace()
                || character == ';'
                || Delimiter::from_opening(character).is_some()
                || Delimiter::from_closing(character).is_some()
                || self.at_pipe_delimiter_close()
            {
                break;
            }
            self.bump();
        }
        let end = self.cursor.position();
        let text = self.source[start.byte_offset..end.byte_offset].to_owned();
        let classification = AtomClassification::classify(&text);
        Block::Atom(Atom {
            text,
            classification,
            span: SourceSpan { start, end },
        })
    }

    fn at_pipe_delimiter_close(&self) -> bool {
        self.peek() == Some('|')
            && self
                .peek_next()
                .is_some_and(|character| Delimiter::from_closing(character).is_some())
    }

    fn skip_spacing(&mut self) {
        loop {
            match self.peek() {
                Some(character) if character.is_whitespace() => {
                    self.bump();
                }
                Some(';') => {
                    while let Some(character) = self.peek() {
                        self.bump();
                        if character == '\n' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor.byte_offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut characters = self.source[self.cursor.byte_offset..].chars();
        characters.next()?;
        characters.next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor.byte_offset += character.len_utf8();
        if character == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 1;
        } else {
            self.cursor.column += 1;
        }
        Some(character)
    }
}

#[derive(Clone, Debug)]
struct Cursor {
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }
}

impl Cursor {
    fn position(&self) -> SourcePosition {
        SourcePosition {
            byte_offset: self.byte_offset,
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct AtomCharacter {
    character: char,
}

impl AtomCharacter {
    fn new(character: char) -> Self {
        Self { character }
    }

    fn is_symbol(&self) -> bool {
        self.character.is_ascii_alphanumeric() || matches!(self.character, '_' | '-' | ':' | '*')
    }
}
