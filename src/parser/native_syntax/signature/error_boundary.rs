use super::facts::path_segments_text;

pub(super) fn application_error_return_type(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let terminal = type_path.path.segments.last()?;
    if terminal.ident != "Result" {
        return None;
    }
    let path_text = path_segments_text(&type_path.path);
    if is_application_result_path(&path_text) {
        return Some(path_text);
    }
    let err_type = result_error_type(terminal)?;
    application_error_type_name(err_type).map(|err_name| format!("Result<_, {err_name}>"))
}

fn result_error_type(segment: &syn::PathSegment) -> Option<&syn::Type> {
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let mut types = args.args.iter().filter_map(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return None;
        };
        Some(ty)
    });
    types.next()?;
    types.next()
}

fn application_error_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) if is_application_error_path(&type_path.path) => {
            Some(path_segments_text(&type_path.path))
        }
        syn::Type::Path(type_path) if is_boxed_dyn_error_path(type_path) => {
            Some("Box<dyn Error>".to_owned())
        }
        syn::Type::TraitObject(trait_object) if trait_object_contains_error(trait_object) => {
            Some("dyn Error".to_owned())
        }
        _ => None,
    }
}

fn is_application_result_path(path_text: &str) -> bool {
    matches!(
        path_text,
        "anyhow::Result" | "eyre::Result" | "color_eyre::Result" | "color_eyre::eyre::Result"
    )
}

fn is_application_error_path(path: &syn::Path) -> bool {
    matches!(
        path_segments_text(path).as_str(),
        "anyhow::Error"
            | "eyre::Report"
            | "eyre::Error"
            | "color_eyre::Report"
            | "color_eyre::eyre::Report"
    )
}

fn is_boxed_dyn_error_path(type_path: &syn::TypePath) -> bool {
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Box" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        let syn::GenericArgument::Type(syn::Type::TraitObject(trait_object)) = arg else {
            return false;
        };
        trait_object_contains_error(trait_object)
    })
}

fn trait_object_contains_error(trait_object: &syn::TypeTraitObject) -> bool {
    trait_object.bounds.iter().any(|bound| {
        let syn::TypeParamBound::Trait(trait_bound) = bound else {
            return false;
        };
        trait_path_is_error_boundary(&trait_bound.path)
    })
}

fn trait_path_is_error_boundary(path: &syn::Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "Error")
        && (path.segments.len() == 1 || path_segments_text(path) == "std::error::Error")
}
