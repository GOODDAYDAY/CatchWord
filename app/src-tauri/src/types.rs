use serde::{Deserialize, Serialize};
use std::fmt;

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
    #[serde(default)]
    pub mastered: bool,
    #[serde(default)]
    pub favorited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslationSource {
    Google,
    Bing,
    MyMemory,
}

impl TranslationSource {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Google => "Google Translate",
            Self::Bing => "Bing Translator",
            Self::MyMemory => "MyMemory",
        }
    }

    pub fn menu_id(&self) -> &str {
        match self {
            Self::Google => "source_google",
            Self::Bing => "source_bing",
            Self::MyMemory => "source_mymemory",
        }
    }

    pub fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            "source_google" => Some(Self::Google),
            "source_bing" => Some(Self::Bing),
            "source_mymemory" => Some(Self::MyMemory),
            _ => None,
        }
    }

    pub fn all() -> &'static [TranslationSource] {
        &[Self::Google, Self::Bing, Self::MyMemory]
    }
}

impl fmt::Display for TranslationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslationMode {
    Overseas,
    Mainland,
}

impl TranslationMode {
    pub fn default_source(&self) -> TranslationSource {
        match self {
            Self::Overseas => TranslationSource::Google,
            Self::Mainland => TranslationSource::Bing,
        }
    }

    pub fn fallback_chain(&self) -> Vec<TranslationSource> {
        match self {
            Self::Overseas => vec![
                TranslationSource::Google,
                TranslationSource::Bing,
                TranslationSource::MyMemory,
            ],
            Self::Mainland => vec![
                TranslationSource::Bing,
                TranslationSource::MyMemory,
            ],
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Overseas => "海外模式",
            Self::Mainland => "大陆模式",
        }
    }

    pub fn menu_id(&self) -> &str {
        match self {
            Self::Overseas => "mode_overseas",
            Self::Mainland => "mode_mainland",
        }
    }

    pub fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            "mode_overseas" => Some(Self::Overseas),
            "mode_mainland" => Some(Self::Mainland),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode: TranslationMode,
    pub source: TranslationSource,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: TranslationMode::Overseas,
            source: TranslationSource::Google,
        }
    }
}
