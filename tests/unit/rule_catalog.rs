use rust_lang_project_harness::{
    RustDiagnosticSeverity, rust_agent_policy_rules, rust_modularity_rules,
    rust_project_policy_rules, rust_rule_pack_descriptors, rust_syntax_rules,
};

#[test]
fn rule_pack_descriptors_expose_default_execution_order() {
    let packs = rust_rule_pack_descriptors();
    let pack_ids = packs.iter().map(|pack| pack.id).collect::<Vec<_>>();

    assert_eq!(
        pack_ids,
        vec![
            "rust.syntax",
            "rust.project_policy",
            "rust.modularity",
            "rust.agent_policy",
        ]
    );
    assert_eq!(packs[0].default_mode, "blocking");
    assert_eq!(packs[1].default_mode, "blocking");
    assert_eq!(packs[2].default_mode, "blocking");
    assert_eq!(packs[3].default_mode, "advisory");
    assert!(packs.iter().all(|pack| pack.version == "1"));
    assert!(
        packs
            .iter()
            .all(|pack| pack.domains.contains(&"rust") && !pack.domains.is_empty())
    );
}

#[test]
fn rule_catalogs_expose_stable_rule_ids() {
    let syntax_ids = rust_syntax_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(syntax_ids, vec!["RUST-SYN-R001"]);

    let agent_ids = rust_agent_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        agent_ids,
        vec![
            "AGENT-R001",
            "AGENT-R002",
            "AGENT-R003",
            "AGENT-R004",
            "AGENT-R005",
            "AGENT-R006",
            "AGENT-R007",
            "AGENT-R008",
            "AGENT-R009",
            "AGENT-R010",
            "AGENT-R011",
            "AGENT-R012",
            "AGENT-R013",
            "AGENT-R014",
            "AGENT-R015",
            "AGENT-R016",
            "AGENT-R017",
            "AGENT-R018",
            "AGENT-R019",
            "AGENT-R020",
            "AGENT-R021",
            "AGENT-R022",
            "AGENT-R023",
            "AGENT-R024",
            "AGENT-R025",
            "AGENT-R026",
            "AGENT-R027",
            "AGENT-R028",
            "AGENT-R029",
            "AGENT-R030",
            "AGENT-R031",
            "AGENT-R032",
            "AGENT-R033",
            "AGENT-R034",
        ]
    );

    let project_ids = rust_project_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        project_ids,
        vec![
            "RUST-PROJ-R001",
            "RUST-PROJ-R002",
            "RUST-PROJ-R003",
            "RUST-PROJ-R004",
            "RUST-PROJ-R005",
            "RUST-PROJ-R006",
            "RUST-PROJ-R007",
            "RUST-PROJ-R008",
            "RUST-PROJ-R009",
            "RUST-PROJ-R010",
            "RUST-PROJ-R011",
            "RUST-PROJ-R012",
            "RUST-PROJ-R013",
            "RUST-PROJ-R014",
            "RUST-PROJ-R015",
            "RUST-PROJ-R016",
            "RUST-PROJ-R017",
            "RUST-PROJ-R018",
            "RUST-PROJ-R019",
            "RUST-PROJ-R020",
            "RUST-PROJ-R021",
            "RUST-PROJ-R022",
        ]
    );

    let modularity_ids = rust_modularity_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        modularity_ids,
        vec![
            "RUST-MOD-R001",
            "RUST-MOD-R002",
            "RUST-MOD-R003",
            "RUST-MOD-R004",
            "RUST-MOD-R005",
            "RUST-MOD-R006",
            "RUST-MOD-R007",
            "RUST-MOD-R008",
            "RUST-MOD-R009",
            "RUST-MOD-R010",
            "RUST-MOD-R011",
        ]
    );
}

#[test]
fn rule_catalogs_keep_default_severities_aligned() {
    assert!(
        rust_syntax_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Error)
    );
    assert!(
        rust_project_policy_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Warning)
    );
    assert!(
        rust_modularity_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Warning)
    );
    assert!(
        rust_agent_policy_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Info)
    );
}
