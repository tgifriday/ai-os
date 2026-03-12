use chrono::Local;

pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let now = Local::now();
    let format_str = args
        .iter()
        .find(|a| a.starts_with('+'))
        .map(|a| &a[1..]);
    let output = match format_str {
        Some(fmt) => now.format(fmt).to_string(),
        None => now.format("%a %b %d %H:%M:%S %Y").to_string(),
    };
    crate::CommandOutput::success(output)
}
