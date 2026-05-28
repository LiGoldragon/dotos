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

## Boundary

This crate does not know what a type, field, schema, enum, macro, or import
means. It only exposes the structure needed by the next layer.

The schema layer may assign declaration meaning to pipe-parenthesis or
pipe-brace, but `nota-next` only reports those delimiter shapes and their
children. It does not promote macro heads, validate symbol case, or decide
whether a parenthesized object is a variant, a macro call, or ordinary data.
