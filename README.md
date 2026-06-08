# nota-next

`nota-next` is the replacement implementation of NOTA's structural reader and
value codec. It reads delimiter-balanced NOTA into blocks, keeps source spans,
exposes recursive object queries, and classifies atoms as structural
candidates.

The codec owns NOTA value shapes for Rust values through `NotaDecode` and
`NotaEncode`: strings, fixed-width Rust integers, booleans, vectors, ordered
maps, and options. Schema type vocabulary and declaration semantics still live
in `schema-next`, not here.
