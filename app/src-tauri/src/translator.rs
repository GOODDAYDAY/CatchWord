use crate::types::{Definition, TranslationResult, TranslationSource};
use std::time::Duration;

pub async fn translate(
    word: &str,
    source: TranslationSource,
    fallback_chain: &[TranslationSource],
) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
    // Build effective chain: [selected source] + [remaining from fallback chain], deduplicated
    let mut chain = vec![source];
    for &s in fallback_chain {
        if !chain.contains(&s) {
            chain.push(s);
        }
    }

    println!("[Translate] 翻译链: {:?}", chain);

    let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;

    for src in &chain {
        println!("[Translate] 尝试源: {}", src.display_name());
        match translate_with_source(word, *src).await {
            Ok(result) => {
                println!("[Translate] {} 成功: {} => {}", src.display_name(), word, result.translation);
                return Ok(result);
            }
            Err(e) => {
                println!("[Translate] {} 失败: {}", src.display_name(), e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "所有翻译源均失败".into()))
}

async fn translate_with_source(
    word: &str,
    source: TranslationSource,
) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
    match source {
        TranslationSource::Google => google::translate(word).await,
        TranslationSource::Bing => bing::translate(word).await,
        TranslationSource::MyMemory => mymemory::translate(word).await,
    }
}

pub fn get_audio_url(word: &str) -> String {
    format!(
        "https://dict.youdao.com/dictvoice?audio={}&type=2",
        urlencoded(word)
    )
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
}

// ─── Google Translate ───

mod google {
    use super::*;

    pub async fn translate(word: &str) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = build_client()?;
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=en&tl=zh-CN&dt=t&dt=bd&dt=rm&q={}",
            urlencoded(word)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Google HTTP {}", status).into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(parse_response(word, &data))
    }

    fn parse_response(word: &str, data: &serde_json::Value) -> TranslationResult {
        let mut translation = String::new();
        let mut phonetic = String::new();
        let mut definitions: Vec<Definition> = Vec::new();

        if let Some(translations) = data.get(0).and_then(|v| v.as_array()) {
            let parts: Vec<String> = translations
                .iter()
                .filter_map(|item| item.get(0).and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect();
            translation = parts.join("");

            if let Some(ph) = translations
                .get(1)
                .and_then(|v| v.get(3))
                .and_then(|v| v.as_str())
            {
                phonetic = ph.to_string();
            }
        }

        if let Some(dict) = data.get(1).and_then(|v| v.as_array()) {
            for entry in dict {
                let pos = entry
                    .get(0)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(meanings) = entry.get(1).and_then(|v| v.as_array()) {
                    for meaning in meanings {
                        if let Some(m) = meaning.as_str() {
                            definitions.push(Definition {
                                part_of_speech: pos.clone(),
                                meaning: m.to_string(),
                            });
                        }
                    }
                }
            }
        }

        TranslationResult {
            word: word.to_string(),
            phonetic,
            translation,
            definitions,
            examples: Vec::new(),
            audio_url: get_audio_url(word),
        }
    }
}

// ─── Bing Translator ───

mod bing {
    use super::*;

    pub async fn translate(word: &str) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = build_client()?;

        // Step 1: Get auth token
        let token_resp = client
            .get("https://edge.microsoft.com/translate/auth")
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;

        if !token_resp.status().is_success() {
            return Err(format!("Bing auth HTTP {}", token_resp.status()).into());
        }

        let token = token_resp.text().await?;

        // Step 2: Translate
        let body = serde_json::json!([{"Text": word}]);
        let resp = client
            .post("https://api-edge.cognitive.microsofttranslator.com/translate?api-version=3.0&from=en&to=zh-Hans")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Bing translate HTTP {}", status).into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(parse_response(word, &data))
    }

    fn parse_response(word: &str, data: &serde_json::Value) -> TranslationResult {
        let translation = data
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("translations"))
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|t| t.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        TranslationResult {
            word: word.to_string(),
            phonetic: String::new(),
            translation,
            definitions: Vec::new(),
            examples: Vec::new(),
            audio_url: get_audio_url(word),
        }
    }
}

// ─── MyMemory ───

mod mymemory {
    use super::*;

    pub async fn translate(word: &str) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = build_client()?;
        let url = format!(
            "https://api.mymemory.translated.net/get?q={}&langpair=en|zh-CN",
            urlencoded(word)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("MyMemory HTTP {}", status).into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(parse_response(word, &data))
    }

    fn parse_response(word: &str, data: &serde_json::Value) -> TranslationResult {
        let translation = data
            .get("responseData")
            .and_then(|rd| rd.get("translatedText"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        TranslationResult {
            word: word.to_string(),
            phonetic: String::new(),
            translation,
            definitions: Vec::new(),
            examples: Vec::new(),
            audio_url: get_audio_url(word),
        }
    }
}
