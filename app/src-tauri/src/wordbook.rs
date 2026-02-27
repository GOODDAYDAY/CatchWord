use crate::types::WordEntry;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Wordbook {
    file_path: PathBuf,
    words: Mutex<Vec<WordEntry>>,
}

impl Wordbook {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let file_path = app_data_dir.join("wordbook.json");
        let words = Self::load_from_file(&file_path);
        Wordbook {
            file_path,
            words: Mutex::new(words),
        }
    }

    fn load_from_file(path: &PathBuf) -> Vec<WordEntry> {
        match fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn save(&self, words: &Vec<WordEntry>) {
        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(words) {
            let _ = fs::write(&self.file_path, data);
        }
    }

    pub fn add_word(
        &self,
        word: &str,
        translation: &str,
        phonetic: &str,
        examples: &[String],
        source_context: &str,
    ) {
        let mut words = self.words.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        if let Some(existing) = words
            .iter_mut()
            .find(|w| w.word.to_lowercase() == word.to_lowercase())
        {
            existing.query_count += 1;
            existing.last_query_time = now;
            if !source_context.is_empty() && existing.source_context.is_empty() {
                existing.source_context = source_context.to_string();
            }
        } else {
            words.push(WordEntry {
                word: word.to_lowercase(),
                translation: translation.to_string(),
                phonetic: phonetic.to_string(),
                examples: examples.to_vec(),
                query_time: now.clone(),
                last_query_time: now,
                source_context: source_context.to_string(),
                query_count: 1,
                mastered: false,
            });
        }

        self.save(&words);
    }
}
