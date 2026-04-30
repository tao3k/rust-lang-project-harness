//! Shared rule-pack support helpers.

use std::path::{Path, PathBuf};

use syn::punctuated::Punctuated;
use syn::{Attribute, Meta, Token};

pub(crate) fn labels(
    domain: &'static str,
) -> std::collections::BTreeMap<&'static str, &'static str> {
    std::collections::BTreeMap::from([("language", "rust"), ("domain", domain)])
}

pub(crate) fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}

pub(crate) fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

pub(crate) fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let Meta::List(list) = &attr.meta else {
        return false;
    };
    cfg_list_has_test(list)
}

fn cfg_list_has_test(list: &syn::MetaList) -> bool {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let Ok(items) = list.parse_args_with(parser) else {
        return false;
    };
    items.iter().any(cfg_meta_has_test)
}

fn cfg_meta_has_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("not") => false,
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            cfg_list_has_test(list)
        }
        Meta::List(_) | Meta::NameValue(_) => false,
    }
}
