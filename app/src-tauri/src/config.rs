use crate::types::{AppConfig, TranslationMode, TranslationSource};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Config {
    file_path: PathBuf,
    config: Mutex<AppConfig>,
}

impl Config {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let file_path = app_data_dir.join("config.json");
        let config = Self::load_from_file(&file_path);
        println!("[Config] 加载配置: 模式={}, 源={}", config.mode.display_name(), config.source.display_name());
        Config {
            file_path,
            config: Mutex::new(config),
        }
    }

    fn load_from_file(path: &PathBuf) -> AppConfig {
        match fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    fn save(&self, config: &AppConfig) {
        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(config) {
            let _ = fs::write(&self.file_path, data);
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn set_mode(&self, mode: TranslationMode) {
        let mut config = self.config.lock().unwrap();
        config.mode = mode;
        config.source = mode.default_source();
        println!("[Config] 切换模式: {} (默认源: {})", mode.display_name(), config.source.display_name());
        self.save(&config);
    }

    pub fn set_source(&self, source: TranslationSource) {
        let mut config = self.config.lock().unwrap();
        config.source = source;
        println!("[Config] 切换翻译源: {}", source.display_name());
        self.save(&config);
    }
}
