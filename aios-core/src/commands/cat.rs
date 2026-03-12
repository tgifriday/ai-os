use aios_kernel::fs::read_file;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let number = parsed.has('n');
    let paths = parsed.paths(cwd);

    if paths.is_empty() {
        return crate::CommandOutput::error("cat: no file specified".to_string(), 1);
    }

    let mut out = String::new();
    for path in &paths {
        let content = match read_file(path) {
            Ok(c) => c,
            Err(e) => return crate::CommandOutput::error(format!("cat: {}: {}", path.display(), e), 1),
        };
        for (i, line) in content.lines().enumerate() {
            if number {
                out.push_str(&format!("{:6}\t{}\n", i + 1, line));
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    crate::CommandOutput::success(out)
}
