use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub word: String,
    pub phonetic: String,
    pub translation: String,
    pub definitions: Vec<Definition>,
    pub examples: Vec<String>,
    pub audio_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub part_of_speech: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordEntry {
    pub word: String,
    pub translation: String,
    pub phonetic: String,
    pub examples: Vec<String>,
    pub query_time: String,
    pub last_query_time: String,
    pub source_context: String,
    pub query_count: u32,
    pub mastered: bool,
}
