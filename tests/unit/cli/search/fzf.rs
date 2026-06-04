#![allow(unused_imports)]

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_fzf_renders_fuzzy_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cli-search-fzf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub mod hook_runtime;\npub struct AgentHookEvent;\npub fn run_codex_agent_hook(_event: AgentHookEvent) {}\npub fn source_snapshot() { assert_snapshot!(\"src\"); }\n",
    )
    .expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create tests unit");
    fs::write(
        root.join("tests/unit/snapshot.rs"),
        "fn snapshot_case() { assert_snapshot!(\"ok\"); }\n",
    )
    .expect("write scoped test");

    let fzf = run_search(root, &["fzf", "runCodexAgentHook"]);
    assert!(
        fzf.starts_with("[search-fzf] q=runCodexAgentHook mode=fuzzy backend=provider pkg=. own=1"),
        "{fzf}"
    );
    assert!(fzf.contains("|owner src/lib.rs hit_kind=fzf"), "{fzf}");
    let path_fzf = run_search(root, &["fzf", "src/lib.rs", "--view", "seeds"]);
    assert!(path_fzf.contains("owner:path(src/lib.rs)"), "{path_fzf}");
    let scoped_fzf = run_search(
        root,
        &[
            "fzf",
            "assert_snapshot!",
            "owner",
            "tests/unit",
            "--view",
            "seeds",
        ],
    );
    assert!(
        scoped_fzf.contains("tests/unit/snapshot.rs"),
        "{scoped_fzf}"
    );
    assert!(
        scoped_fzf.contains("O=owner:path(tests/unit/snapshot.rs)!owner"),
        "{scoped_fzf}"
    );
    assert!(
        scoped_fzf.contains("T=test:path(tests/unit/snapshot.rs)!tests"),
        "{scoped_fzf}"
    );
    assert!(!scoped_fzf.contains("[search-graph]"), "{scoped_fzf}");

    let multi_scoped_fzf = run_search(
        root,
        &[
            "fzf",
            "assert_snapshot!",
            "owner",
            "src",
            "tests/unit",
            "--view",
            "seeds",
        ],
    );
    assert!(
        multi_scoped_fzf.contains("src/lib.rs"),
        "{multi_scoped_fzf}"
    );
    assert!(
        multi_scoped_fzf.contains("tests/unit/snapshot.rs"),
        "{multi_scoped_fzf}"
    );

    let cwd_discovered_output = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .current_dir(root)
        .args([
            "search",
            "fzf",
            "assert_snapshot!",
            "owner",
            "src",
            "tests/unit",
            "--view",
            "seeds",
        ])
        .output()
        .expect("run cli");
    assert!(
        cwd_discovered_output.status.success(),
        "{cwd_discovered_output:?}"
    );
    let cwd_discovered_fzf = normalize_temp_root(
        &String::from_utf8(cwd_discovered_output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        cwd_discovered_fzf.contains("src/lib.rs"),
        "{cwd_discovered_fzf}"
    );
    assert!(
        cwd_discovered_fzf.contains("tests/unit/snapshot.rs"),
        "{cwd_discovered_fzf}"
    );
    assert!(
        cwd_discovered_fzf.contains("O=owner:path(src/lib.rs)!owner"),
        "{cwd_discovered_fzf}"
    );
    assert!(
        cwd_discovered_fzf.contains("T=test:path(tests/unit/snapshot.rs)!tests"),
        "{cwd_discovered_fzf}"
    );

    let query_set = run_search(
        root,
        &[
            "fzf",
            "--query-set",
            "AgentHookEvent",
            "--query-set",
            "runCodexAgentHook",
            "--view",
            "seeds",
        ],
    );
    assert!(
query_set.starts_with(
"[search-fzf] q=AgentHookEvent,runCodexAgentHook querySet=2 selector=fuzzy-set alg=change-frontier-query-set"
),
"{query_set}"
);
    assert!(
        query_set.contains("O=owner:path(src/lib.rs)!owner"),
        "{query_set}"
    );
    assert!(query_set.contains("src/lib.rs"), "{query_set}");

    let fuzzy_acronym = run_search(root, &["fzf", "rCAH", "--view", "seeds"]);
    assert!(fuzzy_acronym.contains("src/lib.rs"), "{fuzzy_acronym}");
    let exact_acronym = run_search(
        root,
        &["fzf", "rCAH", "--view", "seeds", "--fzf-arg", "--exact"],
    );
    assert!(
        exact_acronym.starts_with("[search-fzf] q=rCAH alg=native-syntax-query"),
        "{exact_acronym}"
    );
    assert!(
        !exact_acronym.contains("owner:src/lib.rs!owner"),
        "{exact_acronym}"
    );
    assert!(!exact_acronym.contains("|seed "), "{exact_acronym}");
    let boundary_exact = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "rCAH".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
        "--fzf".as_ref(),
        "--exact".as_ref(),
    ]);
    assert!(boundary_exact.status.success(), "{boundary_exact:?}");
    let boundary_stdout_raw = String::from_utf8(boundary_exact.stdout).expect("utf8 stdout");
    let boundary_stdout = normalize_temp_root(&boundary_stdout_raw, root);
    assert!(
        boundary_stdout.starts_with("[search-fzf] q=rCAH alg=native-syntax-query"),
        "{boundary_stdout}"
    );

    let exact_json = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "runCodexAgentHook".as_ref(),
        "--json".as_ref(),
        "--fzf-arg".as_ref(),
        "--exact".as_ref(),
        root.as_os_str(),
    ]);
    assert!(exact_json.status.success(), "{exact_json:?}");
    let value = serde_json::from_slice::<Value>(&exact_json.stdout).expect("fzf exact json");
    assert_eq!(value["finder"]["engine"], "fzf");
    assert_eq!(value["finder"]["surface"], "search-fzf");
    assert_eq!(value["finder"]["options"]["matchMode"], "exact");
    assert_eq!(value["finder"]["options"]["nativeArgs"][0], "--exact");

    let rejected_fzf = run_cli([
        "search".as_ref(),
        "fzf".as_ref(),
        "runCodexAgentHook".as_ref(),
        "--fzf-arg".as_ref(),
        "--preview".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!rejected_fzf.status.success(), "{rejected_fzf:?}");
    let stderr = String::from_utf8(rejected_fzf.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("unsupported fzf option for agent search: --preview"),
        "{stderr}"
    );

    let text = run_cli([
        "search".as_ref(),
        "text".as_ref(),
        "runCodexAgentHook".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!text.status.success(), "{text:?}");
    let stderr = String::from_utf8(text.stderr).expect("utf8 stderr");
    assert!(stderr.contains("unknown search view: text"), "{stderr}");
    let dash_positional = run_search(root, &["fzf", "--json", "--view", "seeds"]);
    assert!(
        dash_positional.starts_with("[search-fzf] q=--json"),
        "{dash_positional}"
    );
    let dash_query_flag = run_search(
        root,
        &[
            "fzf",
            "--query",
            "--language",
            "owner",
            "tests",
            "--view",
            "seeds",
        ],
    );
    assert!(
        dash_query_flag.starts_with("[search-fzf] q=--language"),
        "{dash_query_flag}"
    );
}
