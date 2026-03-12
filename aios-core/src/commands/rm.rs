use aios_kernel::fs::{remove_file, remove_dir};
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let recursive = parsed.has('r') || parsed.has('R') || parsed.has_long("recursive");
    let force = parsed.has_any('f', "force");
    let paths = parsed.paths(cwd);

    if paths.is_empty() {
        return crate::CommandOutput::error("rm: missing operand".to_string(), 1);
    }

    for path in &paths {
        if !path.exists() {
            if !force {
                return crate::CommandOutput::error(format!("rm: {}: No such file or directory", path.display()), 1);
            }
            continue;
        }

        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return crate::CommandOutput::error(format!("rm: {}: {}", path.display(), e), 1),
        };

        if meta.is_dir() {
            if !recursive {
                return crate::CommandOutput::error(format!("rm: {}: is a directory", path.display()), 1);
            }
            if let Err(e) = remove_dir(path, true) {
                return crate::CommandOutput::error(format!("rm: {}: {}", path.display(), e), 1);
            }
        } else {
            if let Err(e) = remove_file(path) {
                return crate::CommandOutput::error(format!("rm: {}: {}", path.display(), e), 1);
            }
        }
    }

    crate::CommandOutput::success(String::new())
}
