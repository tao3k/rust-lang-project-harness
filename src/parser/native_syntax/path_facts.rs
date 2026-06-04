//! Native Rust path-reference facts.

use syn::spanned::Spanned;

use super::facts::RustPathReferenceSyntax;

pub(super) fn path_reference_syntax(
    path: &syn::Path,
    is_test_context: bool,
) -> Option<RustPathReferenceSyntax> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let terminal_name = segments.last()?.clone();
    Some(RustPathReferenceSyntax {
        line: path.span().start().line.max(1),
        segments,
        terminal_name,
        is_test_context,
    })
}
