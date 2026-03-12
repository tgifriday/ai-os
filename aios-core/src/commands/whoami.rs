#[cfg(unix)]
pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    use users::get_current_username;
    match get_current_username() {
        Some(u) => crate::CommandOutput::success(u.to_string_lossy().into_owned()),
        None => crate::CommandOutput::error("could not get current username".to_string(), 1),
    }
}

#[cfg(not(unix))]
pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    match std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        Ok(u) => crate::CommandOutput::success(u),
        Err(_) => crate::CommandOutput::error("could not get current username".to_string(), 1),
    }
}
