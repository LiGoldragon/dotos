//! Per-instance schema: the schema of one decoded value, captured by the
//! decoder as it validates the value.
//!
//! The load-bearing idea is that decoding *is* a type-directed traversal of a
//! value against a type. [`DotosDecodeTraced`] runs that exact traversal in a
//! projection mode: alongside the decoded value it returns an
//! [`InstanceSchema`] tree whose every node records the **type the decoder
//! expected** at that position. There is no second parser, no inspection by
//! string shape, and no per-type hand-written schema printer — the trace is a
//! by-product of the same recursion that already validates the value.
//!
//! The reference kind held at each node is a dotos-local [`TypeReference`]
//! (a type name plus the structural container forms `Vec` / `Optional` / `Map`
//! / `FixedBytes`). Higher layers project this into their own schema-value
//! vocabulary, such as a schema `SourceReference`, and render it through the
//! schema encoder; this base crate never formats schema text.

use crate::{Block, DotosDecode, DotosDecodeError};

/// The type reference the decoder expected at one value position.
///
/// `Named` carries the declared type name as the decoder saw it (`Kind`,
/// `Domain`, `Entry`, `Magnitude`). The container forms mirror the blanket
/// `DotosDecode` impls for `Vec`, `Option`, `BTreeMap`, and the byte sequences,
/// so an empty container still names its element type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeReference {
    Named(&'static str),
    Vector(Box<TypeReference>),
    Optional(Box<TypeReference>),
    Map(Box<TypeReference>, Box<TypeReference>),
    FixedBytes(usize),
}

impl TypeReference {
    pub fn named(name: &'static str) -> Self {
        Self::Named(name)
    }

    pub fn vector(element: TypeReference) -> Self {
        Self::Vector(Box::new(element))
    }

    pub fn optional(inner: TypeReference) -> Self {
        Self::Optional(Box::new(inner))
    }

    pub fn map(key: TypeReference, value: TypeReference) -> Self {
        Self::Map(Box::new(key), Box::new(value))
    }
}

/// The body of a per-instance schema node — present only when the decoded value
/// has children the decoder descended into. One body shape per decoder step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceSchemaBody {
    /// A terminal leaf the decoder read directly (`String`, `Integer`, bytes,
    /// a unit enum variant).
    Scalar,
    /// A single-field wrapper: the wrapper reference is the parent `expected`,
    /// and this carries the wrapped reference one level in.
    Newtype(Box<InstanceSchema>),
    /// A struct's fields in declared order.
    Struct(Vec<InstanceSchema>),
    /// An enum's chosen-variant payload, if the variant carried one. The
    /// `expected` of the parent node stays the enum name; the variant lives
    /// only in the decoded value.
    EnumPayload(Option<Box<InstanceSchema>>),
    /// A vector's actual elements, one node each.
    Vector(Vec<InstanceSchema>),
    /// An optional's present value, if any.
    Optional(Option<Box<InstanceSchema>>),
    /// A map's actual key/value pairs.
    Map(Vec<(InstanceSchema, InstanceSchema)>),
}

/// The per-instance schema of one value position: the type the decoder expected
/// there, optional provenance metadata, and the body for any descended
/// children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceSchema {
    expected: TypeReference,
    provenance: Option<TypeReference>,
    body: InstanceSchemaBody,
}

impl InstanceSchema {
    pub fn new(expected: TypeReference, body: InstanceSchemaBody) -> Self {
        Self {
            expected,
            provenance: None,
            body,
        }
    }

    /// Attach provenance — a fact that is not part of the one-to-one positional
    /// schema (a transparent wrapper type on a root variant). Provenance never
    /// adds a rendered token; it is carried only as metadata.
    pub fn with_provenance(mut self, provenance: TypeReference) -> Self {
        self.provenance = Some(provenance);
        self
    }

    pub fn scalar(expected: TypeReference) -> Self {
        Self::new(expected, InstanceSchemaBody::Scalar)
    }

    pub fn expected(&self) -> &TypeReference {
        &self.expected
    }

    pub fn provenance(&self) -> Option<&TypeReference> {
        self.provenance.as_ref()
    }

    pub fn body(&self) -> &InstanceSchemaBody {
        &self.body
    }
}

/// A decoded value paired with the per-instance schema captured during its
/// decode. One decode pass produces both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedWithSchema<Value> {
    value: Value,
    schema: InstanceSchema,
}

impl<Value> DecodedWithSchema<Value> {
    pub fn new(value: Value, schema: InstanceSchema) -> Self {
        Self { value, schema }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn schema(&self) -> &InstanceSchema {
        &self.schema
    }

    pub fn into_value(self) -> Value {
        self.value
    }

    pub fn into_parts(self) -> (Value, InstanceSchema) {
        (self.value, self.schema)
    }
}

/// Decode a value and capture its per-instance schema in one pass.
///
/// Every implementor runs the *same* type-directed traversal as its
/// [`DotosDecode`] impl, but at each step it also records the reference it
/// expected. The derive emits this alongside `DotosDecode`; this base crate
/// supplies the leaf and container impls.
pub trait DotosDecodeTraced: DotosDecode {
    /// The reference the decoder expects for `Self` at a parent position,
    /// before reading the value. Containers compose their element references
    /// here so an empty container still names its element type.
    fn instance_reference() -> TypeReference;

    /// Decode `block` into `Self` and the per-instance schema captured along
    /// the way.
    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError>;
}

macro_rules! scalar_traced {
    ($type:ty, $name:literal) => {
        impl DotosDecodeTraced for $type {
            fn instance_reference() -> TypeReference {
                TypeReference::named($name)
            }

            fn from_dotos_block_traced(
                block: &Block,
            ) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
                let value = <$type as DotosDecode>::from_dotos_block(block)?;
                Ok(DecodedWithSchema::new(
                    value,
                    InstanceSchema::scalar(<$type as DotosDecodeTraced>::instance_reference()),
                ))
            }
        }
    };
}

scalar_traced!(String, "String");
scalar_traced!(u8, "Integer");
scalar_traced!(u16, "Integer");
scalar_traced!(u32, "Integer");
scalar_traced!(u64, "Integer");
scalar_traced!(i32, "SignedInteger");
scalar_traced!(i64, "SignedInteger");
scalar_traced!(f64, "Float");
scalar_traced!(bool, "Boolean");

impl DotosDecodeTraced for crate::ByteSequence {
    fn instance_reference() -> TypeReference {
        TypeReference::named("Bytes")
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        let value = <Self as DotosDecode>::from_dotos_block(block)?;
        Ok(DecodedWithSchema::new(
            value,
            InstanceSchema::scalar(Self::instance_reference()),
        ))
    }
}

impl<const WIDTH: usize> DotosDecodeTraced for crate::FixedByteSequence<WIDTH> {
    fn instance_reference() -> TypeReference {
        TypeReference::FixedBytes(WIDTH)
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        let value = <Self as DotosDecode>::from_dotos_block(block)?;
        Ok(DecodedWithSchema::new(
            value,
            InstanceSchema::scalar(Self::instance_reference()),
        ))
    }
}

impl<Element> DotosDecodeTraced for Vec<Element>
where
    Element: DotosDecodeTraced,
{
    fn instance_reference() -> TypeReference {
        TypeReference::vector(Element::instance_reference())
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        // Mirror the blanket `DotosDecode for Vec` traversal: each element is
        // decoded against `Element`, and we keep the per-element schema node.
        let elements =
            crate::DotosCollection::new(block).parse_vector(Element::from_dotos_block_traced)?;
        let mut values = Vec::with_capacity(elements.len());
        let mut nodes = Vec::with_capacity(elements.len());
        for decoded in elements {
            let (value, schema) = decoded.into_parts();
            values.push(value);
            nodes.push(schema);
        }
        Ok(DecodedWithSchema::new(
            values,
            InstanceSchema::new(
                Self::instance_reference(),
                InstanceSchemaBody::Vector(nodes),
            ),
        ))
    }
}

impl<Inner> DotosDecodeTraced for Option<Inner>
where
    Inner: DotosDecodeTraced,
{
    fn instance_reference() -> TypeReference {
        TypeReference::optional(Inner::instance_reference())
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        let decoded =
            crate::DotosCollection::new(block).parse_option(Inner::from_dotos_block_traced)?;
        match decoded {
            Some(inner) => {
                let (value, schema) = inner.into_parts();
                Ok(DecodedWithSchema::new(
                    Some(value),
                    InstanceSchema::new(
                        Self::instance_reference(),
                        InstanceSchemaBody::Optional(Some(Box::new(schema))),
                    ),
                ))
            }
            None => Ok(DecodedWithSchema::new(
                None,
                InstanceSchema::new(
                    Self::instance_reference(),
                    InstanceSchemaBody::Optional(None),
                ),
            )),
        }
    }
}

impl<Key, Value> DotosDecodeTraced for std::collections::BTreeMap<Key, Value>
where
    Key: DotosDecodeTraced + Ord,
    Value: DotosDecodeTraced,
{
    fn instance_reference() -> TypeReference {
        TypeReference::map(Key::instance_reference(), Value::instance_reference())
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        // We re-run the map traversal capturing both the value map and the
        // per-pair schema nodes. `parse_map` collects the value map; the
        // capture closures record the schema pairs in iteration order, sharing
        // the staged key through cells so both closures may borrow it.
        use std::cell::RefCell;
        let pair_nodes: RefCell<Vec<(InstanceSchema, InstanceSchema)>> = RefCell::new(Vec::new());
        let staged_key: RefCell<Option<InstanceSchema>> = RefCell::new(None);
        let map = crate::DotosCollection::new(block).parse_map(
            |key_block| {
                let decoded = Key::from_dotos_block_traced(key_block)?;
                let (value, schema) = decoded.into_parts();
                *staged_key.borrow_mut() = Some(schema);
                Ok(value)
            },
            |value_block| {
                let decoded = Value::from_dotos_block_traced(value_block)?;
                let (value, schema) = decoded.into_parts();
                let key_schema = staged_key
                    .borrow_mut()
                    .take()
                    .expect("map value parsed after its key");
                pair_nodes.borrow_mut().push((key_schema, schema));
                Ok(value)
            },
        )?;
        Ok(DecodedWithSchema::new(
            map,
            InstanceSchema::new(
                Self::instance_reference(),
                InstanceSchemaBody::Map(pair_nodes.into_inner()),
            ),
        ))
    }
}

impl<Inner> DotosDecodeTraced for Box<Inner>
where
    Inner: DotosDecodeTraced,
{
    fn instance_reference() -> TypeReference {
        Inner::instance_reference()
    }

    fn from_dotos_block_traced(block: &Block) -> Result<DecodedWithSchema<Self>, DotosDecodeError> {
        let decoded = Inner::from_dotos_block_traced(block)?;
        let (value, schema) = decoded.into_parts();
        Ok(DecodedWithSchema::new(Box::new(value), schema))
    }
}
