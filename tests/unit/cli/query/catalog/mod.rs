use std::ffi::OsString;
use std::path::Path;

mod enrichment;
mod layout;
mod packet;
mod stdout;

const FUNCTION_NAME_QUERY: &str = "(function_item name: (identifier) @function.name)";

fn function_name_query_args(root: &Path, extra_args: &[&str]) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("query"),
        OsString::from("--treesitter-query"),
        OsString::from(FUNCTION_NAME_QUERY),
    ];
    args.extend(extra_args.iter().map(OsString::from));
    args.extend([
        OsString::from("--workspace"),
        root.as_os_str().to_os_string(),
    ]);
    args.extend([
        OsString::from("--asp-syntax-query-captures"),
        OsString::from("function.name"),
        OsString::from("--asp-syntax-query-node-types"),
        OsString::from("function_item,identifier"),
        OsString::from("--asp-syntax-query-fields"),
        OsString::from("name"),
    ]);
    args
}
