//! Typed `search guide` surface for graph-reasoning agent flows.

pub(super) fn render_search_guide() -> String {
    [
        "[search-guide] language=rust provider=rs-harness protocol=search-guide.v1",
        "|rule intent=agent selectors=typed json=validator-only",
        "|rule natural-language-intent=deny fields=goal,intent,prompt",
        "|flow guide->prime->entry->reasoning-profile->owner|deps|tests",
        "|entry owner-query selectors=O:owner,Q:query returns=items,tests,dependency-usage frontier=O.items,Q.owner,Q.tests cmd=asp rust search reasoning owner-query --owner <O> --query <Q> --view seeds .",
        "|entry query-deps selectors=Q:query,D:dependency returns=owners,imports,usage-tests frontier=Q.owner,D.public-api,D.tests cmd=asp rust search reasoning query-deps --query <Q> --dependency <D> --view seeds .",
        "|entry owner-tests selectors=O:owner returns=covering-tests,test-entrypoints,fixtures frontier=O.tests,T.owner cmd=asp rust search reasoning owner-tests --owner <O> --view seeds .",
        "|entry finding-frontier selectors=F:finding,O:owner? returns=affected-owners,tests,verification-actions frontier=F.owner,F.tests,O.policy cmd=asp rust search reasoning finding-frontier --finding <F> --view seeds .",
        "|entry read-frontier selectors=R:range returns=symbols,windows,tests,next-actions frontier=R.symbols,R.tests,R.code cmd=asp rust query --from-hook direct-source-read --selector <R> .",
        "|cmd prime=asp rust search prime --view seeds .",
        "|cmd owner-query=asp rust search reasoning owner-query --owner <path> --query <term> [--dependency <dep>] --view seeds .",
        "|cmd query-deps=asp rust search reasoning query-deps --query <term> --dependency <dep> --view seeds .",
        "|cmd owner-tests=asp rust search reasoning owner-tests --owner <path> --view seeds .",
        "|cmd read-frontier=asp rust query --from-hook direct-source-read --selector <path:start:end> .",
        "|avoid raw-read,manual-window-scan,full-json,natural-language-intent",
    ]
    .join("\n")
        + "\n"
}
