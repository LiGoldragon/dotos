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

    pub fn is_pipe_text(&self) -> bool {
        matches!(self, Self::PipeText(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self, Self::Atom(_))
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delimiter {
    Parenthesis,
    SquareBracket,
    Brace,
}

impl Delimiter {
    fn opening(self) -> char {
        match self {
            Self::Parenthesis => '(',
            Self::SquareBracket => '[',
            Self::Brace => '{',
        }
    }

    fn closing(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::SquareBracket => ']',
            Self::Brace => '}',
        }
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
    fn classify(text: &str) -> Self {
        if text.parse::<i64>().is_ok() {
            Self::IntegerCandidate
        } else if text.parse::<f64>().is_ok() && text.contains('.') {
            Self::DecimalCandidate
        } else if text.chars().all(is_symbol_character)
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
                delimiter.opening(),
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
            if is_closing_delimiter(character) {
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
            Some('(') => self.parse_delimited(Delimiter::Parenthesis),
            Some('[') if self.peek_next() == Some('|') => self.parse_pipe_text(),
            Some('[') => self.parse_delimited(Delimiter::SquareBracket),
            Some('{') => self.parse_delimited(Delimiter::Brace),
            Some(_) => Ok(self.parse_atom()),
            None => Ok(self.parse_atom()),
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
            if is_closing_delimiter(character) {
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
        let text_start = self.cursor.byte_offset;
        while let Some(character) = self.peek() {
            if character == '|' && self.peek_next() == Some(']') {
                let text = self.source[text_start..self.cursor.byte_offset].to_owned();
                self.bump();
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::PipeText(PipeText {
                    text,
                    span: SourceSpan { start, end },
                }));
            }
            self.bump();
        }
        Err(NotaError::UnclosedPipeText { position: start })
    }

    fn parse_atom(&mut self) -> Block {
        let start = self.cursor.position();
        while let Some(character) = self.peek() {
            if character.is_whitespace()
                || character == ';'
                || is_opening_delimiter(character)
                || is_closing_delimiter(character)
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

fn is_opening_delimiter(character: char) -> bool {
    matches!(character, '(' | '[' | '{')
}

fn is_closing_delimiter(character: char) -> bool {
    matches!(character, ')' | ']' | '}')
}

fn is_symbol_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
}
