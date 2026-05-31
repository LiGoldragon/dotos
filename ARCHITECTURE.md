# Architecture

`nota-next` owns the raw NOTA structural floor.

## Planes

- `Document` is an ordered list of root `Block`s parsed from source text.
- `Block` is either a delimited object, a pipe-text object, or an atom.
  Delimited objects include standard parentheses, square brackets, braces, and
  the recursive pipe forms `(|...|)` and `{|...|}`. Pipe-square `[|...|]`
  remains text, not recursive structure.
- `Delimiter` owns the textual delimiter table: opening text, closing text,
  and wrapping child encodings. `Block` exposes delimiter-specific child
  queries so consumers do not destructure raw enum variants to recover a child
  slice.
- `SourceSpan` preserves byte, line, and column positions for diagnostics and
  later macro passes.
- Factual methods use `is_*`.
- Structural candidate methods use `qualifies_as_*`.
- `NotaDecode` and `NotaEncode` are the shared NOTA value-codec traits used by
  hand-written Rust and by schema-emitted Rust.
- `NotaBody`, `NotaBodyDecode`, and `NotaBodyEncode` are the shared
  inner-object-stream codec layer. A matched file body or delimited block
  yields a body; the expected Rust type decides how to read that ordered stream.
- `nota-next-derive` is the proc-macro companion crate re-exported by
  `nota-next`. It derives `NotaDecode` and `NotaEncode` for named structs,
  one-field tuple newtypes, unit enum variants, one-payload enum variants, and
  enum variants with multiple unnamed fields encoded as `(Variant (field1
  field2 ...))`. Named struct derive emits body decode/encode first; ordinary
  parenthesized struct decode and `#[nota(known_root)]` document decode both
  delegate into that body implementation.
- `NotaSource`, `NotaBlock`, `NotaString`, and `NotaCollection` are the
  data-bearing codec helpers. They own single-root parsing, delimiter
  expectation, direct body parsing, string formatting, and collection value
  shapes.
- `NotaDocumentBody` and `NotaDocumentEncoding` are the known-root document
  compatibility helpers over the shared body layer. They expose a file's root
  object stream as the body of the caller's known type, and they format the
  ordered body fields back to NOTA without an outer wrapper.
- `NotaNamedDocumentFieldDecode` and `NotaNamedDocumentFieldEncode` let a
  known-root field be decoded from a positional body slot while receiving a
  name supplied by the root shape, for example an `Input` enum whose variants
  are stored directly in the root body.
- `Box<T>` is a storage wrapper only. Its codec delegates to `T` so recursive
  Rust data does not create a second NOTA shape.
- `macros` is the reusable macro-node mechanism. `MacroNodeDefinition`
  describes a structural pattern at a position, `MacroRegistry` dispatches a
  candidate block sequence through ordered definitions, and `MacroMatch`
  returns named captures to the consumer. Delimited captures expose the matched
  block's inner `NotaBody`, not the wrapper delimiter, so the next semantic
  parser always receives body contents. The mechanism is semantic-neutral:
  schema-next may register struct/enum/newtype patterns, but nota-next only
  matches atoms, delimiters, literals, and rest captures. A delimited pattern
  can carry a recursive `Pattern` over that block's children, giving consumers
  arbitrarily nested structural constraints without recursive text-template
  logic.

## At-Binding Syntax

The parser understands name-first sigil binding at an open delimiter:

- `Name@{ ... }` parses as a named struct-like declaration block.
- `Name@[ ... ]` parses as a named enum-like declaration block.
- `name@(Reference ...)` parses as a normal two-object member binding whose
  reference remains structural.
- `name@Type` remains an atom for the schema layer to read as a simple member
  binding.

This is syntax-layer structure only. The `@` marker is not a macro-call sigil;
macro calls remain values read against an expected schema-node type. The root
object of a `.schema` file is still implicit from the filename, so the root
does not carry a `Name@{...}` wrapper.

The recursive pipe delimiter forms remain available as low-level compatibility
blocks and for older schema fixtures. `Name@(...)` remains accepted as a
compatibility enum declaration shape, but authored schema should use
`Name@[...]` so parentheses stay available for composite type references and
schema macro calls.

## Boundary

This crate does not know what a schema type, field, declaration, enum, macro,
or import means. It only exposes the raw structure and value serialization
needed by the next layer.

The macro-node layer preserves that boundary. A macro definition says "this
shape matches here" and returns captures; it does not say whether the match is
a schema struct, an intent record, a deployment stanza, or any other consumer
object. Consumers attach vocabulary and lowering on top of the returned match.

The schema layer may assign declaration meaning to pipe-parenthesis or
pipe-brace, but `nota-next` only reports those delimiter shapes and their
children. It does not promote macro heads, validate symbol case, or decide
whether a parenthesized object is a variant, a macro call, or ordinary data.

The codec's collection value shapes are structural NOTA values: `Vec<T>` is a
square-bracket block, `BTreeMap<K, V>` is a brace block of key/value pairs, and
`Option<T>` is `None` or `(Some value)`. Those are serialization shapes, not
schema declaration syntax.

The codec has a shared body-content path. `NotaSource::parse` remains the
single-root-object path for ordinary values; after it matches the outer
parentheses of a named struct, derive hands the inner root-object stream to the
type's `NotaBodyDecode` implementation. `NotaSource::parse_document_body` is
the file/body path for callers that already know the root type from context,
such as a `.schema` or `.asschema` reader; it hands the document's ordered root
objects to the same body logic through `NotaDocumentDecode`. The structural
match decides where the body begins. The expected type decides whether the body
is read as positional struct fields, a vector stream, an enum-like variant
body, or another value shape.
