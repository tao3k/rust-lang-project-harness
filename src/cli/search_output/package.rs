pub(in crate::cli::search_output) struct PackageHeaderParts {
    pub(in crate::cli::search_output) prefix: String,
    pub(in crate::cli::search_output) package: String,
    pub(in crate::cli::search_output) suffix: String,
}

pub(in crate::cli::search_output) fn package_header_parts(
    line: &str,
) -> Option<PackageHeaderParts> {
    let mut cursor = 0;
    for field in line.split_whitespace() {
        let field_start = cursor + line[cursor..].find(field)?;
        let field_end = field_start + field.len();
        cursor = field_end;
        if let Some(package) = field.strip_prefix("pkg=") {
            return Some(PackageHeaderParts {
                prefix: line[..field_start].to_string(),
                package: package.to_string(),
                suffix: line[field_end..].to_string(),
            });
        }
    }
    None
}
