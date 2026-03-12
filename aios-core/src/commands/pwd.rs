pub fn run(_args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    crate::CommandOutput::success(format!("{}\n", cwd.to_string_lossy()))
}
