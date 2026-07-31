# dotos

`dotos` is the replacement implementation of DOTOS's structural reader and
value codec. It reads delimiter-balanced DOTOS into blocks, keeps source spans,
exposes recursive object queries, and classifies atoms as structural
candidates.

The codec owns DOTOS value shapes for Rust values through `DotosDecode` and
`DotosEncode`: strings, fixed-width Rust integers, booleans, vectors, ordered
maps, and options. Schema type vocabulary and declaration semantics still live
in the schema layer, not here.
