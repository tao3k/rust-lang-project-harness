//! Small Rust-standard import taxonomy; project dependencies are configured.

use std::collections::BTreeSet;

use crate::verification::RustOwnerResponsibility;

pub(super) fn standard_import_responsibilities(
    segments: &[String],
) -> BTreeSet<RustOwnerResponsibility> {
    let mut responsibilities = BTreeSet::new();
    if matches!(segments, [first, second, ..] if first == "std" && second == "fs") {
        responsibilities.insert(RustOwnerResponsibility::ExternalDependency);
        responsibilities.insert(RustOwnerResponsibility::Persistence);
    }
    responsibilities
}
