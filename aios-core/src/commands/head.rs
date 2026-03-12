pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &['n']);
    let n: usize = parsed
        .value('n')
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let files = parsed.paths(cwd);

    if files.is_empty() {
        return crate::CommandOutput::error("head: file required\n".into(), 1);
    }

    let mut output_lines = Vec::new();
    for path in &files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return crate::CommandOutput::error(format!("head: {}: {}\n", path.display(), e), 1),
        };
        let taken: Vec<&str> = content.lines().take(n).collect();
        if files.len() > 1 {
            output_lines.push(format!("==> {} <==", path.display()));
        }
        output_lines.extend(taken.iter().map(|s| (*s).to_string()));
        if files.len() > 1 {
            output_lines.push(String::new());
        }
    }

    let mut stdout = output_lines.join("\n");
    if files.len() > 1 && stdout.ends_with('\n') {
        stdout.pop();
    }
    crate::CommandOutput::success(stdout)
}
