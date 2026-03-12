use serde_json::json;
use std::collections::HashMap;

pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let mut env_map: HashMap<String, String> = HashMap::new();
    for (k, v) in std::env::vars() {
        env_map.insert(k, v);
    }
    let stdout = env_map
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    let structured = json!({ "env": env_map });
    crate::CommandOutput::success_structured(stdout, structured)
}
