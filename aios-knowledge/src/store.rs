use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeStore {
    documents: Vec<Document>,
}

impl KnowledgeStore {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    pub fn add_document(&mut self, doc: Document) {
        self.documents.push(doc);
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let idf = compute_idf(&self.documents, &query_terms);

        let mut results: Vec<SearchResult> = self
            .documents
            .iter()
            .filter_map(|doc| {
                let score = score_document(doc, &query_terms, &idf);
                if score > 0.0 {
                    Some(SearchResult {
                        document: doc.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|d| d.id == id)
    }

    pub fn get_by_category(&self, category: &str) -> Vec<&Document> {
        self.documents
            .iter()
            .filter(|d| d.category == category)
            .collect()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let store: Self = serde_json::from_str(&json)?;
        Ok(store)
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
}

impl Default for KnowledgeStore {
    fn default() -> Self {
        Self::new()
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(String::from)
        .collect()
}

fn compute_idf(documents: &[Document], terms: &[String]) -> HashMap<String, f64> {
    let n = documents.len() as f64;
    let mut idf = HashMap::new();

    for term in terms {
        let doc_freq = documents
            .iter()
            .filter(|doc| {
                let haystack = format!(
                    "{} {} {} {}",
                    doc.title,
                    doc.content,
                    doc.category,
                    doc.tags.join(" ")
                )
                .to_lowercase();
                haystack.contains(term.as_str())
            })
            .count() as f64;

        let val = if doc_freq > 0.0 {
            (n / doc_freq).ln() + 1.0
        } else {
            0.0
        };
        idf.insert(term.clone(), val);
    }

    idf
}

fn score_document(doc: &Document, query_terms: &[String], idf: &HashMap<String, f64>) -> f64 {
    let title_lower = doc.title.to_lowercase();
    let content_lower = doc.content.to_lowercase();
    let tags_lower: Vec<String> = doc.tags.iter().map(|t| t.to_lowercase()).collect();
    let tags_joined = tags_lower.join(" ");

    let title_tokens = tokenize(&doc.title);
    let content_tokens = tokenize(&doc.content);
    let total_tokens = (title_tokens.len() + content_tokens.len()).max(1) as f64;

    let mut score = 0.0;

    for term in query_terms {
        let term_idf = idf.get(term).copied().unwrap_or(0.0);
        if term_idf == 0.0 {
            continue;
        }

        let title_hits = title_lower.matches(term.as_str()).count() as f64;
        let content_hits = content_lower.matches(term.as_str()).count() as f64;
        let tag_hits = tags_joined.matches(term.as_str()).count() as f64;

        // Title matches weighted 3x, tag matches 2x
        let tf = (title_hits * 3.0 + content_hits + tag_hits * 2.0) / total_tokens;
        score += tf * term_idf;
    }

    // Boost exact title match
    let query_joined: String = query_terms.join(" ");
    if title_lower == query_joined {
        score *= 2.0;
    } else if title_lower.contains(&query_joined) {
        score *= 1.5;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc(id: &str, title: &str, content: &str, tags: &[&str]) -> Document {
        Document {
            id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            category: "command".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_search_returns_relevant() {
        let mut store = KnowledgeStore::new();
        store.add_document(sample_doc("ls", "ls", "List directory contents", &["list", "directory"]));
        store.add_document(sample_doc("cat", "cat", "Concatenate and print files", &["read", "file"]));

        let results = store.search("list directory", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "ls");
    }

    #[test]
    fn test_get_by_id() {
        let mut store = KnowledgeStore::new();
        store.add_document(sample_doc("ls", "ls", "List files", &[]));
        assert!(store.get_by_id("ls").is_some());
        assert!(store.get_by_id("missing").is_none());
    }

    #[test]
    fn test_get_by_category() {
        let mut store = KnowledgeStore::new();
        store.add_document(sample_doc("ls", "ls", "List files", &[]));
        assert_eq!(store.get_by_category("command").len(), 1);
        assert_eq!(store.get_by_category("concept").len(), 0);
    }
}
