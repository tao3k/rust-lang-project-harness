//! Attribute macros for Rust project harness tests.

mod test_attribute;

use proc_macro::TokenStream;

/// Mark a test as an agent-visible Rust harness test.
///
/// The generated test runs the package-level cargo-test harness gate before
/// executing the annotated function body, so agents can recognize and preserve
/// harness validation with the same clarity as runtime-specific test macros.
#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    test_attribute::expand(args, input)
}
