# Architecture

`dotos` owns the raw DOTOS structural floor.

## Direction

`dotos` is the new DOTOS implementation for the schema-derived stack — the raw
DOTOS replacement repository, not a branch-only temporary surface. The
predecessor surface is the existing `dotos` / `dotos-codec` family; this
repository carries the replacement track on `main`.

DOTOS gives methods on raw delimiter structures — factual delimiter predicates,
root-object queries, source spans, and on-demand structural candidate
predicates — and does not decide schema semantics. The raw layer discovers
structure only — delimiter boundaries, atom extents, pipe forms, and comments —
and that is syntax; it never classifies atom content into a meaning. Atom
meaning — decimal versus string versus variant versus reference — is not a fact
the parser records but a reading decided entirely by the expected type at the
position. Because DOTOS is strictly typed and positional, the expected type is
already known at every position, so a parser that determines what the type
already fixes is a design error. The raw pass therefore records no content
classification, and the earlier content-driven decimal reading — a scan that
guessed a number from the presence of a period — is removed as off-vision. The
first DOTOS pass breaks text into delimiter-balanced object blocks and emits a
compact first-two-level structure header that records delimiter/atom shape and
child counts only, so higher layers can triage before semantic lowering.

DOTOS is a typed text surface: a hack on the text user interface built from
delimiters, beauty, and typed structure where everything is read as a known type
in data-type-theory terms, and a well-formatted valid DOTOS expression decodes
reliably to its declared types (Spirit `rnrg`). That typed-text reliability is
the workspace's reason for rendering structured data as DOTOS everywhere. DOTOS
plus schema together form a pure data specification — more specific than Rust,
which mixes logic with data, and analogous to Cap'n Proto's own spec language —
the deliberate half-step from text-as-pseudocode toward the fully-typed SEMA
form, where DOTOS always corresponds to a specified object (Spirit `61lk`). The
bracket-only string form makes any DOTOS expression embedding-safe and
escape-free inside every double-quote host (JSON, Rust raw strings, Nix, YAML,
TOML, shell args, HTTP bodies, database columns, environment variables, XML
attributes), because DOTOS never emits a double-quote: DOTOS is the carrier and the
schema at each site supplies meaning (Spirit `7y8w`).

Bare atoms are the default string form. A string needs a delimiter only when it
carries spaces or otherwise-forbidden symbols; bare-eligible strings — topic
names, symbol-safe schema and type names — stay bare atoms (Spirit `0dsr`). The
bare-atom punctuation set is broad. A comment requires a double-semicolon marker
(`;;`); a single semicolon is ordinary bare-atom text (Spirit `laim`).
Classification is "qualifies as", not "is": a token qualifies as a symbol when it
is symbol-safe (PascalCase, camelCase, identifier alphabet), and whether it
actually is a symbol is a type-level question the macro and schema layers decide.
Symbol qualification is an on-demand structural-candidate predicate, not a meaning
stamped onto the atom while parsing: the reader answers "qualifies as a symbol"
only when a consumer asks under an expected type, and numeric meaning in
particular is never inferred by scanning content — an atom becomes an integer,
decimal, or string exclusively at decode under the expected type. The reading
default still prefers the higher qualification — qualified-symbol over string —
because narrowing later under type context is easy while widening is hard; the
schema layer demotes to string-only where its types require it (Spirit `fvtf`).

The `@` at-binding declaration sigil is retired. The earlier `Name@{...}`
struct-like, `Name@[...]` enum-like, and `name@(Reference ...)` member-binding
forms are removed; `dotos` does not parse `@` as a declaration or binding sigil.
The entire `@`-binder surface is abandoned and all `@`-binding parser support is
removed, so the authored surface is the positional bracket/brace form that
rejects `@` at source (Spirit `own9`). Declaration meaning is carried by position
and delimiter shape read through the typed macro-node layer, not by an `@`
open-delimiter interface. The root schema object is implicit from the filename
and needs no sigil or outer delimiter; legacy pipe declarations are transitional
and give way to the positional form.

Macro heads are not sigiled at the raw DOTOS layer: a macro name is just a symbol
candidate until a schema context reads a known schema-node position as a
tagged or data-carrying macro variant.

## Planes

- `Document` is an ordered list of root `Block`s parsed from source text.
- `Block` is either a delimited object, a pipe-text object, or an atom.
  Delimited objects are the three base pairs: standard parentheses, square
  brackets, and braces. The structural pipe forms `(|...|)` and `{|...|}` are
  REMOVED from the grammar and parser (see "Structural pipe forms are removed"):
  `(|` and `{|` no longer open a delimited block, so that text parses as whatever
  the base grammar yields or is rejected at source, the way the retired `@` sigil
  is rejected. Pipe-square `[|...|]` remains text, not recursive structure.
  Pipe-square text uses backslash escapes for literal close markers and
  backslashes (`\|]`, `\\`), so the delimiter shape remains bounded and readable
  while `DotosString` stays lossless.
- `Delimiter` owns the textual delimiter table: opening text, closing text,
  and wrapping child encodings. `Block` exposes delimiter-specific child
  queries so consumers do not destructure raw enum variants to recover a child
  slice.
- `SourceSpan` preserves byte, line, and column positions for diagnostics and
  later macro passes.
- Factual methods use `is_*`.
- Structural candidate methods use `qualifies_as_*`.
- `DotosDecode` and `DotosEncode` are the shared DOTOS value-codec traits used by
  hand-written Rust and by schema-emitted Rust.
- `DotosBody`, `DotosBodyDecode`, and `DotosBodyEncode` are the shared
  inner-object-stream codec layer. A matched file body or delimited block
  yields a body; the expected Rust type decides how to read that ordered stream.
- `dotos-derive` is the proc-macro companion crate re-exported by
  `dotos`. It derives `DotosDecode` and `DotosEncode` for named structs,
  one-field tuple newtypes, unit enum variants, one-payload enum variants, and
  enum variants with multiple unnamed fields encoded as `(Variant (field1
  field2 ...))`. Named struct derive emits body decode/encode first; ordinary
  parenthesized struct decode and `#[dotos(known_root)]` document decode both
  delegate into that body implementation. It also derives
  `StructuralMacroNode` for enum types with per-variant `#[shape(...)]`
  attributes (`pascal_atom`, `keyword = "..."`, `head = "...", arity = N`,
  `head = "...", atom`, `head = "...", body`, `pascal_head, arity = N`, and
  `pascal_head, body`), generating the ordered
  structural variant list, recursive per-field capture decoding, and reverse
  structural DOTOS encoding. A `keyword` variant matches a bare literal atom and
  carries no fields, so an inner marker atom can be its own recursively-decoded
  structural macro node. A `body` variant matches a literal-headed parenthesis
  of any object count and carries exactly one field; the headed tail is handed
  to that field type as a multi-block candidate, so `(Head item*)` lists decode
  into `Vec<Node>` through the vector node impl and `(Head a b c d)` records
  decode through the payload type's own ordered body read.
- `DotosSource`, `DotosBlock`, `DotosString`, and `DotosCollection` are the
  data-bearing codec helpers. They own single-root parsing, delimiter
  expectation, direct body parsing, string formatting, and collection value
  shapes. `DotosString` picks the least-delimited canonical form its content can
  carry faithfully: a period-joined chain of bare atoms renders bare (`file.txt`,
  `nix.prometheus.goldragon.criome`), single-space-separated bare-atom words
  render as the space-joined `( … )` form, and content with structural
  delimiters, `;;`, pipe-close markers, newlines, or irregular whitespace takes
  the literal-preserving `(| … |)` pipe form. A period is a structural dot
  operator at the raw layer, but an expected `String` reclaims the text it split:
  a dotted raw application rejoins into the bare string content — the exact
  parallel of a float reconstructed from its fractional period — so a
  period-bearing string needs no escape and the rejoin is case-blind. Typed
  `String` decoding rejects a bracketed or pipe-delimited string whenever the
  decoded text has a strictly less-delimited canonical form. Delimited DOTOS
  strings come exclusively from the two
  bracket forms (Spirit `vfjw`, `f8m3`): the canonical inline `[text]`
  square-bracket string for single-line strings (Spirit `7rrs`), and the
  four-character bracket-pipe block form for pretty indented multiline string
  blocks, whose common indentation is stripped on parse, with literal close-marker
  text escaped rather than the fences widened (Spirit `3qjw`, `bhs5`). One
  pipe-text shape is kept and literal close markers are escaped instead of adding
  more fence variants.
- `DotosDocumentBody` and `DotosDocumentEncoding` are the known-root document
  compatibility helpers over the shared body layer. They expose a file's root
  object stream as the body of the caller's known type, and they format the
  ordered body fields back to DOTOS without an outer wrapper.
- `DotosNamedDocumentFieldDecode` and `DotosNamedDocumentFieldEncode` let a
  known-root field be decoded from a positional body slot while receiving a
  name supplied by the root shape, for example an `Input` enum whose variants
  are stored directly in the root body.
- `Box<T>` is a storage wrapper only. Its codec delegates to `T` so recursive
  Rust data does not create a second DOTOS shape.
- `macros` is the reusable macro-node mechanism, expressed entirely through the
  typed structural-variant codec (`StructuralVariant`/`StructuralVariantSet`
  below). A `Pattern` of ordered `PatternElement`s matches a candidate block
  sequence, and `MacroMatch` returns named captures to the consumer. The retired
  standalone `MacroNodeDefinition`/`MacroRegistry` dispatch pair — a parallel
  matcher schema once consulted transitionally — is gone; the typed structural
  variants are the sole macro-node surface. Delimited captures expose the
  matched block's inner `DotosBody`, not the wrapper delimiter, so the next
  semantic parser always receives body contents. The mechanism is
  semantic-neutral: consumers may express struct/enum/newtype patterns, but
  dotos only matches atoms, delimiters, literals, and rest captures. A
  delimited pattern can carry a recursive `Pattern` over that block's children,
  giving consumers arbitrarily nested structural constraints without recursive
  text-template logic.
- `BlockShape` is the ergonomic per-variant structural description layered on
  top of `Pattern`. It gives structural macro authors names such as
  Pascal-case atom, headed parenthesis, Pascal-headed parenthesis, literal, and
  delimited block, then lowers those shapes into `Pattern` for typed structural
  variants.
- `StructuralVariant` and `StructuralVariantSet` are the codec-facing macro-node
  nouns. A variant carries a name, a structural pattern, and an expected-shape
  diagnostic. The set carries the expected position for one typed node and tries
  the variants in declaration order. Validation rejects silent conflicts,
  including a general Pascal-headed parenthesis variant that would make a later
  same-arity literal-headed variant unreachable.
- `StructuralMacroNode` is the typed enum bridge on top of the same mechanism.
  A consumer-provided enum type lists its structural variants in order, decodes
  the incoming `MacroCandidate` directly into a typed Rust value, and encodes
  back to the structural DOTOS surface. The `#[derive(StructuralMacroNode)]`
  implementation generates that variant list and the direct decode/encode hooks
  from the enum's declaration order and per-variant shape attributes. The
  ordered structural match belongs to the expected enum type's codec path, not
  to a global parser registry. `MacroMatch` remains the lower-level registry
  result for exploration and diagnostics, but it is no longer the typed-node
  trait boundary.

## Dotted prefix reading

DOTOS has no space-separated key-value pairs anywhere. A map entry is written as a
dotted prefix, `key.value`, and the end state keeps no other space-separated pair
form. A brace map is a sequence of `key.value` entries, not a run of adjacent
key and value objects.

The dot can never be a primary parsing character, because a string can contain
periods. Dot handling is situation-dependent: whether a leading dot splits a
prefix off the rest of an atom is decided by the position, never by the presence
of the character.

The reader is context-aware and mode-switching. DOTOS is strictly typed and
positional, so the expected type is known at every position; the reader switches
mode constantly as it advances, always knowing what it can possibly expect next.
A dotted prefix is conditionally expected under this mechanism — expected in some
modes and impossible in others.

Dot-splitting is one instance of a broader principle that governs the whole
reader: the parser never classifies content. The raw layer legitimately
discovers structure — where delimiters open and close, how far an atom extends,
which spans are pipe forms, and where comments sit — and structure is the only
thing syntax can tell it. What an atom means is a separate question with a single
answer: the expected type at the position. Since that type is always known, the
reader never needs, and must never attempt, to guess meaning by looking at an
atom's characters. Deciding whether a period splits a prefix is exactly such a
meaning question, so it is answered by the expectation mode and never by the
character being present. The removed decimal classification was the same category
of error in a different place — a content scan standing in for a decision the
type already owned — and it is gone for the same reason.

CONSTRAINT (invariant): dot-splitting is decided purely by expectation mode,
never by scanning content. This is the local form of the general rule that the
parser classifies no content anywhere.

- When the expected position can carry a dotted prefix, the reader looks for a
  top-level dot in the leading atom and splits the prefix from the remainder.
- In every other mode — expected `String` above all — no prefix is split off.
  The raw layer still binds a period into a dot-application, but an expected
  `String` rejoins that whole application into its flat dotted text instead of
  splitting a key from it, so the entire dotted chain is the string's content.
- The same text splits, rejoins, or stays a lone atom based only on the expected
  type at that position, so no value's content can ever change its parse shape.
  This is what "atomically composable and predictable" means for this mechanism.

There are exactly two dotted-prefix expectation kinds:

- CAPITALIZED: the head is a capitalized object — a type or generic application
  such as `Vector.X` or `Map.(Key Value)`.
- UNCAPITALIZED: leading lowercase name segments — map keys, import path
  segments, and field disambiguators.

The mechanism is implemented once in the DOTOS reader and exported as a reusable
mechanism. Downstream consumers — schema-language above all — reuse the exported
dotted-prefix reader rather than hand-rolling their own dotted-prefix reading.
The one mechanism offers two entry forms over the single shared split rule and
head-case check — a block-level reader for a parsed block sequence and a
string-level reader for a consumer holding an already-extracted atom's text —
so a caller with a raw string routes through the same expectation-driven
mechanism instead of re-deriving a local split.

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
captures mean and how the chosen variant is written back to source. DOTOS does
not discover macro meaning through a global parser.

Both structural pipe forms are removed from the grammar and parser (see
"Structural pipe forms are removed"), and the schema layer no longer assigns
declaration meaning to pipe-parenthesis or pipe-brace; `dotos` no longer reports
those delimiter shapes at all. `dotos` does not promote macro heads, validate
symbol case, or decide
whether a parenthesized object is a variant, a macro call, or ordinary data. It
classifies no atom content into a meaning: it reads a dotted prefix only where
the expected type makes one possible, the raw period stays an ordinary atom
character everywhere else, and no atom is read as a number, string, or variant
until the expected type at decode says so.

Macros themselves are data, the macro most of all: a macro is a serializable
data object with a name, position, pattern, and template, where pattern and
template are data trees rather than text with sigils. Macros pre-assemble,
serialize, and load as data; reading a schema loads pre-assembled macro data and
interprets schema data against it. The only code is one small generic
interpreter, with no per-macro bespoke parsing, and the test is that everything —
macros included — round-trips through both DOTOS text and rkyv bytes (Spirit
`4itr`). DOTOS extension is therefore type-directed structural matching over
raw-parser nodes that preserves delimiters, counts, sigils, and nesting: a
structural macro node is a DOTOS enum decoded by shape, whose codec matches
variants in declaration order — most-specific first, first wins — recursively,
and whose encode emits the matching block, realized through
`#[derive(StructuralMacroNode)]` with per-variant shape attributes rather than a
runtime registry or string-name dispatch. The pattern language is a typed enum of
primitives (arity, atom case, atom sigil, delimiter type, atom literal) so macro
data stays serializable (Spirit `xai7`). Everything reading DOTOS-shaped structure
above the raw parser goes through these typed structural macro nodes; surviving
hand-parsing sites are violations to fix, and a shape the nodes cannot express is
surfaced to the psyche rather than worked around. DOTOS itself should have a
schema whose Rust is schema-derived rather than a hand-maintained parser island,
and dispatch precedence among competing reference forms — built-in heads, then
declared macros, then the generic catch-all — is reified as one explicit typed
DOTOS value that every reader and the resolver consult (Spirit `v0n6`).

Macro application and enum declaration share the grouped head-plus-payload form
`(Foo (A B))`: in an enum declaration `Foo` is the declared type and `A`, `B`
are variants; in a macro invocation `Foo` is the operator applied to grouped
arguments (Spirit `fo38`). A user macro is declared inside a namespace with the
locked positional record `(Macro Input+ Output)` — the `Macro` tag plus one or
more input shapes plus exactly one output shape — and lives as a registry entry
resolvable by qualified name, with the explicit `Macro` tag chosen over
contextual shape-driven matching for introspectability (Spirit `h6fh`). Macro
loading runs a first indexing pass that collects every macro name from the schema
and its imports before any macro is invoked, so later passes resolve references
by name and forward and out-of-order references work across imports, with lazy
resolution loading a macro only when it is referenced during dispatch; macro
handlers receive `DotosBody` streams with the outer wrapper delimiters stripped at
match time, not the wrapper (Spirit `ydpa`). Because each macro's pattern records
where line breaks, indentation, and spacing belong, a DOTOS formatter can be
derived from the macro definitions themselves (Spirit `5p9s`).

## Structural pipe forms are removed

The closed delimiter set is the three base pairs — parenthesis, square-bracket,
brace — plus pipe-text, the sole surviving piped variant (Spirit `j9du`).
Pipe-text is the bracket-safe / multiline string. The two structural piped
variants that the earlier seed parser reported as `PipeParenthesis` /
`PipeBrace` delimited blocks are gone from the grammar, the parser, and the
codec.

The two STRUCTURAL pipe delimiter forms — pipe-parenthesis `(| … |)` and
pipe-brace `{| … |}` — are REMOVED. They existed only to mark a different object
class inside a mixed block: the schema layer read pipe-parenthesis as the
generic-declaration construct (Spirit `hh3z`) and pipe-brace as the trait/impl
construct (Spirit `bpyu`), each a way to fence off one object class living among
others in a shared block. The schema-language per-kind declaration block
principle — every object class gets its own dedicated block — removed their
reason to exist: once generics and impls each have their own block, there is no
mixed block left for a pipe fence to partition. The psyche settled that these
forms leave the language, and they have.

The removal has landed: the parser no longer produces `PipeParenthesis` /
`PipeBrace` blocks, the last consumer (schema-language) stopped reading them
before removal, and the closed delimiter set is now the three base pairs plus
pipe-text only. `(|` and `{|` no longer open a delimited block, so that text
parses as whatever the base grammar yields or is rejected at source the way the
retired `@` sigil is rejected.

Pipe-text is KEPT. It is the sole surviving piped form, retained for the
quotation-safety principle below.

The earlier open derive work — `#[shape(…)]` additions that would recognize the
structural pipe delimiters and their optional-ends matching — was dropped rather
than pursued, because the constructs it would have matched are gone. The
substrate stays meaning-free, and the constructs those pipe forms once carried
move to their own per-kind blocks in the schema layer instead of being fenced
inside a mixed block by delimiter shape.

## Quotation-safety

DOTOS must be quotation-safe. A whole DOTOS expression must be able to sit inside a
host quotation — a shell-quoted CLI argument above all, and equally a JSON
string, a Rust raw string, a Nix, YAML, or TOML value, an HTTP body, a database
column, or an environment variable — without escaping a pile of delimiters. This
is why prose is carried by the pipe-text form rather than by quote-delimited
strings: DOTOS never emits a double-quote, so nesting a DOTOS value inside a
double-quote host needs no escaping and the carrier stays lossless. Prose,
whitespace-bearing text, and delimiter-bearing text ride the bracket string
`[text]` and the multiline pipe-text `[|...|]` forms, and never a quote-fenced
string, precisely so the surrounding host quote is the only quote in play. This
principle is the reason pipe-text survives the retirement of the structural pipe
forms: it is not a declaration fence but the escape-free carrier for prose, and
retiring it would forfeit quotation-safety.

## Collection value shapes and section mapping

The codec's collection value shapes are structural DOTOS values: `Vec<T>` is a
square-bracket block, `BTreeMap<K, V>` is a brace block of dotted-prefix
`key.value` entries, and `Option<T>` is `None` or `(Some value)`. Those are serialization shapes, not
schema declaration syntax. The square bracket is and always has been DOTOS's
vector container delimiter; higher layers may read a vector at a typed position
as a product or field list, but `[]` itself stays a vector and is never redefined
as struct syntax (Spirit `qw1j`). An earlier low-certainty exploration that read
`[]` as struct-and-fields and `()` as enum-and-variants resolved into this
direction: parentheses carry enum and variant headers and choices while bracket
content is a vector that a typed position may read as a field list (Spirit
`ychx`). At a typed position expecting a string or
string newtype, bracket content reads as string data — a string is a vector of
characters — so `[]` is not unconditionally a vector (Spirit `voa8`). The brace
is a strict key-value map: every entry is exactly one key plus one value written
as a dotted-prefix `key.value` pair, with no single-token entries and no
space-separated pair form, and key-value-ness is low-level DOTOS structure that
macros may consume at schema positions (Spirit `ghw7`).

DOTOS structs are positional: position plus the read-time schema encodes meaning,
with no field-name tags. A plain PascalCase token is a unit variant; a
parenthesized variant carries data. Every field is always present, with explicit
`None` for an absent optional value rather than an omitted field (Spirit `vr32`).
At a slot whose type is already known to be an enum, the authored value may omit
that enum type name and supply the enum body directly — `((Parse Expression)
(Render Expression))` is valid where the surrounding slot fixes the outer enum
type — while variant tags inside the body stay named (Spirit `3sq4`). An enum
variant with an optional empty payload still renders as a data-carrying record
such as `(Technology None)`, not as a bare `Technology` atom (Spirit `oqwb`).
Multi-argument type references and applications use the CAPITALIZED dotted-prefix
form — a capitalized head then its arguments as a dotted group, as in
`Map.(Key Value)` — so the head names the construct and the application reads as
one dotted object rather than a run of space-separated arguments; this supersedes
the earlier flat `Map K V` positional form (Spirit `wqdi`). Encoders
avoid over-bracketing: bare-safe atoms encode bare inside typed positions such as
vectors, bracket forms are reserved for whitespace and delimiter safety, and
typed projection drops a redundant wrapper delimiter when the enclosing head
already fixes the payload shape (Spirit `3naf`). Plural record replies expose
their vector directly in the structure rather than nesting it inside a
single-field wrapper record (Spirit `vqbt`).

DOTOS owns raw structure and serialization shapes — including the value literals
`None` and `(Some x)` — while the schema layer owns the entire type-name
vocabulary: scalar names such as `String`, `Integer`, and `Boolean`, and
composite names such as `Vec`, `Optional`, and `Map`. All type-name keywords
belong to schema; splitting only composites into schema while leaving scalar
names in DOTOS would be inconsistent (Spirit `sqx6`). The composite type
constructors are nevertheless DOTOS-layer datatype objects that schema reads, not
schema-only sugar: `Vec`, `Option`, and `KeyValue` (named `KeyValue` rather than
`Map`, which risks reading as a verb, and carrying key type then value type as a
dotted-prefix capitalized application such as `KeyValue.(Key Value)`),
where a plain type-reference position can name scalars or composites without
declaring a new type while declarations use the positional struct, enum, and
newtype forms (Spirit `2dzp`). The three DOTOS delimiters map to the three schema
sections: parentheses hold enum and variant headers and choices, square brackets
hold positional struct field bodies, and braces hold the name-value namespace of
type names to definitions (Spirit `6oun`). That namespace section is a key-value
map of user-defined types — keys are type names, values are definitions — not a
flat sequence of separate declarations (Spirit `5myr`).

The codec has a shared body-content path. `DotosSource::parse` remains the
single-root-object path for ordinary values; after it matches the outer
parentheses of a named struct, derive hands the inner root-object stream to the
type's `DotosBodyDecode` implementation. `DotosSource::parse_document_body` is
the file/body path for callers that already know the root type from context,
such as a `.schema` reader; it hands the document's ordered root
objects to the same body logic through `DotosDocumentDecode`. The structural
match decides where the body begins. The expected type decides whether the body
is read as positional struct fields, a vector stream, an enum-like variant
body, or another value shape.

## Binary boundary

DOTOS is the text projection at one edge of a binary system; it is not the
transport. rkyv binary is the single encoded form, living both in the database as
the SEMA body at rest and on the wire as component messaging, with one
`AssembledSchema` reading those bytes in both places, while DOTOS is the text
projection at the CLI and inspection edge — and tests prove both boundaries, not
only the in-memory Rust types (Spirit `a9sq`). DOTOS mirrors the rkyv
self-describing root-plus-relative-pointer box layout: the root stays compact
with sized fields inline, while unsized or growing fields (`String`, `Vec`,
`Option`-of-unsized, nested records) become boxes appended after the root in
declaration order, so no box-index naming is needed and a coordinate like
`(vector-N element-M)` reads the M-th element of the N-th box vector as a direct
projection of the binary form (Spirit `n5ch`).

The text/binary boundary lives in the client. Clients translate DOTOS text into
binary protocol data and render typed replies and traces back to users; daemons
stay free of DOTOS decoding and avoid string surfaces except for genuine
user-authored string payloads (Spirit `b1vi`). A CLI that accepts DOTOS
structurally forces the daemon protocol to be binary, because the CLI is what
translates DOTOS into the binary protocol calls the daemon receives (Spirit
`o2xk`). A daemon binary therefore compiles with no DOTOS code at all: DOTOS
encode/decode is a thin-CLI text-edge concern, the daemon speaks binary and rkyv
exclusively and must not pull `dotos` into its artifact, and component crates gate
DOTOS behind a `dotos-text` feature that only the CLI binary enables, pairing with
`signal` being DOTOS-free (Spirit `t4gd`). DOTOS encode and decode derives are
accordingly optional generated surfaces applied only where needed; daemon-only
components stay binary-only over rkyv signal frames and carry no DOTOS decoding
code (Spirit `cyik`). Daemon startup takes one pre-generated rkyv `Configure`
message — a deploy helper or CLI authors and encodes the DOTOS — so bootstrap
needs no manager: the daemon opens its named store, applies `Configure` as the
first config when virgin or self-resumes when populated, and the same `Configure`
type is accepted live over the meta socket for runtime reconfiguration, making
bootstrap and runtime config one vocabulary and the baseline meta operation of
every meta-signal-component contract (Spirit `ur16`).

Component feedback, status, and errors are typed self-descriptive DOTOS enums and
structs whose names carry the meaning, so a well-named result type needs no
message string; the feedback language is enums and structs decoded through DOTOS,
specializing the strings-only-at-the-edges rule to the messaging surface (Spirit
`bexd`). Each typed symbol — type, variant, field, operation, route — has a
fully-qualified `SymbolPath` identity through the interface's global namespace,
the canonical machine-readable universal symbol form; machines reason about
identity via the qualified path and DOTOS renders that same path as human-readable
text at user-facing edges (Spirit `r0le`). Help retrieval is always one DOTOS
argument: every component supports `(Help)`, `(Help Main)`, `(Help Verb)`, and
nested forms in its DOTOS vocabulary rather than bare-word CLI args, with help docs
likely auto-wired through the signal-channel macro (Spirit `hetk`). Trace events
render as DOTOS at the client edge: a trace event type derives its DOTOS codec from
its schema definition, so display reduces to the standard DOTOS encoder rather than
ad-hoc `Display` formatting, and the rendered string is itself the typed-data text
projection (Spirit `8p0r`). `TraceEvent` is a transparent newtype or direct
object-name trace root when its only payload is the activated object name,
yielding a single object shape instead of noisy double-delimited DOTOS from a
one-field struct (Spirit `pmg5`).

The assembled schema is canonical DOTOS-and-rkyv only, round-tripped through its
writer and reader. All DOTOS output comes from the typed codec — a printed type
label is a real decodable shape, never a hand-rolled witness or ad hoc field
join, and a known-root file is its root body encoded through an object/body
abstraction over ordered fields (Spirit `hc0t`). The assembled-schema visibility
wrapper is a normal data-carrying variant: `(Public Name Value)` and
`(Private Name Value)`, with `Public`/`Private` as the variant head followed by
the name and value fields (Spirit `zg84`).
