use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: i64,
    pub exit_code: i32,
    pub cwd: String,
}

pub struct History {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
    file_path: PathBuf,
}

impl History {
    pub fn new(max_entries: usize) -> Self {
        let file_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".aios_history");

        let mut history = Self {
            entries: Vec::new(),
            max_entries,
            file_path,
        };
        history.load();
        history
    }

    pub fn add(&mut self, command: String, exit_code: i32, cwd: &str) {
        let entry = HistoryEntry {
            command,
            timestamp: chrono::Utc::now().timestamp(),
            exit_code,
            cwd: cwd.to_string(),
        };
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.save();
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn recent_commands(&self, count: usize) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .take(count)
            .map(|e| e.command.clone())
            .collect()
    }

    pub fn search(&self, pattern: &str) -> Vec<&HistoryEntry> {
        let pattern_lower = pattern.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.command.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    pub fn format_display(&self) -> String {
        let mut output = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            output.push_str(&format!("  {}  {}\n", i + 1, entry.command));
        }
        output
    }

    fn load(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.file_path) {
            if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&content) {
                self.entries = entries;
                if self.entries.len() > self.max_entries {
                    let drain_count = self.entries.len() - self.max_entries;
                    self.entries.drain(..drain_count);
                }
            }
        }
    }

    fn save(&self) {
        if let Ok(content) = serde_json::to_string(&self.entries) {
            let _ = std::fs::write(&self.file_path, content);
        }
    }
}
