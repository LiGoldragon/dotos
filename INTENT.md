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

The predecessor surface is the existing `nota` / `nota-codec` family. This
repository carries the replacement track on `main`.
