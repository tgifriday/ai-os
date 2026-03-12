use aios_kernel::fs;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let recursive = parsed.has_any('p', "parents");
    let paths = parsed.paths(cwd);
    if paths.is_empty() {
        return crate::CommandOutput::error("mkdir: missing operand".to_string(), 1);
    }
    for path in paths {
        if let Err(e) = fs::create_dir(&path, recursive) {
            return crate::CommandOutput::error(format!("mkdir: {}: {}", path.display(), e), 1);
        }
    }
    crate::CommandOutput::success(String::new())
}
