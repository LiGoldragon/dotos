use std::{collections::BTreeMap, fmt};

use crate::{Atom, Block, Delimiter};

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct CaptureName(String);

impl CaptureName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
)]
pub enum MacroDelimiter {
    Parenthesis,
    SquareBracket,
    Brace,
    PipeParenthesis,
    PipeBrace,
}

impl MacroDelimiter {
    pub fn from_delimiter(delimiter: Delimiter) -> Self {
        match delimiter {
            Delimiter::Parenthesis => Self::Parenthesis,
            Delimiter::SquareBracket => Self::SquareBracket,
            Delimiter::Brace => Self::Brace,
            Delimiter::PipeParenthesis => Self::PipeParenthesis,
            Delimiter::PipeBrace => Self::PipeBrace,
        }
    }

    pub fn from_block(block: &Block) -> Option<Self> {
        match block {
            Block::Delimited { delimiter, .. } => Some(Self::from_delimiter(*delimiter)),
            Block::PipeText(_) | Block::Atom(_) => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::PipeParenthesis => "pipe parenthesis",
            Self::PipeBrace => "pipe brace",
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub enum PositionPredicate {
    RootIndex(u64),
    DelimitedEntry(MacroDelimiter),
    Named(String),
}

impl PositionPredicate {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn describe(&self) -> String {
        match self {
            Self::RootIndex(index) => format!("root positional {index}"),
            Self::DelimitedEntry(delimiter) => format!("{} entry", delimiter.as_str()),
            Self::Named(name) => name.clone(),
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct MacroNodeDefinition {
    name: String,
    position: PositionPredicate,
    pattern: Pattern,
    expected: String,
}

impl MacroNodeDefinition {
    pub fn new(
        name: impl Into<String>,
        position: PositionPredicate,
        pattern: Pattern,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            position,
            pattern,
            expected: expected.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn position(&self) -> &PositionPredicate {
        &self.position
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn expected(&self) -> &str {
        &self.expected
    }

    pub fn matches<'block>(
        &self,
        candidate: &MacroCandidate<'block>,
    ) -> Option<MacroMatch<'block>> {
        if self.position != candidate.position {
            return None;
        }
        self.pattern
            .matches(candidate.blocks())
            .map(|captures| MacroMatch::new(self.name.clone(), captures))
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct Pattern {
    elements: Vec<PatternElement>,
}

impl Pattern {
    pub fn new(elements: Vec<PatternElement>) -> Self {
        Self { elements }
    }

    pub fn elements(&self) -> &[PatternElement] {
        &self.elements
    }

    pub fn matches<'block>(&self, blocks: &[&'block Block]) -> Option<MacroCaptures<'block>> {
        let mut captures = MacroCaptures::new();
        let mut index = 0;
        for element in &self.elements {
            match element {
                PatternElement::Rest(capture_name) => {
                    captures.insert(
                        capture_name.clone(),
                        CapturedValue::Blocks(blocks[index..].to_vec()),
                    );
                    return Some(captures);
                }
                _ => {
                    let block = blocks.get(index)?;
                    element.match_block(block, &mut captures)?;
                    index += 1;
                }
            }
        }
        if index == blocks.len() {
            Some(captures)
        } else {
            None
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
#[rkyv(
    bytecheck(bounds(
        __C: rkyv::validation::ArchiveContext,
        __C::Error: rkyv::rancor::Source
    )),
    serialize_bounds(
        __S: rkyv::ser::Writer + rkyv::ser::Allocator,
        __S::Error: rkyv::rancor::Source
    ),
    deserialize_bounds(__D::Error: rkyv::rancor::Source)
)]
pub enum PatternElement {
    Any(Option<CaptureName>),
    Atom(AtomShape),
    Delimited(DelimitedShape),
    Literal(String),
    Rest(CaptureName),
}

impl PatternElement {
    pub fn any(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::Any(capture.into())
    }

    pub fn atom(shape: AtomShape) -> Self {
        Self::Atom(shape)
    }

    pub fn delimited(shape: DelimitedShape) -> Self {
        Self::Delimited(shape)
    }

    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub fn rest(capture_name: CaptureName) -> Self {
        Self::Rest(capture_name)
    }

    fn match_block<'block>(
        &self,
        block: &'block Block,
        captures: &mut MacroCaptures<'block>,
    ) -> Option<()> {
        match self {
            Self::Any(capture_name) => {
                if let Some(capture_name) = capture_name {
                    captures.insert(capture_name.clone(), CapturedValue::Block(block));
                }
                Some(())
            }
            Self::Atom(shape) => shape.match_block(block, captures),
            Self::Delimited(shape) => shape.match_block(block, captures),
            Self::Literal(value) => {
                if block.demote_to_string() == Some(value.as_str()) {
                    Some(())
                } else {
                    None
                }
            }
            Self::Rest(_) => None,
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct AtomShape {
    case: Option<AtomCase>,
    sigil: Option<SigilSpec>,
    capture: Option<CaptureName>,
}

impl AtomShape {
    pub fn new(
        case: Option<AtomCase>,
        sigil: Option<SigilSpec>,
        capture: Option<CaptureName>,
    ) -> Self {
        Self {
            case,
            sigil,
            capture,
        }
    }

    pub fn symbol(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::new(Some(AtomCase::Symbol), None, capture.into())
    }

    pub fn pascal_case(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::new(Some(AtomCase::PascalCase), None, capture.into())
    }

    pub fn camel_case(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::new(Some(AtomCase::CamelCase), None, capture.into())
    }

    pub fn with_sigil(mut self, sigil: SigilSpec) -> Self {
        self.sigil = Some(sigil);
        self
    }

    fn match_block<'block>(
        &self,
        block: &'block Block,
        captures: &mut MacroCaptures<'block>,
    ) -> Option<()> {
        let atom = block.atom()?;
        if !self.matches_atom(atom) {
            return None;
        }
        if let Some(capture_name) = &self.capture {
            captures.insert(capture_name.clone(), CapturedValue::Block(block));
        }
        Some(())
    }

    fn matches_atom(&self, atom: &Atom) -> bool {
        self.case.is_none_or(|case| case.matches(atom))
            && self
                .sigil
                .as_ref()
                .is_none_or(|sigil| sigil.matches(atom.text()))
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
)]
pub enum AtomCase {
    Symbol,
    PascalCase,
    CamelCase,
    KebabCase,
}

impl AtomCase {
    pub fn matches(&self, atom: &Atom) -> bool {
        match self {
            Self::Symbol => atom.qualifies_as_symbol(),
            Self::PascalCase => atom.qualifies_as_pascal_case_symbol(),
            Self::CamelCase => atom.qualifies_as_camel_case_symbol(),
            Self::KebabCase => atom.qualifies_as_kebab_case_symbol(),
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct SigilSpec {
    character: String,
    position: SigilPosition,
}

impl SigilSpec {
    pub fn new(character: impl Into<String>, position: SigilPosition) -> Self {
        Self {
            character: character.into(),
            position,
        }
    }

    pub fn suffix(character: impl Into<String>) -> Self {
        Self::new(character, SigilPosition::Suffix)
    }

    pub fn prefix(character: impl Into<String>) -> Self {
        Self::new(character, SigilPosition::Prefix)
    }

    pub fn matches(&self, text: &str) -> bool {
        match self.position {
            SigilPosition::Prefix => text
                .strip_prefix(self.character.as_str())
                .is_some_and(|rest| !rest.is_empty()),
            SigilPosition::Suffix => text
                .strip_suffix(self.character.as_str())
                .is_some_and(|rest| !rest.is_empty()),
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
)]
pub enum SigilPosition {
    Prefix,
    Suffix,
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct DelimitedShape {
    delimiter: MacroDelimiter,
    object_count: MacroObjectCount,
    capture: Option<CaptureName>,
    children: Option<ChildPattern>,
}

impl DelimitedShape {
    pub fn new(
        delimiter: MacroDelimiter,
        object_count: MacroObjectCount,
        capture: Option<CaptureName>,
    ) -> Self {
        Self {
            delimiter,
            object_count,
            capture,
            children: None,
        }
    }

    pub fn any(delimiter: MacroDelimiter, capture: impl Into<Option<CaptureName>>) -> Self {
        Self::new(delimiter, MacroObjectCount::Any, capture.into())
    }

    pub fn with_children(mut self, children: ChildPattern) -> Self {
        self.children = Some(children);
        self
    }

    fn match_block<'block>(
        &self,
        block: &'block Block,
        captures: &mut MacroCaptures<'block>,
    ) -> Option<()> {
        if MacroDelimiter::from_block(block) != Some(self.delimiter) {
            return None;
        }
        if !self.object_count.matches(block.holds_root_objects()) {
            return None;
        }
        if let Some(children) = &self.children {
            let child_blocks = block.root_objects().iter().collect::<Vec<_>>();
            captures.extend(children.matches(&child_blocks)?);
        }
        if let Some(capture_name) = &self.capture {
            captures.insert(capture_name.clone(), CapturedValue::Block(block));
        }
        Some(())
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct ChildPattern {
    elements: Vec<ChildPatternElement>,
}

impl ChildPattern {
    pub fn new(elements: Vec<ChildPatternElement>) -> Self {
        Self { elements }
    }

    pub fn elements(&self) -> &[ChildPatternElement] {
        &self.elements
    }

    fn matches<'block>(&self, blocks: &[&'block Block]) -> Option<MacroCaptures<'block>> {
        let mut captures = MacroCaptures::new();
        let mut index = 0;
        for element in &self.elements {
            match element {
                ChildPatternElement::Rest(capture_name) => {
                    captures.insert(
                        capture_name.clone(),
                        CapturedValue::Blocks(blocks[index..].to_vec()),
                    );
                    return Some(captures);
                }
                _ => {
                    let block = blocks.get(index)?;
                    element.match_block(block, &mut captures)?;
                    index += 1;
                }
            }
        }
        if index == blocks.len() {
            Some(captures)
        } else {
            None
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub enum ChildPatternElement {
    Any(Option<CaptureName>),
    Atom(AtomShape),
    Delimited(ChildDelimitedShape),
    Literal(String),
    Rest(CaptureName),
}

impl ChildPatternElement {
    pub fn any(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::Any(capture.into())
    }

    pub fn atom(shape: AtomShape) -> Self {
        Self::Atom(shape)
    }

    pub fn delimited(shape: ChildDelimitedShape) -> Self {
        Self::Delimited(shape)
    }

    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub fn rest(capture_name: CaptureName) -> Self {
        Self::Rest(capture_name)
    }

    fn match_block<'block>(
        &self,
        block: &'block Block,
        captures: &mut MacroCaptures<'block>,
    ) -> Option<()> {
        match self {
            Self::Any(capture_name) => {
                if let Some(capture_name) = capture_name {
                    captures.insert(capture_name.clone(), CapturedValue::Block(block));
                }
                Some(())
            }
            Self::Atom(shape) => shape.match_block(block, captures),
            Self::Delimited(shape) => shape.match_block(block, captures),
            Self::Literal(value) => {
                if block.demote_to_string() == Some(value.as_str()) {
                    Some(())
                } else {
                    None
                }
            }
            Self::Rest(_) => None,
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct ChildDelimitedShape {
    delimiter: MacroDelimiter,
    object_count: MacroObjectCount,
    capture: Option<CaptureName>,
}

impl ChildDelimitedShape {
    pub fn new(
        delimiter: MacroDelimiter,
        object_count: MacroObjectCount,
        capture: Option<CaptureName>,
    ) -> Self {
        Self {
            delimiter,
            object_count,
            capture,
        }
    }

    pub fn any(delimiter: MacroDelimiter, capture: impl Into<Option<CaptureName>>) -> Self {
        Self::new(delimiter, MacroObjectCount::Any, capture.into())
    }

    fn match_block<'block>(
        &self,
        block: &'block Block,
        captures: &mut MacroCaptures<'block>,
    ) -> Option<()> {
        if MacroDelimiter::from_block(block) != Some(self.delimiter) {
            return None;
        }
        if !self.object_count.matches(block.holds_root_objects()) {
            return None;
        }
        if let Some(capture_name) = &self.capture {
            captures.insert(capture_name.clone(), CapturedValue::Block(block));
        }
        Some(())
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    nota_next::NotaDecode,
    nota_next::NotaEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub enum MacroObjectCount {
    Any,
    Even,
    Exact(u64),
}

impl MacroObjectCount {
    pub fn matches(&self, found: usize) -> bool {
        match self {
            Self::Any => true,
            Self::Even => found % 2 == 0,
            Self::Exact(expected) => *expected == found as u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MacroCandidate<'block> {
    position: PositionPredicate,
    blocks: Vec<&'block Block>,
}

impl<'block> MacroCandidate<'block> {
    pub fn new(position: PositionPredicate, blocks: Vec<&'block Block>) -> Self {
        Self { position, blocks }
    }

    pub fn from_block(position: PositionPredicate, block: &'block Block) -> Self {
        Self::new(position, vec![block])
    }

    pub fn from_pair(
        position: PositionPredicate,
        key: &'block Block,
        value: &'block Block,
    ) -> Self {
        Self::new(position, vec![key, value])
    }

    pub fn position(&self) -> &PositionPredicate {
        &self.position
    }

    pub fn blocks(&self) -> &[&'block Block] {
        &self.blocks
    }

    pub fn shape_description(&self) -> String {
        self.blocks
            .iter()
            .map(|block| block.structure_shape().as_str().to_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Clone, Debug)]
pub struct MacroRegistry {
    nodes: Vec<MacroNodeDefinition>,
}

impl MacroRegistry {
    pub fn new(nodes: Vec<MacroNodeDefinition>) -> Result<Self, MacroError> {
        let registry = Self { nodes };
        registry.validate_no_silent_conflicts()?;
        Ok(registry)
    }

    pub fn unchecked(nodes: Vec<MacroNodeDefinition>) -> Self {
        Self { nodes }
    }

    pub fn nodes(&self) -> &[MacroNodeDefinition] {
        &self.nodes
    }

    pub fn dispatch<'block>(
        &self,
        candidate: &MacroCandidate<'block>,
    ) -> Result<MacroMatch<'block>, MacroError> {
        let mut tried = Vec::new();
        let mut expected = Vec::new();
        for node in self
            .nodes
            .iter()
            .filter(|node| node.position() == candidate.position())
        {
            tried.push(node.name().to_owned());
            expected.push(format!("{}: {}", node.name(), node.expected()));
            if let Some(matched) = node.matches(candidate) {
                return Ok(matched);
            }
        }
        Err(MacroError::NoMatch {
            position: candidate.position().describe(),
            tried,
            expected,
            found: candidate.shape_description(),
        })
    }

    pub fn validate_no_silent_conflicts(&self) -> Result<(), MacroError> {
        for (index, first) in self.nodes.iter().enumerate() {
            for second in self.nodes.iter().skip(index + 1) {
                if first.position() == second.position() && first.pattern() == second.pattern() {
                    return Err(MacroError::Conflict(MacroConflict::new(
                        first.name().to_owned(),
                        second.name().to_owned(),
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MacroMatch<'block> {
    macro_name: String,
    captures: MacroCaptures<'block>,
}

impl<'block> MacroMatch<'block> {
    pub fn new(macro_name: String, captures: MacroCaptures<'block>) -> Self {
        Self {
            macro_name,
            captures,
        }
    }

    pub fn macro_name(&self) -> &str {
        &self.macro_name
    }

    pub fn captures(&self) -> &MacroCaptures<'block> {
        &self.captures
    }
}

#[derive(Clone, Debug)]
pub struct MacroCaptures<'block> {
    values: BTreeMap<CaptureName, CapturedValue<'block>>,
}

impl<'block> MacroCaptures<'block> {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: CaptureName, value: CapturedValue<'block>) {
        self.values.insert(name, value);
    }

    pub fn extend(&mut self, captures: MacroCaptures<'block>) {
        self.values.extend(captures.values);
    }

    pub fn get(&self, name: &CaptureName) -> Option<&CapturedValue<'block>> {
        self.values.get(name)
    }

    pub fn values(&self) -> &BTreeMap<CaptureName, CapturedValue<'block>> {
        &self.values
    }
}

impl Default for MacroCaptures<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum CapturedValue<'block> {
    Block(&'block Block),
    Blocks(Vec<&'block Block>),
}

impl<'block> CapturedValue<'block> {
    pub fn block(&self) -> Option<&'block Block> {
        match self {
            Self::Block(block) => Some(block),
            Self::Blocks(_) => None,
        }
    }

    pub fn blocks(&self) -> &[&'block Block] {
        match self {
            Self::Block(block) => std::slice::from_ref(block),
            Self::Blocks(blocks) => blocks,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroConflict {
    first: String,
    second: String,
}

impl MacroConflict {
    pub fn new(first: String, second: String) -> Self {
        Self { first, second }
    }

    pub fn first(&self) -> &str {
        &self.first
    }

    pub fn second(&self) -> &str {
        &self.second
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroError {
    NoMatch {
        position: String,
        tried: Vec<String>,
        expected: Vec<String>,
        found: String,
    },
    Conflict(MacroConflict),
}

impl fmt::Display for MacroError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatch {
                position,
                tried,
                expected,
                found,
            } => write!(
                formatter,
                "no macro matched at {position}; tried [{}]; expected [{}]; found {found}",
                tried.join(", "),
                expected.join(", ")
            ),
            Self::Conflict(conflict) => write!(
                formatter,
                "macro registry conflict between {} and {}",
                conflict.first(),
                conflict.second()
            ),
        }
    }
}

impl std::error::Error for MacroError {}
