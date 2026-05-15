use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::parser::RustReasoningModuleFacts;

pub(super) struct RustVerificationModuleLookup<'a> {
    absolute: BTreeMap<PathBuf, &'a RustReasoningModuleFacts>,
    project_relative: BTreeMap<PathBuf, &'a RustReasoningModuleFacts>,
    package_relative: BTreeMap<PathBuf, &'a RustReasoningModuleFacts>,
}

impl<'a> RustVerificationModuleLookup<'a> {
    pub(super) fn new(
        project_root: &Path,
        package_root: &Path,
        modules: impl IntoIterator<Item = &'a RustReasoningModuleFacts>,
    ) -> Self {
        let mut lookup = Self {
            absolute: BTreeMap::new(),
            project_relative: BTreeMap::new(),
            package_relative: BTreeMap::new(),
        };
        for module in modules {
            lookup.absolute.entry(module.path.clone()).or_insert(module);
            if let Ok(relative_path) = module.path.strip_prefix(project_root) {
                lookup
                    .project_relative
                    .entry(relative_path.to_path_buf())
                    .or_insert(module);
            }
            if let Ok(relative_path) = module.path.strip_prefix(package_root) {
                lookup
                    .package_relative
                    .entry(relative_path.to_path_buf())
                    .or_insert(module);
            }
        }
        lookup
    }

    pub(super) fn get_config_path(
        &self,
        owner_path: &Path,
    ) -> Option<&'a RustReasoningModuleFacts> {
        if owner_path.is_absolute() {
            return self.absolute.get(owner_path).copied();
        }
        self.project_relative
            .get(owner_path)
            .or_else(|| self.package_relative.get(owner_path))
            .copied()
    }
}
