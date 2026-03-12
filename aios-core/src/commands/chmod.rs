use aios_kernel::fs::set_permissions;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    if args.len() < 2 {
        return crate::CommandOutput::error("chmod: missing operand".to_string(), 1);
    }
    let mode_str = args[0];
    let mode = match u32::from_str_radix(mode_str, 8) {
        Ok(m) if m <= 0o7777 => m,
        _ => {
            return crate::CommandOutput::error(
                format!("chmod: invalid mode: {}", mode_str),
                1,
            );
        }
    };
    for arg in &args[1..] {
        if arg.starts_with('-') {
            continue;
        }
        let path = if Path::new(arg).is_absolute() {
            std::path::PathBuf::from(arg)
        } else {
            cwd.join(arg)
        };
        if let Err(e) = set_permissions(&path, mode) {
            return crate::CommandOutput::error(format!("chmod: {}: {}", path.display(), e), 1);
        }
    }
    crate::CommandOutput::success(String::new())
}
