//! Structural macro node demonstration: the enum type is the specification.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example structural_macro_round_trip
//! ```

use nota_next::{
    BlockShape, CaptureName, MacroCandidate, MacroMatch, PositionPredicate, StructuralMacroError,
    StructuralMacroNode, StructuralMacroNodeError, StructuralVariant, StructuralVariantSet,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct TypeName(String);

impl StructuralMacroNode for TypeName {
    type Error = StructuralMacroNodeError;

    fn structural_position() -> PositionPredicate {
        PositionPredicate::named("TypeName")
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        vec![
            BlockShape::pascal_atom(Some(CaptureName::new("field_0")))
                .into_structural_variant("TypeName", "PascalCase type name"),
        ]
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        let variants =
            StructuralVariantSet::new(Self::structural_position(), Self::structural_variants())
                .map_err(StructuralMacroError::Dispatch)?;
        let matched = variants
            .dispatch(&candidate)
            .map_err(StructuralMacroError::Dispatch)?;
        Self::from_match(matched).map_err(StructuralMacroError::MatchedNode)
    }

    fn to_structural_nota(&self) -> String {
        self.0.clone()
    }
}

impl TypeName {
    fn from_match(matched: MacroMatch<'_>) -> Result<Self, StructuralMacroNodeError> {
        let block = matched.block_capture(&CaptureName::new("field_0")).ok_or(
            StructuralMacroNodeError::MissingCapture {
                node: "TypeName",
                variant: "TypeName",
                capture: "field_0",
            },
        )?;
        let Some(text) = block.demote_to_string() else {
            return Err(StructuralMacroNodeError::MissingCapture {
                node: "TypeName",
                variant: "TypeName",
                capture: "field_0",
            });
        };
        Ok(Self(text.to_owned()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum TypeReference {
    #[shape(pascal_atom)]
    Named(TypeName),
    #[shape(head = "Optional", arity = 2)]
    Optional(Box<TypeReference>),
    #[shape(head = "Vec", arity = 2)]
    Vector(Box<TypeReference>),
    #[shape(pascal_head, arity = 2)]
    Application(TypeName, Box<TypeReference>),
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum MisorderedReference {
    #[shape(pascal_atom)]
    Named(TypeName),
    #[shape(pascal_head, arity = 2)]
    Application(TypeName, Box<MisorderedReference>),
    #[shape(head = "Optional", arity = 2)]
    Optional(Box<MisorderedReference>),
    #[shape(head = "Vec", arity = 2)]
    Vector(Box<MisorderedReference>),
}

struct RoundTripExample<'input> {
    inputs: &'input [&'input str],
}

impl<'input> RoundTripExample<'input> {
    fn new(inputs: &'input [&'input str]) -> Self {
        Self { inputs }
    }

    fn run(&self) {
        println!("== the enum type is the structural macro node spec ==\n");
        for input in self.inputs {
            let value = TypeReference::from_structural_nota(input).expect("decode");
            let output = value.to_structural_nota();
            println!("in  : {input}");
            println!("rust: {value:?}");
            println!("out : {output}");
            assert_eq!(*input, output, "text round-trip mismatch");
            let decoded_output = TypeReference::from_structural_nota(&output).expect("re-decode");
            assert_eq!(value, decoded_output, "value round-trip mismatch");
            println!("      text round-trips exactly; value round-trips exactly\n");
        }

        self.print_declaration_order_shadowing();
    }

    fn print_declaration_order_shadowing(&self) {
        println!("== declaration order is validated for unreachable variants ==\n");
        let input = "(Optional Integer)";
        let correct = TypeReference::from_structural_nota(input).expect("correct order");
        let rejected =
            MisorderedReference::from_structural_nota(input).expect_err("rejected order");
        println!("input: {input}");
        println!("  TypeReference (Optional before Application) -> {correct:?}");
        println!("  Misordered    (Application before Optional) -> {rejected}");
        println!();
        println!("a general variant may follow a specific one, but it may not make");
        println!("the later specific variant unreachable.");
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
