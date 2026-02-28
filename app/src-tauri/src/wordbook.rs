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
                favorited: false,
            });
        }

        self.save(&words);
    }

    pub fn get_word(&self, word: &str) -> Option<WordEntry> {
        let words = self.words.lock().unwrap();
        words
            .iter()
            .find(|w| w.word.to_lowercase() == word.to_lowercase())
            .cloned()
    }

    pub fn list_words(&self) -> Vec<WordEntry> {
        let words = self.words.lock().unwrap();
        words.clone()
    }

    pub fn update_favorited(&self, word: &str, favorited: bool) -> bool {
        let mut words = self.words.lock().unwrap();
        if let Some(entry) = words
            .iter_mut()
            .find(|w| w.word.to_lowercase() == word.to_lowercase())
        {
            entry.favorited = favorited;
            self.save(&words);
            true
        } else {
            false
        }
    }

    pub fn update_mastered(&self, word: &str, mastered: bool) -> bool {
        let mut words = self.words.lock().unwrap();
        if let Some(entry) = words
            .iter_mut()
            .find(|w| w.word.to_lowercase() == word.to_lowercase())
        {
            entry.mastered = mastered;
            self.save(&words);
            true
        } else {
            false
        }
    }

    pub fn update_translation(&self, word: &str, translation: &str) -> bool {
        let mut words = self.words.lock().unwrap();
        if let Some(entry) = words
            .iter_mut()
            .find(|w| w.word.to_lowercase() == word.to_lowercase())
        {
            entry.translation = translation.to_string();
            self.save(&words);
            true
        } else {
            false
        }
    }

    pub fn delete_word(&self, word: &str) -> bool {
        let mut words = self.words.lock().unwrap();
        let len_before = words.len();
        words.retain(|w| w.word.to_lowercase() != word.to_lowercase());
        if words.len() < len_before {
            self.save(&words);
            true
        } else {
            false
        }
    }
}
