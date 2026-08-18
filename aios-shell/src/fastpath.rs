//! Instant, deterministic answers for common shell-assistant intents.
//!
//! The LLM backends (local or network) are accurate but pay a per-query cost:
//! model prefill over a large system/knowledge/context prompt on CPU takes
//! seconds. For the most frequent, unambiguous requests we can answer in
//! microseconds with no model at all. This keeps the shell feeling instant and
//! only falls back to the LLM when no rule matches with high confidence.
//!
//! Rules only fire when confident; anything ambiguous returns `None` so the
//! full LLM path handles it.

/// A fast, deterministic answer plus the rule that produced it (for logging).
pub struct FastAnswer {
    pub text: String,
    pub rule: &'static str,
}

/// Commands the shell knows about, used for typo correction.
const KNOWN_COMMANDS: &[&str] = &[
    "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "grep", "find", "wc", "head", "tail", "ps",
    "kill", "top", "echo", "env", "pwd", "chmod", "chown", "df", "du", "date", "uptime", "whoami",
    "hostname", "cd", "export", "clear", "history", "help", "git", "ssh", "curl", "wget", "tar",
    "unzip", "python", "python3", "pip", "node", "npm", "cargo", "make", "docker", "vim", "nano",
];

/// Try to answer a raw user query instantly. Returns `None` when the query is
/// not a confident match for any rule (the caller should then use the LLM).
pub fn try_fast_answer(query: &str) -> Option<FastAnswer> {
    if std::env::var("AIOS_FASTPATH").map(|v| v == "0").unwrap_or(false) {
        return None;
    }

    let q = query.trim();
    let lower = q.to_lowercase();

    if let Some(text) = chmod_explanation(&lower) {
        return Some(FastAnswer { text, rule: "chmod-explain" });
    }
    if let Some(text) = find_large_files(&lower) {
        return Some(FastAnswer { text, rule: "find-large-files" });
    }
    if let Some(text) = simple_intent(&lower) {
        return Some(FastAnswer { text, rule: "simple-intent" });
    }
    if let Some(text) = typo_correction(&lower) {
        return Some(FastAnswer { text, rule: "typo-correction" });
    }

    None
}

/// "what does chmod 755 do" -> explain the octal permission bits.
fn chmod_explanation(lower: &str) -> Option<String> {
    if !lower.contains("chmod") {
        return None;
    }
    let digits: String = lower
        .split_whitespace()
        .find(|w| w.len() == 3 && w.chars().all(|c| ('0'..='7').contains(&c)))?
        .to_string();
    let bytes = digits.as_bytes();
    let who = ["owner", "group", "others"];
    let mut parts = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        let d = (b - b'0') as u8;
        let mut perms = Vec::new();
        if d & 4 != 0 {
            perms.push("read");
        }
        if d & 2 != 0 {
            perms.push("write");
        }
        if d & 1 != 0 {
            perms.push("execute");
        }
        let perms = if perms.is_empty() {
            "no access".to_string()
        } else {
            perms.join("/")
        };
        parts.push(format!("{}: {}", who[i], perms));
    }
    Some(format!(
        "`chmod {}` sets permissions to {}.",
        digits,
        parts.join(", ")
    ))
}

/// "list/find files larger than 100 MB" -> a `find` command.
fn find_large_files(lower: &str) -> Option<String> {
    let mentions_files = lower.contains("file");
    let mentions_size = lower.contains("larger")
        || lower.contains("bigger")
        || lower.contains("over")
        || lower.contains("greater")
        || lower.contains("more than");
    if !(mentions_files && mentions_size) {
        return None;
    }
    // Find a "<number><unit>" or "<number> <unit>" token.
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if let Some((num, unit)) = parse_size(clean) {
            return Some(size_command(num, unit));
        }
        // number then a separate unit word
        if clean.chars().all(|c| c.is_ascii_digit()) && !clean.is_empty() {
            if let Some(next) = tokens.get(i + 1) {
                let unit = next.trim_matches(|c: char| !c.is_ascii_alphabetic());
                if let Some(u) = normalize_unit(unit) {
                    return Some(size_command(clean.to_string(), u));
                }
            }
        }
    }
    None
}

fn parse_size(tok: &str) -> Option<(String, char)> {
    let digits_end = tok.find(|c: char| !c.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    let (num, rest) = tok.split_at(digits_end);
    let unit = normalize_unit(rest)?;
    Some((num.to_string(), unit))
}

fn normalize_unit(unit: &str) -> Option<char> {
    match unit.to_lowercase().as_str() {
        "k" | "kb" | "kib" => Some('k'),
        "m" | "mb" | "mib" => Some('M'),
        "g" | "gb" | "gib" => Some('G'),
        _ => None,
    }
}

fn size_command(num: String, unit: char) -> String {
    format!(
        "```sh\nfind . -type f -size +{}{}\n```",
        num, unit
    )
}

/// A few high-frequency, unambiguous questions.
fn simple_intent(lower: &str) -> Option<String> {
    let has = |terms: &[&str]| terms.iter().all(|t| lower.contains(t));
    if has(&["where", "am", "i"]) || (lower.contains("current") && lower.contains("director")) {
        return Some("```sh\npwd\n```".to_string());
    }
    if lower.contains("disk") && (lower.contains("space") || lower.contains("usage") || lower.contains("free")) {
        return Some("```sh\ndf -h\n```".to_string());
    }
    None
}

/// "I typed 'gti status' and it failed" -> suggest the closest known command.
fn typo_correction(lower: &str) -> Option<String> {
    let signals_typo = lower.contains("did i mean")
        || lower.contains("what did i mean")
        || (lower.contains("typed") && (lower.contains("fail") || lower.contains("mean")))
        || (lower.contains("meant"));
    if !signals_typo {
        return None;
    }
    // Extract a quoted token or the first word-like token that isn't a known command.
    let candidate = extract_quoted_word(lower)
        .or_else(|| lower.split_whitespace().find(|w| looks_like_command_token(w)).map(|s| s.to_string()))?;
    let candidate = candidate.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if candidate.is_empty() || KNOWN_COMMANDS.contains(&candidate) {
        return None;
    }
    let (best, dist) = KNOWN_COMMANDS
        .iter()
        .map(|c| (*c, osa_distance(candidate, c)))
        .min_by_key(|(_, d)| *d)?;
    if dist == 1 {
        Some(format!("You likely meant `{}` (not `{}`).", best, candidate))
    } else {
        None
    }
}

fn extract_quoted_word(s: &str) -> Option<String> {
    let start = s.find('\'')?;
    let rest = &s[start + 1..];
    let end = rest.find('\'')?;
    let inner = &rest[..end];
    inner.split_whitespace().next().map(|w| w.to_string())
}

fn looks_like_command_token(w: &str) -> bool {
    let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    w.len() >= 2 && w.len() <= 12 && w.chars().all(|c| c.is_ascii_lowercase())
}

/// Optimal String Alignment distance (Levenshtein plus adjacent transposition),
/// so common keyboard typos like `gti` -> `git` count as a single edit.
fn osa_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        d[i][0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chmod_755() {
        let a = try_fast_answer("in one sentence, what does chmod 755 do?").unwrap();
        assert_eq!(a.rule, "chmod-explain");
        assert!(a.text.contains("owner: read/write/execute"));
        assert!(a.text.contains("group: read/execute"));
        assert!(a.text.contains("others: read/execute"));
    }

    #[test]
    fn chmod_644() {
        let a = try_fast_answer("what does chmod 644 do").unwrap();
        assert!(a.text.contains("owner: read/write"));
        assert!(a.text.contains("group: read"));
        assert!(a.text.contains("others: read"));
    }

    #[test]
    fn large_files_mb() {
        let a = try_fast_answer("give the command to list files larger than 100MB").unwrap();
        assert_eq!(a.rule, "find-large-files");
        assert!(a.text.contains("find . -type f -size +100M"));
    }

    #[test]
    fn large_files_spaced_unit() {
        let a = try_fast_answer("find files bigger than 2 GB here").unwrap();
        assert!(a.text.contains("find . -type f -size +2G"));
    }

    #[test]
    fn typo_git() {
        let a = try_fast_answer("I typed 'gti status' and it failed, what did I mean?").unwrap();
        assert_eq!(a.rule, "typo-correction");
        assert!(a.text.contains("`git`"));
    }

    #[test]
    fn pwd_intent() {
        let a = try_fast_answer("where am i").unwrap();
        assert!(a.text.contains("pwd"));
    }

    #[test]
    fn no_match_falls_through() {
        assert!(try_fast_answer("explain how tcp congestion control works").is_none());
        assert!(try_fast_answer("write a haiku about rust").is_none());
    }
}
