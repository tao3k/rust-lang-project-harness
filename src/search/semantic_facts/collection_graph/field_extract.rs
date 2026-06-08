//! Collection field extraction and query matching.

use quote::ToTokens;

pub(super) struct CollectionField {
    pub(super) owner_path: String,
    pub(super) container_name: String,
    pub(super) field_name: String,
    pub(super) type_value: String,
    pub(super) type_args: String,
    pub(super) collection_kind: String,
    pub(super) element_shape: String,
    pub(super) line: usize,
}

impl CollectionField {
    pub(super) fn matches_query(&self, query: &str) -> bool {
        let terms = query_terms(query);
        if terms
            .iter()
            .any(|term| collection_term_constrains_kind(term))
            && !terms
                .iter()
                .any(|term| collection_term_matches_kind(term, &self.collection_kind))
        {
            return false;
        }
        let text = format!(
            "{} {} {} {} {} {} field fields type types collection collections list lists map maps set sets",
            self.container_name,
            self.field_name,
            self.type_value,
            self.type_args,
            self.collection_kind,
            self.element_shape,
        )
        .to_ascii_lowercase();
        terms
            .iter()
            .any(|term| text.contains(term.as_str()) || self.alias_matches(term))
    }

    fn alias_matches(&self, term: &str) -> bool {
        match term {
            "collection" | "collections" | "list" | "lists" => true,
            "map" | "maps" => self.collection_kind.ends_with("Map"),
            "set" | "sets" => self.collection_kind.ends_with("Set"),
            "field" | "fields" | "type" | "types" => true,
            "scalar" | "scalars" => self.element_shape == "scalar",
            _ => false,
        }
    }
}

fn collection_term_constrains_kind(term: &str) -> bool {
    matches!(
        term,
        "vec"
            | "vecdeque"
            | "hashmap"
            | "hashset"
            | "btreemap"
            | "btreeset"
            | "map"
            | "maps"
            | "set"
            | "sets"
            | "list"
            | "lists"
    )
}

fn collection_term_matches_kind(term: &str, collection_kind: &str) -> bool {
    let kind = collection_kind.to_ascii_lowercase();
    match term {
        "vec" => kind == "vec",
        "vecdeque" => kind == "vecdeque",
        "hashmap" => kind == "hashmap",
        "hashset" => kind == "hashset",
        "btreemap" => kind == "btreemap",
        "btreeset" => kind == "btreeset",
        "map" | "maps" => kind.ends_with("map"),
        "set" | "sets" => kind.ends_with("set"),
        "list" | "lists" => matches!(kind.as_str(), "vec" | "vecdeque"),
        _ => false,
    }
}

pub(super) fn collection_fields(owner_path: &str, syntax: &syn::File) -> Vec<CollectionField> {
    syntax
        .items
        .iter()
        .flat_map(|item| match item {
            syn::Item::Struct(item_struct) => struct_collection_fields(owner_path, item_struct),
            _ => Vec::new(),
        })
        .collect()
}

fn struct_collection_fields(
    owner_path: &str,
    item_struct: &syn::ItemStruct,
) -> Vec<CollectionField> {
    let syn::Fields::Named(fields) = &item_struct.fields else {
        return Vec::new();
    };
    let container_name = item_struct.ident.to_string();
    fields
        .named
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?.to_string();
            let collection = direct_collection_type(&field.ty)?;
            Some(CollectionField {
                owner_path: owner_path.to_string(),
                container_name: container_name.clone(),
                field_name,
                type_value: field.ty.to_token_stream().to_string(),
                type_args: collection.type_args.clone(),
                collection_kind: collection.kind.clone(),
                element_shape: collection.element_shape(),
                line: field.ident.as_ref()?.span().start().line.max(1),
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct CollectionType {
    kind: String,
    type_args: String,
}

impl CollectionType {
    fn element_shape(&self) -> String {
        match self.kind.as_str() {
            "Vec" | "VecDeque" => {
                if self.type_args.trim().is_empty() {
                    "unknown"
                } else if contains_collection_type(&self.type_args) {
                    "collection"
                } else {
                    "scalar"
                }
            }
            "HashMap" | "BTreeMap" => "key-value",
            "HashSet" | "BTreeSet" => "scalar",
            _ => "unknown",
        }
        .to_string()
    }
}

fn contains_collection_type(value: &str) -> bool {
    [
        "Vec", "VecDeque", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    ]
    .iter()
    .any(|name| value.contains(name))
}

fn direct_collection_type(ty: &syn::Type) -> Option<CollectionType> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let kind = segment.ident.to_string();
    if !matches!(
        kind.as_str(),
        "Vec" | "VecDeque" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
    ) {
        return None;
    }
    Some(CollectionType {
        kind,
        type_args: generic_args_text(&segment.arguments),
    })
}

fn generic_args_text(arguments: &syn::PathArguments) -> String {
    let syn::PathArguments::AngleBracketed(arguments) = arguments else {
        return String::new();
    };
    arguments
        .args
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|tokens| tokens.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !(character == '_' || character.is_ascii_alphanumeric()))
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
