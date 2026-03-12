use aios_kernel::fs::{get_file_info, rename};
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();

    for arg in args {
        if !arg.starts_with('-') {
            let path = if Path::new(arg).is_absolute() {
                std::path::PathBuf::from(arg)
            } else {
                cwd.join(arg)
            };
            paths.push(path);
        }
    }

    if paths.len() < 2 {
        return crate::CommandOutput::error("mv: missing file operand".to_string(), 1);
    }

    let dest = paths.pop().unwrap();
    let sources = paths;

    for src in &sources {
        let target = if dest.is_dir() || (dest.exists() && std::fs::metadata(&dest).map(|m| m.is_dir()).unwrap_or(false)) {
            let info = match get_file_info(src) {
                Ok(i) => i,
                Err(e) => return crate::CommandOutput::error(format!("mv: {}: {}", src.display(), e), 1),
            };
            dest.join(&info.name)
        } else {
            dest.clone()
        };

        if let Err(e) = rename(src, &target) {
            return crate::CommandOutput::error(format!("mv: {}: {}", src.display(), e), 1);
        }
    }

    crate::CommandOutput::success(String::new())
}
