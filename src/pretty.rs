//! Deterministic readability projection for NOTA documents.
//!
//! The canonical NOTA encoders emit a document on a single line. That form is
//! byte-stable and quotation-safe, but a large document is hard for a person to
//! read. [`PrettyLayout`] is an additive projection that reflows the same
//! document across indented lines without changing the canonical encoding.
//!
//! The projection is a pure reformatting. Every leaf — an atom or a pipe-text
//! block — re-emits the exact source bytes it was parsed from, and the layout
//! only adds whitespace between structural delimiters. NOTA treats whitespace
//! as an object separator that is skipped adjacent to delimiters, so pretty
//! output always re-parses to the same block tree as its source. The layout
//! therefore never touches atom content, string escaping, or delimiter shape;
//! it decides only where line breaks and indentation belong.
//!
//! This is a structural interim of the macro-derived formatter the schema stack
//! envisions (Spirit `5p9s`): a formatter that reads line-break and indentation
//! intent from each macro's own pattern. Until that lands, the rule set here is
//! deliberately small and data-like: break a delimited block across lines only
//! when its single-line form would not fit the line width at its indentation,
//! and indent one step per delimiter depth.

use crate::{Block, Delimiter, Document, NotaError};

/// A deterministic readability layout for NOTA documents.
///
/// The layout carries the two policy values that drive every break decision:
/// the target `line_width` a single-line form must fit within, and the
/// `indent_unit` number of spaces added per delimiter depth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrettyLayout {
    line_width: usize,
    indent_unit: usize,
}

impl Default for PrettyLayout {
    fn default() -> Self {
        Self::standard()
    }
}

impl PrettyLayout {
    /// The standard layout: an eighty-column target with two-space indentation.
    pub const fn standard() -> Self {
        Self {
            line_width: 80,
            indent_unit: 2,
        }
    }

    /// A layout with an explicit line-width target and indentation step.
    pub const fn new(line_width: usize, indent_unit: usize) -> Self {
        Self {
            line_width,
            indent_unit,
        }
    }

    /// Parse `source` as a NOTA document and render its readable projection.
    ///
    /// This is the entry point for a CLI that holds an already-encoded NOTA
    /// string and wants the same document reflowed for reading.
    pub fn render_nota(&self, source: &str) -> Result<String, NotaError> {
        let document = Document::parse(source)?;
        Ok(self.render_document(&document))
    }

    /// Render the readable projection of an already-parsed document.
    pub fn render_document(&self, document: &Document) -> String {
        let source = document.source();
        let mut rendered = String::new();
        for (index, root_object) in document.root_objects().iter().enumerate() {
            if index > 0 {
                rendered.push('\n');
            }
            self.render_block(root_object, source, 0, &mut rendered);
        }
        rendered
    }

    /// Render one block, assuming it begins at column `depth * indent_unit`.
    ///
    /// A block whose single-line form fits the remaining width is emitted
    /// inline. Otherwise a delimited block breaks: its opening delimiter, its
    /// children reflowed across indented lines, and its closing delimiter
    /// returned to the block's own indentation. A block with no children carries
    /// nothing to break and is always inline.
    fn render_block(&self, block: &Block, source: &str, depth: usize, rendered: &mut String) {
        let inline = block.render_inline(source);
        match block {
            Block::Delimited {
                delimiter,
                root_objects,
                ..
            } if !root_objects.is_empty() && !self.fits(&inline, depth) => {
                self.render_broken(*delimiter, root_objects, source, depth, rendered);
            }
            Block::Delimited { .. } | Block::PipeText(_) | Block::Atom(_) => {
                rendered.push_str(&inline);
            }
        }
    }

    /// Render a delimited block across lines: opening delimiter, then the
    /// children reflowed one indent step deeper, then the closing delimiter back
    /// at the block's own indentation.
    ///
    /// A line break falls only between children the source itself separated with
    /// whitespace. Source-adjacent children — a dotted-prefix head `key.` glued
    /// to its value block, or a capitalized application head `Head.` glued to its
    /// argument group — stay on one line with no separator, exactly as canonical
    /// NOTA writes them. Respecting the source's own token adjacency keeps the
    /// layout content-free: it never asks what a dotted pair means, only where
    /// the source placed a separator.
    fn render_broken(
        &self,
        delimiter: Delimiter,
        children: &[Block],
        source: &str,
        depth: usize,
        rendered: &mut String,
    ) {
        rendered.push_str(delimiter.opening_text());
        let mut previous: Option<&Block> = None;
        for child in children {
            let glued = previous.is_some_and(|previous| previous.immediately_precedes(child));
            if !glued {
                rendered.push('\n');
                self.push_indentation(depth + 1, rendered);
            }
            self.render_block(child, source, depth + 1, rendered);
            previous = Some(child);
        }
        rendered.push('\n');
        self.push_indentation(depth, rendered);
        rendered.push_str(delimiter.closing_text());
    }

    /// Whether `inline` fits the line width when placed at `depth` indentation.
    fn fits(&self, inline: &str, depth: usize) -> bool {
        depth * self.indent_unit + inline.chars().count() <= self.line_width
    }

    fn push_indentation(&self, depth: usize, rendered: &mut String) {
        for _ in 0..depth * self.indent_unit {
            rendered.push(' ');
        }
    }
}

impl Block {
    /// The canonical single-line form of this block's subtree.
    ///
    /// Leaves re-emit their exact source bytes, so atom content and pipe-text
    /// escaping are preserved verbatim; a delimited block wraps its children
    /// joined by single spaces. This is both the width measure the readability
    /// layout consults and the text it emits when a block fits on one line.
    pub fn render_inline(&self, source: &str) -> String {
        match self {
            Self::Delimited {
                delimiter,
                root_objects,
                ..
            } => {
                let mut rendered = String::from(delimiter.opening_text());
                let mut previous: Option<&Block> = None;
                for child in root_objects {
                    if previous.is_some_and(|previous| !previous.immediately_precedes(child)) {
                        rendered.push(' ');
                    }
                    rendered.push_str(&child.render_inline(source));
                    previous = Some(child);
                }
                rendered.push_str(delimiter.closing_text());
                rendered
            }
            Self::PipeText(_) | Self::Atom(_) => self.reemit(source).to_owned(),
        }
    }

    /// Whether this block ends exactly where `next` begins, with no source text
    /// between them. Source-adjacent siblings carry no separator in canonical
    /// NOTA — a dotted-prefix head `key.` glued to its value block, or a
    /// capitalized application head `Head.` glued to its argument group — so the
    /// readability layout keeps them on one line without inventing a space.
    fn immediately_precedes(&self, next: &Block) -> bool {
        self.source_span().end.byte_offset == next.source_span().start.byte_offset
    }
}
