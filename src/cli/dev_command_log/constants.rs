pub(super) const SCHEMA_ID: &str = "agent.semantic-protocols.dev-command-log";
pub(super) const SCHEMA_VERSION: &str = "1";
pub(super) const PROTOCOL_ID: &str = "agent.semantic-protocols.semantic-language";
pub(super) const PROTOCOL_VERSION: &str = "1";
pub(super) const SECRET_FLAGS: &[&str] =
    &["--api-key", "--apikey", "--password", "--secret", "--token"];
pub(super) const PROJECT_ANCHORS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pnpm-lock.yaml",
    "pyproject.toml",
    "Project.toml",
    ".git",
];
pub(super) const VALUE_OPTIONS: &[&str] = &[
    "--from-hook",
    "--package",
    "--query",
    "--query-set",
    "--selector",
    "--term",
    "--view",
];
pub(super) const SEARCH_PIPES: &[&str] = &[
    "dependency",
    "docs",
    "features",
    "items",
    "owner",
    "owners",
    "prime",
    "symbol",
    "tests",
    "workspace",
];
