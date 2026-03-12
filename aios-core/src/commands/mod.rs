pub mod cat;
pub mod chmod;
pub mod cp;
pub mod date;
pub mod df;
pub mod du;
pub mod echo;
pub mod env_cmd;
pub mod find;
pub mod free;
pub mod grep;
pub mod head;
pub mod hostname;
pub mod kill;
pub mod ls;
pub mod mkdir;
pub mod mv;
pub mod ps;
pub mod pwd;
pub mod rm;
pub mod rmdir;
pub mod tail;
pub mod top;
pub mod uname;
pub mod uptime;
pub mod wc;
pub mod whoami;

use crate::CommandOutput;
use std::collections::HashMap;

pub type CommandFn = fn(&[&str], &std::path::Path) -> CommandOutput;

/// Commands handled directly by the shell (not shadowing OS commands).
/// Standard OS commands (ls, ps, df, grep, etc.) pass through to the real OS.
pub fn builtin_commands() -> HashMap<&'static str, CommandFn> {
    HashMap::new()
}

/// Shell-level builtins that require special handling (cd changes cwd,
/// export modifies env, etc.). These are handled in the executor/router directly.
pub fn is_builtin(name: &str) -> bool {
    matches!(name, "cd" | "export" | "clear" | "help" | "llm")
}
