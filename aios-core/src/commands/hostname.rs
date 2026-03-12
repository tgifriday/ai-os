use aios_kernel::network::get_hostname;

pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let name = get_hostname();
    crate::CommandOutput::success(name)
}
