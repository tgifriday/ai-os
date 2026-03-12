use aios_kernel::fs::{copy_file, get_file_info};
use std::path::Path;

fn copy_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(src).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            copy_recursive(&entry.path(), &dst.join(name))?;
        }
    } else {
        copy_file(src, dst).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let recursive = parsed.has('r') || parsed.has('R') || parsed.has_long("recursive");
    let mut paths = parsed.paths(cwd);

    if paths.len() < 2 {
        return crate::CommandOutput::error("cp: missing file operand".to_string(), 1);
    }

    let dest = paths.pop().unwrap();
    let sources = paths;

    for src in &sources {
        let info = match get_file_info(src) {
            Ok(i) => i,
            Err(e) => return crate::CommandOutput::error(format!("cp: {}: {}", src.display(), e), 1),
        };

        let target = if dest.is_dir() || (dest.exists() && std::fs::metadata(&dest).map(|m| m.is_dir()).unwrap_or(false)) {
            dest.join(&info.name)
        } else {
            dest.clone()
        };

        if info.is_dir {
            if !recursive {
                return crate::CommandOutput::error(format!("cp: -r not specified; omitting directory '{}'", src.display()), 1);
            }
            if let Err(e) = copy_recursive(src, &target) {
                return crate::CommandOutput::error(format!("cp: {}: {}", src.display(), e), 1);
            }
        } else {
            if let Err(e) = copy_file(src, &target) {
                return crate::CommandOutput::error(format!("cp: {}: {}", src.display(), e), 1);
            }
        }
    }

    crate::CommandOutput::success(String::new())
}
