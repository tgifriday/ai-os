use users::get_current_username;

pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    match get_current_username() {
        Some(u) => crate::CommandOutput::success(
            u.to_string_lossy().into_owned()
        ),
        None => crate::CommandOutput::error("could not get current username".to_string(), 1),
    }
}
