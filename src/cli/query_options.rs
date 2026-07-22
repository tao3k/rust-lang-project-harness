use std::ffi::OsString;

#[derive(Debug, Default)]
pub(super) struct QueryOptions {
    pub(super) selector: Option<String>,
    pub(super) names_only: bool,
    pub(super) code: bool,
    pub(super) json: bool,
    pub(super) help: bool,
}

impl QueryOptions {
    pub(super) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let value = arg
                .into_string()
                .map_err(|_| "expected UTF-8 query arguments".to_string())?;
            match value.as_str() {
                "--help" | "-h" => options.help = true,
                "--json" => options.json = true,
                "--names-only" => options.names_only = true,
                "--code" => options.code = true,
                "--selector" => {
                    let selector = next_value(&mut args, "--selector")?;
                    if options.selector.replace(selector).is_some() {
                        return Err("query accepts exactly one --selector".to_string());
                    }
                }
                "--workspace" | "--from-hook" => {
                    let _ = next_value(&mut args, value.as_str())?;
                }
                "--query" | "--term" | "--surface" | "--pipe" | "--package" | "--view"
                | "--seeds" | "--source" => {
                    return Err(owner_discovery_error());
                }
                "--" => {
                    return Err(owner_discovery_error());
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown query option: {value}"));
                }
                _ => return Err(owner_discovery_error()),
            }
        }
        if options.names_only && options.code {
            return Err("query --names-only and --code cannot be combined".to_string());
        }
        Ok(options)
    }
}

fn next_value(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("expected value after {option}"))?
        .into_string()
        .map_err(|_| format!("expected UTF-8 value after {option}"))
}

fn owner_discovery_error() -> String {
    "rust query requires an exact --selector; use `asp rust search owner <owner-path> items --query <symbol> --names-only --workspace .` for owner or symbol discovery"
        .to_string()
}
