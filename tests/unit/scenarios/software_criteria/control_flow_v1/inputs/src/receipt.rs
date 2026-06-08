//! Receipt helpers.

struct ContractReceipt {
    summary: ReceiptSummary,
    repositories: Vec<RepositoryReceipt>,
}

struct ReceiptSummary {
    failed_query_count: usize,
}

struct RepositoryReceipt {
    query_receipts: Vec<QueryReceipt>,
}

struct QueryReceipt {
    passed: bool,
    name: String,
}

fn collect_failed_queries(receipt: &ContractReceipt) -> Vec<String> {
    let mut failed = Vec::new();
    if receipt.summary.failed_query_count > 0 {
        for repository in &receipt.repositories {
            for query in &repository.query_receipts {
                if !query.passed {
                    failed.push(query.name.clone());
                }
            }
        }
    }
    failed
}

fn failed_query_names(queries: &[QueryReceipt]) -> Vec<String> {
    let mut names = Vec::new();
    for query in queries {
        if !query.passed {
            names.push(query.name.clone());
        }
    }
    names
}
