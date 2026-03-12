use aios_kernel::fs;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    if args.is_empty() {
        return crate::CommandOutput::error("rmdir: missing operand".to_string(), 1);
    }
    for arg in args {
        if arg.starts_with('-') {
            continue;
        }
        let path = if Path::new(arg).is_absolute() {
            std::path::PathBuf::from(arg)
        } else {
            cwd.join(arg)
        };
        if let Err(e) = fs::remove_dir(&path, false) {
            return crate::CommandOutput::error(format!("rmdir: {}: {}", path.display(), e), 1);
        }
    }
    crate::CommandOutput::success(String::new())
}
