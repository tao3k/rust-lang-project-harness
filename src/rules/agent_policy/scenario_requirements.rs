use super::pack::RustPolicyScenarioRequirement;
use super::{
    RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1, RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1,
    RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1,
    RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1,
    RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1, RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1,
    RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1,
    RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1, RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1,
    RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1, RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1,
    RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1, RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1,
    RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1, RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1,
    RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1, RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1,
    RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1, RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1,
    RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1, RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1,
    RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1, RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1,
    RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1, RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1,
    RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1, RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1,
    RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1, RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1,
    RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1, RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1,
    RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1,
    RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1,
    RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1, RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1,
    RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1, RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1,
    RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1, RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1,
};
use crate::rules::project_policy::RUST_PROJ_R023;

macro_rules! policy_scenario_requirements {
    ($(($rule_id:expr, $scenario_id:expr, $policy_id:expr, $scenario_root:expr)),+ $(,)?) => {
        &[$(policy_scenario_requirement($rule_id, $scenario_id, $policy_id, $scenario_root)),+]
    };
}

const POLICY_SCENARIO_REQUIREMENTS: &[RustPolicyScenarioRequirement] = policy_scenario_requirements![
    (
        RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1,
        "control-flow-v1",
        "RUST-AGENT-CFG-001",
        "tests/unit/scenarios/software_criteria/control_flow_v1"
    ),
    (
        RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1,
        "data-structure-linear-membership-scan-v1",
        "RUST-AGENT-DS-001",
        "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1,
        "async-blocking-boundary-v1",
        "RUST-AGENT-ASYNC-BLOCKING-001",
        "tests/unit/scenarios/software_criteria/async_blocking_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1,
        "async-sync-lock-boundary-v1",
        "RUST-AGENT-ASYNC-SYNC-LOCK-001",
        "tests/unit/scenarios/software_criteria/async_sync_lock_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1,
        "async-backpressure-boundary-v1",
        "RUST-AGENT-ASYNC-BACKPRESSURE-001",
        "tests/unit/scenarios/software_criteria/async_backpressure_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1,
        "async-select-cancellation-safety-v1",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-001",
        "tests/unit/scenarios/software_criteria/async_select_cancellation_safety_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1,
        "async-timeout-cancellation-safety-v1",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-002",
        "tests/unit/scenarios/software_criteria/async_timeout_cancellation_safety_v1"
    ),
    (
        RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1,
        "async-task-lifecycle-boundary-v1",
        "RUST-AGENT-ASYNC-TASK-LIFECYCLE-001",
        "tests/unit/scenarios/software_criteria/async_task_lifecycle_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1,
        "public-dynamic-json-api-boundary-v1",
        "RUST-AGENT-API-SHAPE-036",
        "tests/unit/scenarios/software_criteria/public_dynamic_json_api_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1,
        "process-command-probe-v1",
        "RUST-AGENT-PROC-001",
        "tests/unit/scenarios/software_criteria/process_command_probe_v1"
    ),
    (
        RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1,
        "tokio-runtime-boundary-v1",
        "RUST-AGENT-TOKIO-RUNTIME-002",
        "tests/unit/scenarios/software_criteria/tokio_runtime_boundary_v1"
    ),
    (
        RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1,
        "native-abi-contract-surface-v1",
        "RUST-AGENT-NATIVE-ABI-001",
        "tests/unit/scenarios/software_criteria/native_abi_contract_surface_v1"
    ),
    (
        RUST_PROJ_R023,
        "rust-package-edition-2024-v1",
        "RUST-AGENT-PROJECT-MANIFEST-023",
        "tests/unit/scenarios/software_criteria/rust_package_edition_2024_v1"
    ),
];

pub(crate) fn rust_agent_policy_scenario_requirements() -> &'static [RustPolicyScenarioRequirement]
{
    POLICY_SCENARIO_REQUIREMENTS
}

const fn policy_scenario_requirement(
    rule_id: &'static str,
    scenario_id: &'static str,
    policy_id: &'static str,
    scenario_root: &'static str,
) -> RustPolicyScenarioRequirement {
    RustPolicyScenarioRequirement {
        rule_id,
        scenario_id,
        policy_id,
        scenario_root,
    }
}
