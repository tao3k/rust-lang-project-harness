//! Agent policy rules derived from public data-shape facts.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax, path_line_location, source_line,
};
use crate::{RustHarnessFinding, RustHarnessRule};

use crate::rules::display_path;

use super::{AGENT_R020, AGENT_R021, AGENT_R022, AGENT_R024};

const MIN_SEMANTIC_PRIMITIVE_FIELDS: usize = 3;
const MIN_ENUM_VARIANT_SEMANTIC_PRIMITIVE_FIELDS: usize = 2;
const MIN_ENUM_TUPLE_VARIANT_SEMANTIC_PRIMITIVE_FIELDS: usize = 2;

pub(super) fn data_shape_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(public_data_struct_primitive_field_findings(module, rules));
    findings.extend(public_enum_variant_primitive_payload_findings(
        module, rules,
    ));
    findings.extend(public_type_generic_bound_findings(module, rules));
    findings.extend(public_enum_tuple_variant_payload_findings(module, rules));
    findings
}

fn public_data_struct_primitive_field_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut fields_by_struct = BTreeMap::<(usize, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_struct_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_data_field_contract_type(field) else {
            continue;
        };
        fields_by_struct
            .entry((field.struct_line, field.struct_name.clone()))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }

    let rule = &rules[AGENT_R020];
    fields_by_struct
        .into_iter()
        .filter_map(|((struct_line, struct_name), mut fields)| {
            if fields.len() < MIN_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public data struct `{struct_name}` with primitive semantic fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, struct_line),
                source_line(&module.source, struct_line),
                "wrap these fields in named domain types or make the raw DTO boundary explicit",
            ))
        })
        .collect()
}

fn public_enum_variant_primitive_payload_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut fields_by_variant = BTreeMap::<(usize, String, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_enum_variant_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_enum_payload_field_contract_type(field) else {
            continue;
        };
        fields_by_variant
            .entry((
                field.variant_line,
                field.enum_name.clone(),
                field.variant_name.clone(),
            ))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }

    let rule = &rules[AGENT_R021];
    fields_by_variant
        .into_iter()
        .filter_map(|((variant_line, enum_name, variant_name), mut fields)| {
            if fields.len() < MIN_ENUM_VARIANT_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public enum `{enum_name}` variant `{variant_name}` with primitive semantic payload fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, variant_line),
                source_line(&module.source, variant_line),
                "wrap this variant payload in named domain types or move it into a named payload struct",
            ))
        })
        .collect()
}

fn public_type_generic_bound_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut bounds_by_type = BTreeMap::<(usize, &'static str, String), Vec<(usize, String)>>::new();
    for bound in &module.syntax_facts.public_type_generic_bounds {
        if bound.is_test_context {
            continue;
        }
        bounds_by_type
            .entry((bound.type_line, bound.type_kind, bound.type_name.clone()))
            .or_default()
            .push((
                bound.line,
                format!("{}: {}", bound.param_name, bound.bound_name),
            ));
    }

    let rule = &rules[AGENT_R022];
    bounds_by_type
        .into_iter()
        .map(|((type_line, type_kind, type_name), mut bounds)| {
            bounds.sort_by_key(|(line, _)| *line);
            bounds.dedup();
            let bound_list = bounds
                .into_iter()
                .map(|(_, bound)| bound)
                .collect::<Vec<_>>()
                .join(", ");
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public {type_kind} `{type_name}` with duplicated data-type bounds: {bound_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, type_line),
                source_line(&module.source, type_line),
                "move these bounds to derived impls, inherent impls, or methods that actually require them",
            )
        })
        .collect()
}

fn public_enum_tuple_variant_payload_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut fields_by_variant = BTreeMap::<(usize, String, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_enum_tuple_variant_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_enum_tuple_payload_field_contract_type(field) else {
            continue;
        };
        fields_by_variant
            .entry((
                field.variant_line,
                field.enum_name.clone(),
                field.variant_name.clone(),
            ))
            .or_default()
            .push((
                field.line,
                format!("#{}: {contract_type}", field.field_index + 1),
            ));
    }

    let rule = &rules[AGENT_R024];
    fields_by_variant
        .into_iter()
        .filter_map(|((variant_line, enum_name, variant_name), mut fields)| {
            if fields.len() < MIN_ENUM_TUPLE_VARIANT_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} exposes public enum `{enum_name}` tuple variant `{variant_name}` with anonymous primitive payload fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, variant_line),
                source_line(&module.source, variant_line),
                "replace the tuple variant payload with named fields, a named payload struct, or domain newtypes",
            ))
        })
        .collect()
}

fn public_data_field_contract_type(field: &RustPublicStructFieldSyntax) -> Option<&str> {
    if is_semantic_public_data_field(&field.field_name) {
        return field
            .primitive_contract_type
            .as_deref()
            .or(field.flag_contract_type.as_deref());
    }
    is_public_flag_field(&field.field_name)
        .then_some(field.flag_contract_type.as_deref())
        .flatten()
}

fn public_enum_payload_field_contract_type(
    field: &RustPublicEnumVariantFieldSyntax,
) -> Option<&str> {
    if is_semantic_public_data_field(&field.field_name) {
        return field
            .primitive_contract_type
            .as_deref()
            .or(field.flag_contract_type.as_deref());
    }
    is_public_flag_field(&field.field_name)
        .then_some(field.flag_contract_type.as_deref())
        .flatten()
}

fn public_enum_tuple_payload_field_contract_type(
    field: &RustPublicEnumTupleVariantFieldSyntax,
) -> Option<&str> {
    field
        .primitive_contract_type
        .as_deref()
        .or(field.flag_contract_type.as_deref())
}

fn is_semantic_public_data_field(name: &str) -> bool {
    is_semantic_identifier(name)
        || matches!(name, "path" | "url" | "uri" | "key" | "token")
        || name.ends_with("_path")
        || name.ends_with("_url")
        || name.ends_with("_uri")
        || name.ends_with("_key")
        || name.ends_with("_token")
        || name.ends_with("_ms")
        || name.ends_with("_secs")
        || name.ends_with("_seconds")
        || name.ends_with("_bytes")
}

fn is_semantic_identifier(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}

fn is_public_flag_field(name: &str) -> bool {
    matches!(name, "enabled" | "disabled")
        || name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("allow_")
        || name.starts_with("include_")
        || name.starts_with("should_")
}
