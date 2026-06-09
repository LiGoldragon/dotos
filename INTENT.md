# Intent

`nota-next` is the new NOTA implementation for the schema-derived stack.

Psyche intent:

*The raw NOTA replacement repository is nota-next. It is the new NOTA
implementation, not a branch-only temporary surface.*

*NOTA is the library that gives methods on raw delimiter structures: factual
delimiter predicates, root-object queries, source spans, and structural
candidate classification. It does not decide schema semantics.*

*The first NOTA pass breaks text into delimiter-balanced object blocks and
emits a compact first-two-level structure header. The header is structural
only: it records delimiter/atom shape and child counts so higher layers can
triage before semantic lowering.*

*Square brackets are vectors. Pipe-square `[|...|]` is the string-safe text
form; it is not recursive structure. Pipe text uses backslash as the normal
escape character: `\|]` carries a literal close marker and `\\` carries a
literal backslash. That keeps the delimiter shape readable and bounded while
letting NOTA string encoding stay lossless for bracket-bearing text.
Pipe-parenthesis `(|...|)` and pipe-brace `{|...|}` are recursive delimiter
forms: they preserve inner NOTA objects while giving higher schema layers
distinct structural shapes for low-level enum-like and struct-like
declarations.*

*Macro heads are not sigiled at the raw NOTA layer. A macro name is just a
symbol candidate until a schema context reads a known schema-node position as a
tagged/data-carrying macro variant.*

*The `@` at-binding declaration sigil is retired. The earlier `Name@{...}`
struct-like, `Name@[...]` enum-like, and `name@(Reference ...)` member-binding
forms are removed; nota-next does not parse `@` as a declaration/binding sigil.
Schema declaration meaning is carried by position and delimiter shape read
through the typed macro-node layer, not by an `@` open-delimiter interface.
Parentheses remain the composite/type-reference and macro-call argument shape
for schema (`(Vec Entry)`, `(Optional Kind)`, `(Map (Key Value))`). The root
schema object remains implicit from the filename and needs no sigil or outer
delimiter.*

*NOTA owns Rust value codec shapes through shared `NotaDecode` and
`NotaEncode` traits. The codec can read and write strings, integers, booleans,
vectors, ordered maps, and options as NOTA values. String encoding is minimal:
bare-safe strings render as bare atoms in typed string positions, bracket
strings render strings with whitespace or punctuation, and pipe text renders
delimiter-bearing strings. Schema owns the type-name vocabulary and declaration
semantics layered above those value shapes.*

*Known-root files are decoded as document bodies. When a caller already knows
the root type, NOTA should expose the ordered root objects as that type's body
instead of requiring an outer wrapper object. `NotaDocumentBody` and
`NotaDocumentEncoding` own that parse/format boundary so higher layers do not
hand-join field strings. The `#[nota(known_root)]` derive attribute makes this
the normal code path for typed Rust nouns that read a whole file as the known
root body.*

*After NOTA structural parsing matches a file body or a delimited object, the
next semantic parsing step should receive the matched body's inner object
stream rather than the outer delimiter wrapper. Known-root files and matched
delimited objects share the same body abstraction: the expected type decides
whether the body is read as a struct, vector, enum variant, or other value.*

*The shared NOTA codec includes derive macros for generated and hand-written
Rust nouns. Schema-generated Rust should derive `NotaDecode` and `NotaEncode`
instead of hand-emitting per-type codec implementations.*

*Rust storage indirection is not a NOTA value shape. `Box<T>` decodes and
encodes through the contained `T` without adding syntax, so recursive schema
data such as assembled type references can use boxed Rust fields while staying
the same NOTA data.*

*Macro nodes are a reusable NOTA-layer mechanism. NOTA owns the structural
pattern data, named capture extraction, conflict detection, and rich no-match
diagnostics. The low-level registry is a reusable matcher for standalone macro
exploration, but the typed macro-node vision is not a global NOTA parser or a
consumer-managed registry: the expected Rust type defines the macro node before
the value is read. Consumers such as schema-next own the vocabulary they attach
to those typed nodes and the semantic lowering they perform from the selected
match. Delimited macro patterns can also constrain their immediate children by
delimiter, object count, atom case, literal, or rest capture, so consumers can
express structural matches before semantic lowering without falling back to text
macros.*

*A typed structural macro node is the enum-shaped consumer of that mechanism.
The macro node is a type, and that type is an enum. Its codec reads an already
known expected type, tries the enum variants' structural matches in declaration
order, and only after a structural match is selected decodes the captures into
domain data. The same consumer type must encode back to the structural NOTA
surface, so schema sugar and other dialects remain specialized NOTA rather than
one-way lowering languages.*

*The authored API for those structural variants should speak in per-variant
shape vocabulary, not raw matcher plumbing. `BlockShape` is that vocabulary on
main: it names common structural cases such as Pascal atoms and headed
parentheses while lowering to the existing `Pattern` substrate.
`StructuralVariant` and `StructuralVariantSet` are the typed codec-facing nouns:
the enum supplies variants, the type supplies the position, and the codec path
selects a variant without exposing a registry as the design surface.*

*The enum type is also the derive surface. `#[derive(StructuralMacroNode)]`
reads per-variant `#[shape(...)]` attributes such as `pascal_atom`,
`keyword = "opens"`, `head = "Optional", arity = 2`, and
`pascal_head, arity = 2`; the generated implementation lists those variants in
declaration order, validates that later variants are still reachable, decodes
the incoming structural candidate directly into the enum payload fields
recursively, and writes the same structural NOTA surface back out. A `keyword`
variant matches a bare literal atom and carries no fields, so an inner marker
such as a stream-relation keyword can itself be a structural macro node decoded
recursively from one child, discriminating sibling forms without any
variant-name string comparison in hand-written code. The derive makes the
macro-node type itself the specification rather than making users hand-write a
parser or registry. The low-level `MacroMatch` registry remains an exploration
and diagnostics surface, not the required typed-node codec hook.*

The predecessor surface is the existing `nota` / `nota-codec` family. This
repository carries the replacement track on `main`.
