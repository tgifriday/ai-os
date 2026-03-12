pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let mut no_newline = false;
    let mut tokens = Vec::new();
    for arg in args {
        if *arg == "-n" {
            no_newline = true;
        } else {
            tokens.push(*arg);
        }
    }
    let output = tokens.join(" ");
    let output = if no_newline {
        output
    } else {
        format!("{}\n", output)
    };
    crate::CommandOutput::success(output)
}
