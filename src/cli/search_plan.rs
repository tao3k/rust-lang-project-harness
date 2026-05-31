//! Recipe-aware compact search plans for `rs-harness search --explain`.

pub(super) struct SearchPlanOptions<'a> {
    pub(super) view: &'a str,
    pub(super) query: Option<&'a str>,
    pub(super) output_view: Option<&'a str>,
    pub(super) depth: Option<usize>,
    pub(super) dir: Option<&'a str>,
    pub(super) edges: &'a [String],
    pub(super) pipes: &'a [String],
}

pub(super) fn render_search_plan(options: SearchPlanOptions<'_>) -> String {
    let mut rendered = format!(
        "[search-plan] view={} q={} mode={} depth={} dir={} edge={} pipes={}\n|step resolve-project\n|step render:{}\n",
        options.view,
        options.query.unwrap_or("-"),
        options.output_view.unwrap_or("graph"),
        options
            .depth
            .map(|depth| depth.to_string())
            .unwrap_or_else(|| "1".to_string()),
        options.dir.unwrap_or("-"),
        if options.edges.is_empty() {
            "-".to_string()
        } else {
            options.edges.join(",")
        },
        if options.pipes.is_empty() {
            "-".to_string()
        } else {
            options.pipes.join(",")
        },
        options.view
    );
    append_search_recipe_plan(&mut rendered, options);
    rendered
}

fn append_search_recipe_plan(rendered: &mut String, options: SearchPlanOptions<'_>) {
    match (options.view, options.query) {
        ("dependency", Some(query)) => append_dependency_plan(rendered, query),
        ("deps", Some(query)) => append_dependency_api_plan(rendered, query),
        ("features", Some(query)) => append_feature_plan(rendered, query),
        ("owner", Some(query)) => append_owner_plan(rendered, query),
        ("text" | "symbol" | "callsite" | "import", Some(query)) => {
            append_unknown_scope_plan(rendered, options.view, query);
        }
        _ => append_default_plan(rendered),
    }
}

fn append_dependency_plan(rendered: &mut String, query: &str) {
    rendered.push_str("|recipe dependency-change focus=multi-pipe token=final-only\n");
    rendered.push_str(&format!(
        "|prefer search:dependency:{query}(items,public-api,docs,tests)\n"
    ));
    rendered.push_str(&format!("|subagent deps=search:deps:{query}[::api]\n"));
    rendered.push_str(&format!(
        "|subagent source=search:dependency:{query}(owners,items)\n"
    ));
    rendered.push_str(&format!(
        "|subagent tests=search:dependency:{query}(tests)\n"
    ));
    rendered.push_str(&format!("|fallback ingest=rg-n:{query}(scope=src,tests)\n"));
    rendered.push_str("|after check:changed\n");
    rendered.push_str("|budget commands=3 rounds=2 output=bounded\n");
}

fn append_dependency_api_plan(rendered: &mut String, query: &str) {
    let dependency = dependency_root_from_query(query);
    let api = api_query_from_dependency_query(query).unwrap_or("api");
    rendered.push_str("|recipe dependency-api-docs focus=versioned-usage token=bounded\n");
    rendered.push_str(&format!("|prefer search:deps:{query}\n"));
    rendered.push_str(&format!(
        "|subagent public=search:dependency:{dependency}(public-api)\n"
    ));
    rendered.push_str(&format!("|subagent docs=search:docs:{dependency}::{api}\n"));
    rendered.push_str(&format!(
        "|subagent tests=search:ingest:rg-n:{api}(scope=tests)\n"
    ));
    rendered.push_str("|after check:changed\n");
    rendered.push_str("|budget commands=4 rounds=2 output=bounded\n");
}

fn append_feature_plan(rendered: &mut String, query: &str) {
    rendered.push_str("|recipe feature-cfg focus=feature-owner-tests token=final-only\n");
    rendered.push_str(&format!(
        "|prefer search:features:{query}(cfg,owners,tests)\n"
    ));
    rendered.push_str(&format!("|subagent cfg=search:cfg:{query}\n"));
    rendered.push_str(&format!("|fallback ingest=rg-n:{query}(scope=src,tests)\n"));
    rendered.push_str("|after check:changed\n");
    rendered.push_str("|budget commands=2 rounds=2 output=bounded\n");
}

fn append_owner_plan(rendered: &mut String, query: &str) {
    rendered.push_str("|recipe owner-edit focus=owner-items-tests token=bounded\n");
    rendered.push_str(&format!("|prefer search:owner:{query}(items)\n"));
    rendered.push_str(&format!("|subagent tests=search:tests:{query}\n"));
    rendered.push_str("|after check:changed\n");
    rendered.push_str("|budget commands=2 rounds=2 output=bounded\n");
}

fn append_unknown_scope_plan(rendered: &mut String, view: &str, query: &str) {
    rendered.push_str("|recipe unknown-scope focus=bounded-candidates token=bounded\n");
    rendered.push_str(&format!("|prefer search:{view}:{query}\n"));
    rendered.push_str(&format!("|fallback ingest=rg-n:{query}(scope=src,tests)\n"));
    rendered.push_str("|next search:owner:<top-owner>(items),search:tests:<top-owner>\n");
    rendered.push_str("|budget commands=3 rounds=2 output=bounded\n");
}

fn append_default_plan(rendered: &mut String) {
    rendered.push_str("|recipe default focus=prime-first token=bounded\n");
    rendered.push_str("|prefer search:prime\n");
    rendered.push_str("|next search:workspace,search:prime\n");
    rendered.push_str("|budget commands=2 rounds=2 output=bounded\n");
}

fn dependency_root_from_query(query: &str) -> &str {
    let dependency = query
        .split_once("::")
        .map_or(query, |(dependency, _api)| dependency);
    let dependency = dependency
        .split_once('@')
        .map_or(dependency, |(dependency, _version)| dependency);
    dependency
        .split_once('/')
        .map_or(dependency, |(dependency, _subpath)| dependency)
}

fn api_query_from_dependency_query(query: &str) -> Option<&str> {
    query.split_once("::").map(|(_, api)| api)
}
