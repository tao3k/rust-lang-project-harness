//! Data-only callable signature facts shared by parser-owned projections.

pub(super) fn path_segments_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionParamSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub param_name: String,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionReturnSyntax {
    pub line: usize,
    pub function_name: String,
    pub type_text: String,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub receiver: Option<String>,
    pub impl_type: Option<String>,
    pub trait_path: Option<String>,
    pub application_error_boundary: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionTupleApiSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub surface_name: String,
    pub type_text: String,
    pub element_contract_types: Vec<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionDynamicJsonApiSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub surface_name: String,
    pub type_text: String,
    pub json_type_name: String,
    pub is_test_context: bool,
}
