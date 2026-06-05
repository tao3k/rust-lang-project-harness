//! Typed `search guide` surface for graph-reasoning agent flows.

pub(crate) fn render_search_guide() -> String {
    [
        "[search-guide] root=. alg=seed-frontier",
        "legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next",
        "alias: graph:{G=search,O=owner,Q=query,D=dependency,T=test,F=finding,F2=feature,R=range}",
        "Q=query:term(<query>)!query;O=owner:path(<path>)!owner;D=dependency:pkg(<dep>)!deps;T=test:path(<test>)!tests;F=finding:finding(<finding>)!finding;F2=feature:feature(<feature>)!cfg;R=range:requested(<path[:line|:start:end]>)!read",
        "G>{Q:matches,O:selects,D:uses,T:covers,F:flags,F2:gates,R:reads}",
        "rank=O,Q,D,T,F,F2,R frontier=O.items,Q.owner,D.deps,T.tests,F.finding,F2.cfg,R.read",
        "omit=code,full-json,raw-source",
        "avoid=raw-read,repeat-owner,full-json",
        "entries=owner-query(O,Q=>items+tests+dependency-usage),query-deps(Q,D=>owners+imports+usage-tests),owner-tests(O=>covering-tests+test-entrypoints+fixtures),finding-frontier(F,O=>affected-owners+tests+verification-actions),feature-cfg(F2=>cfg-gates+owners+verification-surfaces)",
        "|catalog reasoningProfiles=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg entries=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg routes=read-frontier",
        "|entry owner-query selectors=O:owner,Q:query returns=items,tests,dependency-usage frontier=O.items,Q.owner,Q.tests cmd=asp rust search owner <path> items --query <query> --view seeds .",
        "|entry query-deps selectors=Q:query,D:dependency returns=owners,imports,usage-tests frontier=Q.owner,D.public-api,D.tests cmd=asp rust search reasoning query-deps --query <query> --dependency <dep> --view seeds .",
        "|entry owner-tests selectors=O:owner returns=covering-tests,test-entrypoints,fixtures frontier=O.tests,T.owner cmd=asp rust search reasoning owner-tests --owner <path> --view seeds .",
        "|entry finding-frontier selectors=F:finding,O:owner? returns=affected-owners,tests,verification-actions frontier=F.owner,F.tests,O.policy cmd=asp rust search reasoning finding-frontier --query <finding> [--owner <path>] --view seeds .",
        "|entry feature-cfg selectors=F2:feature returns=cfg-gates,owners,verification-surfaces frontier=F2.cfg,F2.owner,F2.tests cmd=asp rust search reasoning feature-cfg --query <feature> --view seeds .",
        "|route read-frontier selectors=R:range returns=symbols,windows,tests,next-actions frontier=R.symbols,R.tests,R.code cmd=asp rust query --from-hook direct-source-read --selector <path[:line-range]> [--code] .",
    ]
    .join("\n")
        + "\n"
}
