use rewrit_engine::ExplainResult;

pub fn print(result: ExplainResult) {
    println!("Case: {}", result.case_id);
    if result.divergences.is_empty() {
        println!("Status: equivalent or no divergence recorded");
        return;
    }
    for divergence in result.divergences {
        println!("Status: failed");
        println!("Kind: {:?}", divergence.kind);
        if let Some(path) = divergence.path {
            println!("Path: {path}");
        }
        println!("Message: {}", divergence.message);
        if let Some(reference) = divergence.reference {
            println!("\nReference:\n{}", serde_json::to_string_pretty(&reference).unwrap_or_default());
        }
        if let Some(candidate) = divergence.candidate {
            println!("\nCandidate:\n{}", serde_json::to_string_pretty(&candidate).unwrap_or_default());
        }
        if let Some(policy) = divergence.policy {
            println!("\nPolicy: {policy}");
        }
        if let Some(hint) = divergence.hint {
            println!("\nHint: {hint}");
        }
    }
}

