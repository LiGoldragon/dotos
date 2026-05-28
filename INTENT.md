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
form; it is not recursive structure. Pipe-parenthesis `(|...|)` and pipe-brace
`{|...|}` are recursive delimiter forms: they preserve inner NOTA objects while
giving higher schema layers distinct structural shapes for low-level enum-like
and struct-like declarations.*

*Macro heads are not sigiled at the raw NOTA layer. A macro name is just a
symbol candidate until a schema context reads a known schema-node position as a
tagged/data-carrying macro variant.*

The predecessor surface is the existing `nota` / `nota-codec` family. This
repository carries the replacement track on `main`.
