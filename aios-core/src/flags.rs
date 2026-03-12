use std::collections::HashSet;

/// Parsed command-line arguments with expanded short flags.
///
/// Handles combined short flags like `-alh` (expanded to `a`, `l`, `h`),
/// long options like `--all`, value-bearing flags like `-n 10`,
/// and `--` to stop flag parsing.
#[derive(Debug)]
pub struct ParsedArgs<'a> {
    pub flags: HashSet<char>,
    pub long_flags: HashSet<&'a str>,
    pub positional: Vec<&'a str>,
    values: Vec<(char, &'a str)>,
}

impl<'a> ParsedArgs<'a> {
    pub fn has(&self, short: char) -> bool {
        self.flags.contains(&short)
    }

    pub fn has_long(&self, name: &str) -> bool {
        self.long_flags.contains(name)
    }

    pub fn has_any(&self, short: char, long: &str) -> bool {
        self.has(short) || self.has_long(long)
    }

    pub fn value(&self, key: char) -> Option<&'a str> {
        self.values
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    }

    pub fn resolve_path(&self, raw: &str, cwd: &std::path::Path) -> std::path::PathBuf {
        if std::path::Path::new(raw).is_absolute() {
            std::path::PathBuf::from(raw)
        } else {
            cwd.join(raw)
        }
    }

    pub fn paths(&self, cwd: &std::path::Path) -> Vec<std::path::PathBuf> {
        self.positional
            .iter()
            .map(|p| self.resolve_path(p, cwd))
            .collect()
    }
}

/// Parse command arguments with support for combined short flags.
///
/// `value_flags` lists short flag chars that consume the next argument as a value
/// (e.g., `&['n']` means `-n 10` captures `10` as the value for `'n'`).
pub fn parse<'a>(args: &'a [&'a str], value_flags: &[char]) -> ParsedArgs<'a> {
    let mut parsed = ParsedArgs {
        flags: HashSet::new(),
        long_flags: HashSet::new(),
        positional: Vec::new(),
        values: Vec::new(),
    };

    let mut i = 0;
    let mut saw_dashdash = false;

    while i < args.len() {
        let arg = args[i];

        if saw_dashdash || !arg.starts_with('-') || arg == "-" {
            parsed.positional.push(arg);
            i += 1;
            continue;
        }

        if arg == "--" {
            saw_dashdash = true;
            i += 1;
            continue;
        }

        if arg.starts_with("--") {
            parsed.long_flags.insert(&arg[2..]);
            i += 1;
            continue;
        }

        for c in arg[1..].chars() {
            if value_flags.contains(&c) {
                if i + 1 < args.len() {
                    i += 1;
                    parsed.values.push((c, args[i]));
                }
                break;
            }
            parsed.flags.insert(c);
        }
        i += 1;
    }

    parsed
}
