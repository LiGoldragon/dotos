use std::{collections::BTreeMap, fmt};

use crate::{Atom, Block, Delimiter, DotosBody};

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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
    dotos::DotosDecode,
    dotos::DotosEncode,
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
    Angle,
}

impl MacroDelimiter {
    pub fn from_delimiter(delimiter: Delimiter) -> Self {
        match delimiter {
            Delimiter::Parenthesis => Self::Parenthesis,
            Delimiter::SquareBracket => Self::SquareBracket,
            Delimiter::Brace => Self::Brace,
            Delimiter::Angle => Self::Angle,
        }
    }

    pub fn from_block(block: &Block) -> Option<Self> {
        match block {
            Block::Delimited { delimiter, .. } => Some(Self::from_delimiter(*delimiter)),
            Block::Application { .. } | Block::CurlyText(_) | Block::Atom(_) => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::Angle => "angle bracket",
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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

/// A position-scoped structural variant retained for consumers that own an
/// ordered registry rather than one typed variant set per decode site.
#[derive(Clone, Debug, Eq, PartialEq)]
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

    fn matches<'block>(&self, candidate: &MacroCandidate<'block>) -> Option<MacroMatch<'block>> {
        (self.position == *candidate.position())
            .then(|| self.pattern.matches(candidate.blocks()))
            .flatten()
            .map(|captures| MacroMatch::new(self.name.clone(), captures))
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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
pub enum BlockShape {
    Any(Option<CaptureName>),
    Atom(AtomShape),
    Delimited(DelimitedShape),
    Literal(String),
}

impl BlockShape {
    pub fn any(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::Any(capture.into())
    }

    pub fn atom(
        case: Option<AtomCase>,
        sigil: Option<SigilSpec>,
        capture: impl Into<Option<CaptureName>>,
    ) -> Self {
        Self::Atom(AtomShape::new(case, sigil, capture.into()))
    }

    pub fn symbol(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::atom(Some(AtomCase::Symbol), None, capture)
    }

    pub fn pascal_atom(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::atom(Some(AtomCase::PascalCase), None, capture)
    }

    pub fn camel_atom(capture: impl Into<Option<CaptureName>>) -> Self {
        Self::atom(Some(AtomCase::CamelCase), None, capture)
    }

    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub fn delimited(
        delimiter: MacroDelimiter,
        object_count: MacroObjectCount,
        capture: impl Into<Option<CaptureName>>,
    ) -> Self {
        Self::Delimited(DelimitedShape::new(delimiter, object_count, capture.into()))
    }

    pub fn headed_parenthesis(
        head: impl Into<String>,
        object_count: MacroObjectCount,
        capture: impl Into<Option<CaptureName>>,
    ) -> Self {
        Self::delimited(MacroDelimiter::Parenthesis, object_count, capture).with_children(
            Pattern::new(vec![
                PatternElement::literal(head),
                PatternElement::rest(CaptureName::new("arguments")),
            ]),
        )
    }

    pub fn pascal_headed_parenthesis(
        object_count: MacroObjectCount,
        head_capture: CaptureName,
        body_capture: impl Into<Option<CaptureName>>,
    ) -> Self {
        Self::delimited(MacroDelimiter::Parenthesis, object_count, body_capture).with_children(
            Pattern::new(vec![
                PatternElement::atom(AtomShape::pascal_case(Some(head_capture))),
                PatternElement::rest(CaptureName::new("arguments")),
            ]),
        )
    }

    pub fn with_children(self, children: Pattern) -> Self {
        match self {
            Self::Delimited(shape) => Self::Delimited(shape.with_children(children)),
            other => other,
        }
    }

    pub fn into_pattern_element(self) -> PatternElement {
        match self {
            Self::Any(capture) => PatternElement::any(capture),
            Self::Atom(shape) => PatternElement::atom(shape),
            Self::Delimited(shape) => PatternElement::delimited(shape),
            Self::Literal(value) => PatternElement::literal(value),
        }
    }

    pub fn into_pattern(self) -> Pattern {
        Pattern::new(vec![self.into_pattern_element()])
    }

    pub fn into_structural_variant(
        self,
        name: impl Into<String>,
        expected: impl Into<String>,
    ) -> StructuralVariant {
        StructuralVariant::new(name, self.into_pattern(), expected)
    }

    pub fn into_macro_node(
        self,
        name: impl Into<String>,
        position: PositionPredicate,
        expected: impl Into<String>,
    ) -> MacroNodeDefinition {
        MacroNodeDefinition::new(name, position, self.into_pattern(), expected)
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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

    fn silently_shadows(&self, later: &Self) -> bool {
        self.has_same_match_shape(later)
            || self
                .parenthesized_head_shape()
                .is_some_and(|earlier| earlier.silently_shadows(later.parenthesized_head_shape()))
    }

    fn has_same_match_shape(&self, other: &Self) -> bool {
        self.elements().len() == other.elements().len()
            && self
                .elements()
                .iter()
                .zip(other.elements())
                .all(|(left, right)| left.has_same_match_shape(right))
    }

    fn parenthesized_head_shape(&self) -> Option<ParenthesizedHeadShape<'_>> {
        match self.elements() {
            [PatternElement::Delimited(shape)] => shape.parenthesized_head_shape(),
            _ => None,
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub struct StructuralVariant {
    name: String,
    pattern: Pattern,
    expected: String,
}

impl StructuralVariant {
    pub fn new(name: impl Into<String>, pattern: Pattern, expected: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pattern,
            expected: expected.into(),
        }
    }

    pub fn from_shape(
        name: impl Into<String>,
        shape: BlockShape,
        expected: impl Into<String>,
    ) -> Self {
        shape.into_structural_variant(name, expected)
    }

    pub fn from_macro_node(node: MacroNodeDefinition) -> Self {
        Self::new(node.name, node.pattern, node.expected)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn expected(&self) -> &str {
        &self.expected
    }

    pub fn into_macro_node(self, position: PositionPredicate) -> MacroNodeDefinition {
        MacroNodeDefinition::new(self.name, position, self.pattern, self.expected)
    }

    pub fn matches<'block>(&self, blocks: &[&'block Block]) -> Option<MacroMatch<'block>> {
        self.pattern
            .matches(blocks)
            .map(|captures| MacroMatch::new(self.name.clone(), captures))
    }
}

#[derive(Clone, Debug)]
pub struct StructuralVariantSet {
    position: PositionPredicate,
    variants: Vec<StructuralVariant>,
}

impl StructuralVariantSet {
    pub fn new(
        position: PositionPredicate,
        variants: Vec<StructuralVariant>,
    ) -> Result<Self, StructuralVariantError> {
        let set = Self { position, variants };
        set.validate_no_silent_conflicts()?;
        Ok(set)
    }

    pub fn unchecked(position: PositionPredicate, variants: Vec<StructuralVariant>) -> Self {
        Self { position, variants }
    }

    pub fn position(&self) -> &PositionPredicate {
        &self.position
    }

    pub fn variants(&self) -> &[StructuralVariant] {
        &self.variants
    }

    pub fn dispatch<'block>(
        &self,
        candidate: &MacroCandidate<'block>,
    ) -> Result<MacroMatch<'block>, StructuralVariantError> {
        let mut tried = Vec::new();
        let mut expected = Vec::new();
        if self.position() == candidate.position() {
            for variant in self.variants() {
                tried.push(variant.name().to_owned());
                expected.push(format!("{}: {}", variant.name(), variant.expected()));
                if let Some(matched) = variant.matches(candidate.blocks()) {
                    return Ok(matched);
                }
            }
        }
        Err(StructuralVariantError::NoMatch {
            position: candidate.position().describe(),
            tried,
            expected,
            found: candidate.shape_description(),
        })
    }

    pub fn validate_no_silent_conflicts(&self) -> Result<(), StructuralVariantError> {
        for (index, first) in self.variants.iter().enumerate() {
            for second in self.variants.iter().skip(index + 1) {
                if first.pattern().silently_shadows(second.pattern()) {
                    return Err(StructuralVariantError::Conflict(
                        StructuralVariantConflict::new(
                            first.name().to_owned(),
                            second.name().to_owned(),
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Ordered, position-scoped structural dispatch retained for schema consumers
/// that register their grammar incrementally. New code should prefer
/// [`StructuralVariantSet`] at a known decode position.
#[derive(Clone, Debug)]
pub struct MacroRegistry {
    nodes: Vec<MacroNodeDefinition>,
}

impl MacroRegistry {
    pub fn new(nodes: Vec<MacroNodeDefinition>) -> Result<Self, StructuralVariantError> {
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

    pub fn validate_no_silent_conflicts(&self) -> Result<(), StructuralVariantError> {
        for (index, first) in self.nodes.iter().enumerate() {
            for second in self.nodes.iter().skip(index + 1) {
                if first.position() == second.position()
                    && first.pattern().silently_shadows(second.pattern())
                {
                    return Err(StructuralVariantError::Conflict(
                        StructuralVariantConflict::new(
                            first.name().to_owned(),
                            second.name().to_owned(),
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub type MacroConflict = StructuralVariantConflict;

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

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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

    fn has_same_match_shape(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any(_), Self::Any(_)) => true,
            (Self::Atom(left), Self::Atom(right)) => left.has_same_match_shape(right),
            (Self::Delimited(left), Self::Delimited(right)) => left.has_same_match_shape(right),
            (Self::Literal(left), Self::Literal(right)) => left == right,
            (Self::Rest(_), Self::Rest(_)) => true,
            (
                Self::Any(_)
                | Self::Atom(_)
                | Self::Delimited(_)
                | Self::Literal(_)
                | Self::Rest(_),
                Self::Any(_)
                | Self::Atom(_)
                | Self::Delimited(_)
                | Self::Literal(_)
                | Self::Rest(_),
            ) => false,
        }
    }

    fn parenthesized_head_shape(&self, arity: u64) -> Option<ParenthesizedHeadShape<'_>> {
        match self {
            Self::Atom(shape) if shape.matches_pascal_head() => {
                Some(ParenthesizedHeadShape::Pascal { arity })
            }
            Self::Literal(value) => Some(ParenthesizedHeadShape::Literal { arity, value }),
            Self::Any(_) | Self::Atom(_) | Self::Delimited(_) | Self::Rest(_) => None,
        }
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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

    fn has_same_match_shape(&self, other: &Self) -> bool {
        self.case == other.case && self.sigil == other.sigil
    }

    fn matches_pascal_head(&self) -> bool {
        self.case == Some(AtomCase::PascalCase) && self.sigil.is_none()
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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
    dotos::DotosDecode,
    dotos::DotosEncode,
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
    dotos::DotosDecode,
    dotos::DotosEncode,
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
    dotos::DotosDecode,
    dotos::DotosEncode,
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
pub struct DelimitedShape {
    delimiter: MacroDelimiter,
    object_count: MacroObjectCount,
    capture: Option<CaptureName>,
    #[rkyv(omit_bounds)]
    children: Option<Box<Pattern>>,
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

    pub fn with_children(mut self, children: Pattern) -> Self {
        self.children = Some(Box::new(children));
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
            captures.insert(
                capture_name.clone(),
                CapturedValue::Body(DotosBody::new(block.root_objects())),
            );
        }
        Some(())
    }

    fn has_same_match_shape(&self, other: &Self) -> bool {
        self.delimiter == other.delimiter
            && self.object_count == other.object_count
            && self.children_match_shape(other)
    }

    fn children_match_shape(&self, other: &Self) -> bool {
        match (&self.children, &other.children) {
            (Some(left), Some(right)) => left.has_same_match_shape(right),
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        }
    }

    fn parenthesized_head_shape(&self) -> Option<ParenthesizedHeadShape<'_>> {
        if self.delimiter != MacroDelimiter::Parenthesis {
            return None;
        }
        let MacroObjectCount::Exact(arity) = self.object_count else {
            return None;
        };
        let children = self.children.as_ref()?;
        match children.elements() {
            [head, PatternElement::Rest(_)] => head.parenthesized_head_shape(arity),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParenthesizedHeadShape<'shape> {
    Pascal { arity: u64 },
    Literal { arity: u64, value: &'shape str },
}

impl<'shape> ParenthesizedHeadShape<'shape> {
    fn silently_shadows(&self, later: Option<Self>) -> bool {
        match (self, later) {
            (
                Self::Pascal { arity: first },
                Some(Self::Literal {
                    arity: second,
                    value,
                }),
            ) => first == &second && Self::literal_matches_pascal_head(value),
            _ => false,
        }
    }

    fn literal_matches_pascal_head(value: &str) -> bool {
        value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
            && !value.contains('-')
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    dotos::DotosDecode,
    dotos::DotosEncode,
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

    pub fn capture(&self, name: &CaptureName) -> Option<&CapturedValue<'block>> {
        self.captures.get(name)
    }

    pub fn block_capture(&self, name: &CaptureName) -> Option<&'block Block> {
        self.capture(name).and_then(CapturedValue::block)
    }

    pub fn body_capture(&self, name: &CaptureName) -> Option<&DotosBody<'block>> {
        self.capture(name).and_then(CapturedValue::body)
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
    Body(DotosBody<'block>),
}

impl<'block> CapturedValue<'block> {
    pub fn block(&self) -> Option<&'block Block> {
        match self {
            Self::Block(block) => Some(block),
            Self::Blocks(_) | Self::Body(_) => None,
        }
    }

    pub fn body(&self) -> Option<&DotosBody<'block>> {
        match self {
            Self::Body(body) => Some(body),
            Self::Block(_) | Self::Blocks(_) => None,
        }
    }

    pub fn blocks(&self) -> &[&'block Block] {
        match self {
            Self::Block(block) => std::slice::from_ref(block),
            Self::Blocks(blocks) => blocks,
            Self::Body(_) => &[],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralVariantConflict {
    first: String,
    second: String,
}

impl StructuralVariantConflict {
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
pub enum StructuralVariantError {
    NoMatch {
        position: String,
        tried: Vec<String>,
        expected: Vec<String>,
        found: String,
    },
    Conflict(StructuralVariantConflict),
}

impl fmt::Display for StructuralVariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatch {
                position,
                tried,
                expected,
                found,
            } => write!(
                formatter,
                "no structural variant matched at {position}; tried [{}]; expected [{}]; found {found}",
                tried.join(", "),
                expected.join(", ")
            ),
            Self::Conflict(conflict) => write!(
                formatter,
                "structural variant conflict between {} and {}",
                conflict.first(),
                conflict.second()
            ),
        }
    }
}

impl std::error::Error for StructuralVariantError {}

pub trait StructuralMacroNode: Sized {
    type Error;

    fn structural_position() -> PositionPredicate;

    fn structural_variants() -> Vec<StructuralVariant>;

    fn to_structural_dotos(&self) -> String;

    fn from_structural_dotos(source: &str) -> Result<Self, StructuralMacroError<Self::Error>> {
        let document =
            crate::Document::parse(source).map_err(|error| StructuralMacroError::Parse {
                error: error.to_string(),
            })?;
        match document.root_objects() {
            [single] => Self::from_structural_block(single),
            many => Err(StructuralMacroError::ExpectedSingleRoot { found: many.len() }),
        }
    }

    fn from_structural_block(block: &Block) -> Result<Self, StructuralMacroError<Self::Error>> {
        Self::from_structural_candidate(MacroCandidate::from_block(
            Self::structural_position(),
            block,
        ))
    }

    fn from_structural_pair(
        key: &Block,
        value: &Block,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        Self::from_structural_candidate(MacroCandidate::from_pair(
            Self::structural_position(),
            key,
            value,
        ))
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>>;
}

impl<Inner> StructuralMacroNode for Box<Inner>
where
    Inner: StructuralMacroNode,
{
    type Error = Inner::Error;

    fn structural_position() -> PositionPredicate {
        Inner::structural_position()
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        Inner::structural_variants()
    }

    fn to_structural_dotos(&self) -> String {
        self.as_ref().to_structural_dotos()
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        Ok(Box::new(Inner::from_structural_candidate(candidate)?))
    }
}

/// A vector reads a candidate's whole block sequence as ordered items, so a
/// `body`-shaped variant can carry `(Head item*)` tails without a wrapper
/// object. Encoding writes the items back space-separated as body objects.
impl<Item> StructuralMacroNode for Vec<Item>
where
    Item: StructuralMacroNode,
{
    type Error = Item::Error;

    fn structural_position() -> PositionPredicate {
        Item::structural_position()
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        Item::structural_variants()
    }

    fn to_structural_dotos(&self) -> String {
        self.iter()
            .map(Item::to_structural_dotos)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        candidate
            .blocks()
            .iter()
            .map(|block| Item::from_structural_block(block))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralMacroNodeError {
    MissingCapture {
        node: &'static str,
        variant: &'static str,
        capture: &'static str,
    },
    MissingSlot {
        node: &'static str,
        variant: &'static str,
        capture: &'static str,
        slot: usize,
    },
    Field {
        node: &'static str,
        variant: &'static str,
        field: usize,
        error: String,
    },
    UnexpectedVariant {
        node: &'static str,
        variant: String,
    },
}

impl fmt::Display for StructuralMacroNodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCapture {
                node,
                variant,
                capture,
            } => write!(
                formatter,
                "{node}::{variant} is missing structural capture {capture}"
            ),
            Self::MissingSlot {
                node,
                variant,
                capture,
                slot,
            } => write!(
                formatter,
                "{node}::{variant} capture {capture} is missing slot {slot}"
            ),
            Self::Field {
                node,
                variant,
                field,
                error,
            } => write!(
                formatter,
                "{node}::{variant} failed to decode field {field}: {error}"
            ),
            Self::UnexpectedVariant { node, variant } => {
                write!(formatter, "{node} matched unexpected variant {variant}")
            }
        }
    }
}

impl std::error::Error for StructuralMacroNodeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralMacroError<NodeError> {
    Parse { error: String },
    ExpectedSingleRoot { found: usize },
    Dispatch(StructuralVariantError),
    MatchedNode(NodeError),
}

impl<NodeError> fmt::Display for StructuralMacroError<NodeError>
where
    NodeError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { error } => write!(formatter, "{error}"),
            Self::ExpectedSingleRoot { found } => {
                write!(
                    formatter,
                    "expected exactly one DOTOS root object, found {found}"
                )
            }
            Self::Dispatch(error) => write!(formatter, "{error}"),
            Self::MatchedNode(error) => write!(formatter, "{error}"),
        }
    }
}

impl<NodeError> std::error::Error for StructuralMacroError<NodeError> where
    NodeError: std::error::Error + 'static
{
}
