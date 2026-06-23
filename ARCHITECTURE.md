# Architecture

`nota` owns the raw NOTA structural floor.

## Planes

- `Document` is an ordered list of root `Block`s parsed from source text.
- `Block` is either a delimited object, a pipe-text object, or an atom.
  Delimited objects include standard parentheses, square brackets, braces, and
  the recursive pipe forms `(|...|)` and `{|...|}`. Pipe-square `[|...|]`
  remains text, not recursive structure. Pipe-square text uses backslash escapes
  for literal close markers and backslashes (`\|]`, `\\`), so the delimiter
  shape remains bounded and readable while `NotaString` stays lossless.
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
- `nota-derive` is the proc-macro companion crate re-exported by
  `nota`. It derives `NotaDecode` and `NotaEncode` for named structs,
  one-field tuple newtypes, unit enum variants, one-payload enum variants, and
  enum variants with multiple unnamed fields encoded as `(Variant (field1
  field2 ...))`. Named struct derive emits body decode/encode first; ordinary
  parenthesized struct decode and `#[nota(known_root)]` document decode both
  delegate into that body implementation. It also derives
  `StructuralMacroNode` for enum types with per-variant `#[shape(...)]`
  attributes (`pascal_atom`, `keyword = "..."`, `head = "...", arity = N`,
  `head = "...", atom`, `head = "...", body`, `pascal_head, arity = N`, and
  `pascal_head, body`), generating the ordered
  structural variant list, recursive per-field capture decoding, and reverse
  structural NOTA encoding. A `keyword` variant matches a bare literal atom and
  carries no fields, so an inner marker atom can be its own recursively-decoded
  structural macro node. A `body` variant matches a literal-headed parenthesis
  of any object count and carries exactly one field; the headed tail is handed
  to that field type as a multi-block candidate, so `(Head item*)` lists decode
  into `Vec<Node>` through the vector node impl and `(Head a b c d)` records
  decode through the payload type's own ordered body read.
- `NotaSource`, `NotaBlock`, `NotaString`, and `NotaCollection` are the
  data-bearing codec helpers. They own single-root parsing, delimiter
  expectation, direct body parsing, string formatting, and collection value
  shapes. `NotaString` renders broad bare-safe strings as bare atoms, inline
  brackets for whitespace-delimited strings, and pipe text for strings carrying
  structural delimiters, `;;`, pipe-close markers, or newlines. Typed `String`
  decoding rejects bracketed or pipe-delimited strings when the decoded text can
  be written as a bare atom.
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
  describes a standalone structural pattern at a position, `MacroRegistry`
  dispatches a candidate block sequence through ordered definitions, and
  `MacroMatch` returns named captures to the consumer. This registry surface is
  useful for low-level exploration and schema's existing transitional matcher,
  but it is not the conceptual home of typed macro nodes. Delimited captures
  expose the matched block's inner `NotaBody`, not the wrapper delimiter, so the
  next semantic parser always receives body contents. The mechanism is
  semantic-neutral: schema-next may register struct/enum/newtype patterns, but
  nota only matches atoms, delimiters, literals, and rest captures. A
  delimited pattern can carry a recursive `Pattern` over that block's children,
  giving consumers arbitrarily nested structural constraints without recursive
  text-template logic.
- `BlockShape` is the ergonomic per-variant structural description layered on
  top of `Pattern`. It gives structural macro authors names such as
  Pascal-case atom, headed parenthesis, Pascal-headed parenthesis, literal, and
  delimited block, then lowers those shapes into `Pattern` for either
  standalone macro definitions or typed structural variants.
- `StructuralVariant` and `StructuralVariantSet` are the codec-facing macro-node
  nouns. A variant carries a name, a structural pattern, and an expected-shape
  diagnostic. The set carries the expected position for one typed node and tries
  the variants in declaration order. Validation rejects silent conflicts,
  including a general Pascal-headed parenthesis variant that would make a later
  same-arity literal-headed variant unreachable.
- `StructuralMacroNode` is the typed enum bridge on top of the same mechanism.
  A consumer-provided enum type lists its structural variants in order, decodes
  the incoming `MacroCandidate` directly into a typed Rust value, and encodes
  back to the structural NOTA surface. The `#[derive(StructuralMacroNode)]`
  implementation generates that variant list and the direct decode/encode hooks
  from the enum's declaration order and per-variant shape attributes. The
  ordered structural match belongs to the expected enum type's codec path, not
  to a global parser registry. `MacroMatch` remains the lower-level registry
  result for exploration and diagnostics, but it is no longer the typed-node
  trait boundary.

## Boundary

This crate does not know what a schema type, field, declaration, enum, macro,
or import means. It only exposes the raw structure and value serialization
needed by the next layer.

The macro-node layer preserves that boundary. A macro definition says "this
shape matches here" and returns captures; it does not say whether the match is
a schema struct, an intent record, a deployment stanza, or any other consumer
object. Consumers attach vocabulary and lowering on top of the returned match.
The typed structural node layer preserves the same split while making the
expected type primary: the caller asks the codec for a known enum type, that
enum tries its ordered structural variants, and the enum decides what the
captures mean and how the chosen variant is written back to source. NOTA does
not discover macro meaning through a global parser.

The schema layer assigns declaration meaning to pipe-parenthesis and
pipe-brace, but `nota` only reports those delimiter shapes and their
children. It does not promote macro heads, validate symbol case, or decide
whether a parenthesized object is a variant, a macro call, or ordinary data.

The closed delimiter set is the three base pairs — parenthesis, square-bracket,
brace — plus three piped variants — pipe-text, pipe-parenthesis, pipe-brace
(Spirit `j9du`). Pipe-text is the bracket-safe / multiline string; the other two
`nota` reports as `PipeParenthesis` / `PipeBrace` delimited blocks without
interpreting them. The schema layer (not `nota`) assigns them:
pipe-parenthesis `(| … |)` is the generic-declaration construct (Spirit `hh3z`)
and pipe-brace `{| … |}` is the trait/impl construct (Spirit `bpyu`).

CONSTRAINT (what we want): the substrate stays meaning-free, and the
structural-macro shape vocabulary is rich enough to carry those constructs and
their sugar. Shape discrimination (delimiter, object count, atom case,
leading/trailing shape) lets optional elements be dropped and recovered by
shape — e.g. an impl `{| [params]? Trait Target [body]? |}` tells its marker,
parameterized, and bodied forms apart by whether a leading or trailing `[…]` is
present, with no keywords. OPEN (how): the `#[shape(…)]` additions that
recognize the pipe delimiters and the optional-ends matching are not yet
implemented. The parser already produces the `PipeParenthesis` / `PipeBrace`
blocks, so this is derive / structural-vocabulary work, not a seed-parser
change.

The codec's collection value shapes are structural NOTA values: `Vec<T>` is a
square-bracket block, `BTreeMap<K, V>` is a brace block of key/value pairs, and
`Option<T>` is `None` or `(Some value)`. Those are serialization shapes, not
schema declaration syntax.

The codec has a shared body-content path. `NotaSource::parse` remains the
single-root-object path for ordinary values; after it matches the outer
parentheses of a named struct, derive hands the inner root-object stream to the
type's `NotaBodyDecode` implementation. `NotaSource::parse_document_body` is
the file/body path for callers that already know the root type from context,
such as a `.schema` reader; it hands the document's ordered root
objects to the same body logic through `NotaDocumentDecode`. The structural
match decides where the body begins. The expected type decides whether the body
is read as positional struct fields, a vector stream, an enum-like variant
body, or another value shape.
