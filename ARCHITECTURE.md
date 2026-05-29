# Architecture

`nota-next` owns the raw NOTA structural floor.

## Planes

- `Document` is an ordered list of root `Block`s parsed from source text.
- `Block` is either a delimited object, a pipe-text object, or an atom.
  Delimited objects include standard parentheses, square brackets, braces, and
  the recursive pipe forms `(|...|)` and `{|...|}`. Pipe-square `[|...|]`
  remains text, not recursive structure.
- `SourceSpan` preserves byte, line, and column positions for diagnostics and
  later macro passes.
- Factual methods use `is_*`.
- Structural candidate methods use `qualifies_as_*`.
- `NotaDecode` and `NotaEncode` are the shared NOTA value-codec traits used by
  hand-written Rust and by schema-emitted Rust.
- `nota-next-derive` is the proc-macro companion crate re-exported by
  `nota-next`. It derives `NotaDecode` and `NotaEncode` for named structs,
  one-field tuple newtypes, unit enum variants, and one-payload enum variants.
- `NotaSource`, `NotaBlock`, `NotaString`, and `NotaCollection` are the
  data-bearing codec helpers. They own single-root parsing, delimiter
  expectation, string formatting, and collection value shapes.

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

The schema layer may assign declaration meaning to pipe-parenthesis or
pipe-brace, but `nota-next` only reports those delimiter shapes and their
children. It does not promote macro heads, validate symbol case, or decide
whether a parenthesized object is a variant, a macro call, or ordinary data.

The codec's collection value shapes are structural NOTA values: `Vec<T>` is a
square-bracket block, `BTreeMap<K, V>` is a brace block of key/value pairs, and
`Option<T>` is `None` or `(Some value)`. Those are serialization shapes, not
schema declaration syntax.
