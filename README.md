# nota-next

`nota-next` is the replacement implementation of NOTA's structural reader.
It is intentionally narrow: it reads delimiter-balanced NOTA into blocks,
keeps source spans, exposes recursive object queries, and classifies atoms as
structural candidates.

Schema semantics live in `schema-next`, not here.
