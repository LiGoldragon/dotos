use std::fmt;

use nota_next::{
    Block, BlockShape, CaptureName, Document, MacroMatch, MacroObjectCount, PositionPredicate,
    StructuralMacroNode, StructuralVariant, StructuralVariantSet,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct TypeName(String);

impl TypeName {
    fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn from_block(block: &Block) -> Result<Self, TypeReferenceError> {
        let Some(text) = block.demote_to_string() else {
            return Err(TypeReferenceError::ExpectedPascalSymbol {
                found: block.structure_shape().as_str().to_owned(),
            });
        };
        if !block.qualifies_as_pascal_case_symbol() {
            return Err(TypeReferenceError::ExpectedPascalSymbol {
                found: block.structure_shape().as_str().to_owned(),
            });
        }
        Ok(Self::new(text))
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TypeReference {
    Named(TypeName),
    Optional(Box<TypeReference>),
    Vector(Box<TypeReference>),
    Application(TypeName, Box<TypeReference>),
}

impl TypeReference {
    fn from_source(source: &str) -> Result<Self, StructuralMacroError> {
        let document = Document::parse(source)
            .map_err(|error| StructuralMacroError::Parse(error.to_string()))?;
        match document.root_objects() {
            [single] => {
                Self::from_structural_block(single).map_err(StructuralMacroError::StructuralMacro)
            }
            many => Err(StructuralMacroError::ExpectedSingleRoot { found: many.len() }),
        }
    }

    fn variants_application_first() -> Vec<StructuralVariant> {
        vec![
            Self::named_variant(),
            Self::application_variant(),
            Self::optional_variant(),
            Self::vector_variant(),
        ]
    }

    fn from_block_application_first(block: &Block) -> Result<Self, StructuralMacroError> {
        let candidate = nota_next::MacroCandidate::from_block(Self::structural_position(), block);
        let variants = StructuralVariantSet::new(
            Self::structural_position(),
            Self::variants_application_first(),
        )
        .map_err(StructuralMacroError::StructuralVariant)?;
        let matched = variants
            .dispatch(&candidate)
            .map_err(StructuralMacroError::StructuralVariant)?;
        Self::from_structural_match(matched).map_err(StructuralMacroError::Decode)
    }

    fn named_variant() -> StructuralVariant {
        BlockShape::pascal_atom(Some(CaptureName::new("type_name")))
            .into_structural_variant("named type", "PascalCase type name atom")
    }

    fn optional_variant() -> StructuralVariant {
        BlockShape::headed_parenthesis(
            "Optional",
            MacroObjectCount::Exact(2),
            Some(CaptureName::new("signature")),
        )
        .into_structural_variant(
            "optional reference",
            "parenthesized Optional head carrying one reference argument",
        )
    }

    fn vector_variant() -> StructuralVariant {
        BlockShape::headed_parenthesis(
            "Vec",
            MacroObjectCount::Exact(2),
            Some(CaptureName::new("signature")),
        )
        .into_structural_variant(
            "vector reference",
            "parenthesized Vec head carrying one reference argument",
        )
    }

    fn application_variant() -> StructuralVariant {
        BlockShape::pascal_headed_parenthesis(
            MacroObjectCount::Exact(2),
            CaptureName::new("constructor"),
            Some(CaptureName::new("signature")),
        )
        .into_structural_variant(
            "application reference",
            "parenthesized PascalCase constructor carrying one reference argument",
        )
    }

    fn variant_match<'match_value, 'block>(
        matched: &'match_value MacroMatch<'block>,
    ) -> TypeReferenceMatch<'match_value, 'block> {
        TypeReferenceMatch::new(matched)
    }
}

impl StructuralMacroNode for TypeReference {
    type Error = TypeReferenceError;

    fn structural_position() -> PositionPredicate {
        PositionPredicate::named("TypeReference")
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        vec![
            Self::named_variant(),
            Self::optional_variant(),
            Self::vector_variant(),
            Self::application_variant(),
        ]
    }

    fn from_structural_match(matched: MacroMatch<'_>) -> Result<Self, Self::Error> {
        match matched.macro_name() {
            "named type" => {
                let block = Self::variant_match(&matched).block("type_name")?;
                Ok(Self::Named(TypeName::from_block(block)?))
            }
            "optional reference" => Ok(Self::Optional(Box::new(
                Self::variant_match(&matched).single_argument()?,
            ))),
            "vector reference" => Ok(Self::Vector(Box::new(
                Self::variant_match(&matched).single_argument()?,
            ))),
            "application reference" => {
                let matched_reference = Self::variant_match(&matched);
                let constructor = TypeName::from_block(matched_reference.block("constructor")?)?;
                Ok(Self::Application(
                    constructor,
                    Box::new(matched_reference.single_argument()?),
                ))
            }
            other => Err(TypeReferenceError::UnexpectedVariant(other.to_owned())),
        }
    }

    fn to_structural_nota(&self) -> String {
        match self {
            Self::Named(name) => name.to_string(),
            Self::Optional(inner) => format!("(Optional {})", inner.to_structural_nota()),
            Self::Vector(inner) => format!("(Vec {})", inner.to_structural_nota()),
            Self::Application(constructor, inner) => {
                format!("({constructor} {})", inner.to_structural_nota())
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TypeReferenceMatch<'match_value, 'block> {
    matched: &'match_value MacroMatch<'block>,
}

impl<'match_value, 'block> TypeReferenceMatch<'match_value, 'block> {
    fn new(matched: &'match_value MacroMatch<'block>) -> Self {
        Self { matched }
    }

    fn block(&self, capture_name: &'static str) -> Result<&'block Block, TypeReferenceError> {
        let name = CaptureName::new(capture_name);
        self.matched
            .block_capture(&name)
            .ok_or(TypeReferenceError::MissingCapture(capture_name))
    }

    fn single_argument(&self) -> Result<TypeReference, TypeReferenceError> {
        let name = CaptureName::new("arguments");
        let arguments = self
            .matched
            .capture(&name)
            .ok_or(TypeReferenceError::MissingCapture("arguments"))?
            .blocks();
        let [single] = arguments else {
            return Err(TypeReferenceError::ExpectedSingleArgument {
                found: arguments.len(),
            });
        };
        TypeReference::from_structural_block(single).map_err(TypeReferenceError::from)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StructuralMacroError {
    Parse(String),
    ExpectedSingleRoot { found: usize },
    StructuralVariant(nota_next::StructuralVariantError),
    Decode(TypeReferenceError),
    StructuralMacro(nota_next::StructuralMacroError<TypeReferenceError>),
}

impl fmt::Display for StructuralMacroError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::ExpectedSingleRoot { found } => {
                write!(
                    formatter,
                    "expected exactly one NOTA root object, found {found}"
                )
            }
            Self::StructuralVariant(error) => write!(formatter, "{error}"),
            Self::Decode(error) => write!(formatter, "{error}"),
            Self::StructuralMacro(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StructuralMacroError {}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TypeReferenceError {
    ExpectedPascalSymbol { found: String },
    ExpectedSingleArgument { found: usize },
    MissingCapture(&'static str),
    StructuralDispatch(String),
    UnexpectedVariant(String),
}

impl From<nota_next::StructuralMacroError<TypeReferenceError>> for TypeReferenceError {
    fn from(error: nota_next::StructuralMacroError<TypeReferenceError>) -> Self {
        match error {
            nota_next::StructuralMacroError::Dispatch(error) => {
                Self::StructuralDispatch(error.to_string())
            }
            nota_next::StructuralMacroError::MatchedNode(error) => error,
        }
    }
}

impl fmt::Display for TypeReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedPascalSymbol { found } => {
                write!(formatter, "expected a PascalCase symbol, found {found}")
            }
            Self::ExpectedSingleArgument { found } => {
                write!(formatter, "expected one structural argument, found {found}")
            }
            Self::MissingCapture(capture_name) => {
                write!(formatter, "missing capture {capture_name}")
            }
            Self::StructuralDispatch(error) => write!(formatter, "{error}"),
            Self::UnexpectedVariant(name) => write!(formatter, "unexpected variant {name}"),
        }
    }
}

impl std::error::Error for TypeReferenceError {}

struct RoundTripExample<'input> {
    inputs: &'input [&'input str],
}

impl<'input> RoundTripExample<'input> {
    fn new(inputs: &'input [&'input str]) -> Self {
        Self { inputs }
    }

    fn run(&self) {
        println!("== round-trip: NOTA text -> Rust value -> NOTA text ==\n");
        for input in self.inputs {
            let value =
                TypeReference::from_source(input).expect("decode structural type reference");
            let output = value.to_structural_nota();
            println!("in   : {input}");
            println!("rust : {value:?}");
            println!("out  : {output}");
            assert_eq!(*input, output, "text round-trip mismatch");
            let decoded_output =
                TypeReference::from_source(&output).expect("decode structural output");
            assert_eq!(value, decoded_output, "value round-trip mismatch");
            println!("       text round-trips exactly; value round-trips exactly\n");
        }

        self.print_declaration_order_shadowing();
    }

    fn print_declaration_order_shadowing(&self) {
        println!("== declaration order matters (first structural match wins) ==\n");
        let document = Document::parse("(Optional Integer)").expect("parse order fixture");
        let block = document
            .root_objects()
            .first()
            .expect("order fixture has one root object");
        let correct = TypeReference::from_structural_block(block).expect("correct order decodes");
        let shadowed =
            TypeReference::from_block_application_first(block).expect("wrong order decodes");
        println!("input: (Optional Integer)");
        println!("  Optional declared BEFORE Application -> {correct:?}");
        println!("  Application declared BEFORE Optional  -> {shadowed:?}");
        println!();
        println!("the general head shadows the specific one when mis-ordered;");
        println!("that is why variants are tried in declaration order.");
    }
}

fn main() {
    let inputs = [
        "Integer",
        "(Optional Integer)",
        "(Vec (Optional Integer))",
        "(Map Entry)",
        "(Vec (Map (Optional RecordIdentifier)))",
    ];
    RoundTripExample::new(&inputs).run();
}
