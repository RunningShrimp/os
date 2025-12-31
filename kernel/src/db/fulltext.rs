//! Full-Text Search
//!
//! Implements full-text search with:
//! - Inverted index
//! - Tokenizer
//! - BM25 ranking
//! - Fuzzy search

use super::types::{Value, RowId};
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::{BTreeMap, HashMap};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;

/// Full-text search engine
pub struct FullTextIndex {
    name: String,
    table_name: String,
    column_name: String,
    inverted_index: Mutex<InvertedIndex>,
    tokenizer: Tokenizer,
    ranking: RankingModel,
}

impl FullTextIndex {
    pub fn new(
        name: impl Into<String>,
        table_name: impl Into<String>,
        column_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            table_name: table_name.into(),
            column_name: column_name.into(),
            inverted_index: Mutex::new(InvertedIndex::new()),
            tokenizer: Tokenizer::new(),
            ranking: RankingModel::BM25,
        }
    }

    /// Index a document
    pub fn insert_document(&self, row_id: RowId, text: &str) -> DbResult<()> {
        let tokens = self.tokenizer.tokenize(text);

        let mut index = self.inverted_index.lock();
        for (token, position) in tokens {
            index.add_document(token.clone(), row_id, position);
        }

        Ok(())
    }

    /// Update indexed document
    pub fn update_document(&self, row_id: RowId, old_text: &str, new_text: &str) -> DbResult<()> {
        self.delete_document(row_id, old_text)?;
        self.insert_document(row_id, new_text)?;
        Ok(())
    }

    /// Delete a document
    pub fn delete_document(&self, row_id: RowId, text: &str) -> DbResult<()> {
        let tokens = self.tokenizer.tokenize(text);

        let mut index = self.inverted_index.lock();
        for (token, _) in tokens {
            index.remove_document(&token, row_id);
        }

        Ok(())
    }

    /// Search for documents
    pub fn search(&self, query: &str, limit: usize) -> DbResult<Vec<SearchResult>> {
        let query_tokens = self.tokenizer.tokenize(query);
        let index = self.inverted_index.lock();

        // Collect matching documents
        let mut doc_scores: BTreeMap<RowId, f64> = BTreeMap::new();

        for (token, _) in query_tokens {
            if let Some(postings) = index.get_postings(&token) {
                for posting in postings {
                    let score = self.calculate_score(&token, posting);
                    *doc_scores.entry(posting.row_id).or_insert(0.0) += score;
                }
            }
        }

        // Sort by score
        let mut results: Vec<_> = doc_scores
            .into_iter()
            .map(|(row_id, score)| SearchResult { row_id, score })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Apply limit
        results.truncate(limit);

        Ok(results)
    }

    /// Calculate document score
    fn calculate_score(&self, term: &str, posting: &Posting) -> f64 {
        match self.ranking {
            RankingModel::BM25 => self.bm25_score(term, posting),
            RankingModel::TFIDF => self.tfidf_score(term, posting),
        }
    }

    /// BM25 ranking score
    fn bm25_score(&self, term: &str, posting: &Posting) -> f64 {
        let k1 = 1.5; // Term frequency saturation
        let b = 0.75; // Length normalization

        let idf = self.idf(term);
        let tf = posting.term_frequency as f64;
        let doc_len = posting.document_length as f64;
        let avg_doc_len = 100.0; // Average document length (simplified)

        let tf_component = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (doc_len / avg_doc_len)));
        idf * tf_component
    }

    /// TF-IDF score
    fn tfidf_score(&self, term: &str, posting: &Posting) -> f64 {
        let tf = (posting.term_frequency as f64).log2();
        let idf = self.idf(term);
        tf * idf
    }

    /// Inverse document frequency
    fn idf(&self, term: &str) -> f64 {
        let index = self.inverted_index.lock();

        let doc_count = index.document_count();
        let term_doc_count = index.document_count_for_term(term);

        if term_doc_count == 0 {
            return 0.0;
        }

        ((doc_count as f64) / (term_doc_count as f64)).log2()
    }

    /// Fuzzy search with edit distance
    pub fn fuzzy_search(&self, query: &str, max_distance: usize, limit: usize) -> DbResult<Vec<SearchResult>> {
        let index = self.inverted_index.lock();
        let mut results = Vec::new();

        // Get all unique terms
        let all_terms = index.all_terms();

        for term in all_terms {
            let distance = edit_distance(query, term);

            if distance <= max_distance {
                if let Some(postings) = index.get_postings(term) {
                    for posting in postings {
                        let score = 1.0 / (distance as f64 + 1.0);
                        results.push(SearchResult {
                            row_id: posting.row_id,
                            score,
                        });
                    }
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }
}

/// Inverted index
#[derive(Debug)]
struct InvertedIndex {
    postings: HashMap<String, Vec<Posting>>,
    document_lengths: HashMap<RowId, usize>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            document_lengths: HashMap::new(),
        }
    }

    pub fn add_document(&mut self, term: String, row_id: RowId, position: usize) {
        let postings_list = self.postings.entry(term).or_insert_with(Vec::new);

        // Find or create posting for this document
        let posting = postings_list.iter_mut().find(|p| p.row_id == row_id);

        if let Some(p) = posting {
            p.term_frequency += 1;
            p.positions.push(position);
        } else {
            postings_list.push(Posting {
                row_id,
                term_frequency: 1,
                positions: vec![position],
                document_length: 0, // Will be updated separately
            });
        }
    }

    pub fn remove_document(&mut self, term: &str, row_id: RowId) {
        if let Some(postings) = self.postings.get_mut(term) {
            postings.retain(|p| p.row_id != row_id);
        }
    }

    pub fn get_postings(&self, term: &str) -> Option<&[Posting]> {
        self.postings.get(term).map(|v| v.as_slice())
    }

    pub fn document_count(&self) -> usize {
        self.document_lengths.len()
    }

    pub fn document_count_for_term(&self, term: &str) -> usize {
        self.postings.get(term).map(|v| v.len()).unwrap_or(0)
    }

    pub fn all_terms(&self) -> Vec<&str> {
        self.postings.keys().map(|k| k.as_str()).collect()
    }
}

/// Posting in inverted index
#[derive(Debug, Clone)]
pub struct Posting {
    pub row_id: RowId,
    pub term_frequency: usize,
    pub positions: Vec<usize>,
    pub document_length: usize,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub row_id: RowId,
    pub score: f64,
}

/// Tokenizer for text processing
pub struct Tokenizer {
    lowercase: bool,
    remove_stopwords: bool,
    stem: bool,
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            lowercase: true,
            remove_stopwords: true,
            stem: false,
        }
    }

    pub fn tokenize(&self, text: &str) -> Vec<(String, usize)> {
        let mut tokens = Vec::new();
        let mut position = 0;

        for word in text.split_whitespace() {
            let mut word = word.to_string();

            // Remove punctuation
            word.retain(|c| c.is_alphanumeric());

            if word.is_empty() {
                continue;
            }

            // Lowercase
            if self.lowercase {
                word = word.to_lowercase();
            }

            // Remove stopwords
            if self.remove_stopwords && Self::is_stopword(&word) {
                continue;
            }

            // Stemming (simplified)
            if self.stem {
                word = self.stem(&word);
            }

            tokens.push((word, position));
            position += 1;
        }

        tokens
    }

    fn is_stopword(word: &str) -> bool {
        let stopwords = ["a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for"];
        stopwords.contains(&word)
    }

    fn stem(&self, word: &str) -> String {
        // Very simplified stemming
        if word.ends_with("ing") {
            word[..word.len() - 3].to_string()
        } else if word.ends_with("ed") {
            word[..word.len() - 2].to_string()
        } else if word.ends_with("s") {
            word[..word.len() - 1].to_string()
        } else {
            word.to_string()
        }
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Ranking model
#[derive(Debug, Clone, Copy)]
pub enum RankingModel {
    BM25,
    TFIDF,
}

/// Calculate Levenshtein edit distance
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }

    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            if a_chars[i - 1] == b_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = 1 + [
                    dp[i - 1][j],      // deletion
                    dp[i][j - 1],      // insertion
                    dp[i - 1][j - 1],  // substitution
                ].into_iter().min().unwrap();
            }
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fulltext_index() {
        let index = FullTextIndex::new("test_idx", "documents", "content");
        index.insert_document(1, "Hello world").unwrap();
        index.insert_document(2, "World of coding").unwrap();

        let results = index.search("world", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_tokenizer() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hello World, this is a TEST!");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("hello", "hello"), 0);
        assert_eq!(edit_distance("test", "text"), 1);
    }
}
