//! Schema metadata for verification report manifests.

use serde::{Deserialize, Serialize};

/// Stable schema identifier for modular verification report manifests.
pub const RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID: &str = "rust_verification_report_manifest";
/// Current modular verification report manifest schema version.
pub const RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION: &str = "1";

/// Schema metadata embedded in every modular verification report manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportManifestSchema {
    /// Stable manifest schema identifier.
    pub schema_id: String,
    /// Stable manifest schema version.
    pub schema_version: String,
}

/// Compatibility result for a verification report manifest schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum RustVerificationReportManifestCompatibility {
    /// The manifest schema is supported by this harness version.
    Supported,
    /// The manifest uses an unknown schema id.
    UnsupportedSchemaId {
        /// Schema id supported by this harness version.
        expected: String,
        /// Schema id found in the manifest.
        actual: String,
    },
    /// The manifest uses a known schema id with an unsupported version.
    UnsupportedSchemaVersion {
        /// Schema version supported by this harness version.
        expected: String,
        /// Schema version found in the manifest.
        actual: String,
    },
}

impl Default for RustVerificationReportManifestSchema {
    fn default() -> Self {
        Self {
            schema_id: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID.to_string(),
            schema_version: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION.to_string(),
        }
    }
}

impl RustVerificationReportManifestSchema {
    /// Return the compact `schema_id/schema_version` label.
    #[must_use]
    pub fn compact_label(&self) -> String {
        format!("{}/{}", self.schema_id, self.schema_version)
    }

    /// Check whether this schema is supported by the current harness version.
    #[must_use]
    pub fn compatibility(&self) -> RustVerificationReportManifestCompatibility {
        check_rust_verification_report_manifest_schema(self)
    }
}

impl RustVerificationReportManifestCompatibility {
    /// Return whether the manifest schema is supported.
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }

    /// Return a stable lowercase state label.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::UnsupportedSchemaId { .. } => "unsupported_schema_id",
            Self::UnsupportedSchemaVersion { .. } => "unsupported_schema_version",
        }
    }

    /// Return the stable Agent action for this compatibility state.
    #[must_use]
    pub const fn action(&self) -> &'static str {
        if self.is_supported() {
            "read_manifest_payloads"
        } else {
            "stop_and_refresh_harness_contract"
        }
    }

    /// Return a compact human-readable reason for unsupported manifests.
    #[must_use]
    pub const fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Supported => None,
            Self::UnsupportedSchemaId { .. } => Some("unsupported manifest schema id"),
            Self::UnsupportedSchemaVersion { .. } => Some("unsupported manifest schema version"),
        }
    }
}

/// Check whether a report manifest schema is supported by this harness version.
#[must_use]
pub fn check_rust_verification_report_manifest_schema(
    schema: &RustVerificationReportManifestSchema,
) -> RustVerificationReportManifestCompatibility {
    if schema.schema_id != RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID {
        return RustVerificationReportManifestCompatibility::UnsupportedSchemaId {
            expected: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID.to_string(),
            actual: schema.schema_id.clone(),
        };
    }
    if schema.schema_version != RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION {
        return RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion {
            expected: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION.to_string(),
            actual: schema.schema_version.clone(),
        };
    }
    RustVerificationReportManifestCompatibility::Supported
}

/// Render compact compatibility advice for Agents.
#[must_use]
pub fn render_rust_verification_report_manifest_compatibility(
    compatibility: &RustVerificationReportManifestCompatibility,
) -> String {
    match compatibility {
        RustVerificationReportManifestCompatibility::Supported => {
            format!(
                "[verify-report-manifest] state={} action={}",
                compatibility.as_str(),
                compatibility.action()
            )
        }
        RustVerificationReportManifestCompatibility::UnsupportedSchemaId { expected, actual } => {
            format!(
                "[verify-report-manifest] state={} expected={} actual={} reason=\"{}\" action={}",
                compatibility.as_str(),
                expected,
                actual,
                compatibility.reason().expect("unsupported reason"),
                compatibility.action()
            )
        }
        RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion {
            expected,
            actual,
        } => {
            format!(
                "[verify-report-manifest] state={} expected={} actual={} reason=\"{}\" action={}",
                compatibility.as_str(),
                expected,
                actual,
                compatibility.reason().expect("unsupported reason"),
                compatibility.action()
            )
        }
    }
}

/// Render compact schema compatibility advice for Agents.
#[must_use]
pub fn render_rust_verification_report_manifest_schema_compatibility(
    schema: &RustVerificationReportManifestSchema,
) -> String {
    let compatibility = schema.compatibility();
    match &compatibility {
        RustVerificationReportManifestCompatibility::Supported => {
            format!(
                "[verify-report-manifest] state={} schema={} action={}",
                compatibility.as_str(),
                schema.compact_label(),
                compatibility.action()
            )
        }
        RustVerificationReportManifestCompatibility::UnsupportedSchemaId { .. }
        | RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion { .. } => {
            render_rust_verification_report_manifest_compatibility(&compatibility)
        }
    }
}
