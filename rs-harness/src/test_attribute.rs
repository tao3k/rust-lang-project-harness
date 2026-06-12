//! Expansion for `#[rs_harness::test]`.

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Expr, FnArg, ItemFn, Meta, Token, parse_macro_input};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let options = parse_macro_input!(args with HarnessTestOptions::parse);
    let function = parse_macro_input!(input as ItemFn);
    expand_harness_test(options, function).into()
}

#[derive(Default)]
struct HarnessTestOptions {
    advice_allow: bool,
    config: Option<Expr>,
    test_attrs: Vec<proc_macro2::TokenStream>,
}

impl HarnessTestOptions {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut options = Self::default();
        while !input.is_empty() {
            let meta = input.parse::<Meta>()?;
            match meta {
                Meta::Path(path) if path.is_ident("allow_advice") => {
                    options.advice_allow = true;
                }
                Meta::NameValue(name_value) if name_value.path.is_ident("advice") => {
                    options.advice_allow = matches_ident_expr(&name_value.value, "allow");
                }
                Meta::NameValue(name_value) if name_value.path.is_ident("config") => {
                    options.config = Some(name_value.value);
                }
                Meta::Path(path) if path.is_ident("ignore") => {
                    options.test_attrs.push(quote! { #[ignore] });
                }
                Meta::NameValue(name_value) if name_value.path.is_ident("ignore") => {
                    let value = name_value.value;
                    options.test_attrs.push(quote! { #[ignore = #value] });
                }
                Meta::Path(path) if path.is_ident("should_panic") => {
                    options.test_attrs.push(quote! { #[should_panic] });
                }
                Meta::List(list) if list.path.is_ident("should_panic") => {
                    let tokens = list.tokens;
                    options.test_attrs.push(quote! { #[should_panic(#tokens)] });
                }
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        "expected `allow_advice`, `advice = allow`, `config = <expr>`, `ignore`, or `should_panic`",
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(options)
    }
}

fn expand_harness_test(options: HarnessTestOptions, function: ItemFn) -> proc_macro2::TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    if sig.asyncness.is_some() {
        return quote_spanned! {sig.span()=>
            compile_error!("`#[rs_harness::test]` supports synchronous tests only");
        };
    }
    if !sig.inputs.is_empty() {
        let span = sig
            .inputs
            .iter()
            .next()
            .map_or_else(|| sig.span(), FnArg::span);
        return quote_spanned! {span=>
            compile_error!("`#[rs_harness::test]` test functions must not take arguments");
        };
    }
    let ident = sig.ident;
    let output = sig.output;
    let harness_call = harness_gate_call(&options);
    let test_attrs = options.test_attrs;
    quote! {
        #(#attrs)*
        #[test]
        #(#test_attrs)*
        #vis fn #ident() #output {
            #harness_call
            #block
        }
    }
}

fn harness_gate_call(options: &HarnessTestOptions) -> proc_macro2::TokenStream {
    match (&options.config, options.advice_allow) {
        (Some(config), true) => quote! {
            let __rs_harness_config = #config;
            rust_lang_project_harness::assert_rust_project_harness_clean_with_config(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &__rs_harness_config,
            );
        },
        (Some(config), false) => quote! {
            let __rs_harness_config = #config;
            rust_lang_project_harness::assert_rust_project_harness_cargo_test_clean_with_config(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &__rs_harness_config,
            );
        },
        (None, true) => quote! {
            rust_lang_project_harness::assert_rust_project_harness_clean(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            );
        },
        (None, false) => quote! {
            rust_lang_project_harness::assert_rust_project_harness_cargo_test_clean(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            );
        },
    }
}

fn matches_ident_expr(expr: &Expr, expected: &str) -> bool {
    matches!(expr, Expr::Path(path) if path.path.is_ident(expected))
}
