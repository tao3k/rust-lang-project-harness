//! Parser-owned callable signature facts and contract-type projections.

mod callable;
mod contract_type;
mod error_boundary;
mod facts;

pub(crate) use callable::{
    public_function_dynamic_json_api_syntax, public_function_param_syntax,
    public_function_return_syntax, public_function_tuple_api_syntax,
};
pub(crate) use facts::{
    RustFunctionDynamicJsonApiSyntax, RustFunctionParamSyntax, RustFunctionReturnSyntax,
    RustFunctionTupleApiSyntax,
};
