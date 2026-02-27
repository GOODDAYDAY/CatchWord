use crate::types::{Definition, TranslationResult};
use std::time::Duration;

pub async fn translate(word: &str) -> Result<TranslationResult, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=en&tl=zh-CN&dt=t&dt=bd&dt=rm&q={}",
        urlencoded(word)
    );

    println!("[Translate] 请求: {}", url);

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;

    let status = resp.status();
    println!("[Translate] HTTP 状态: {}", status);

    if !status.is_success() {
        return Err(format!("HTTP {}", status).into());
    }

    let data: serde_json::Value = resp.json().await?;
    Ok(parse_response(word, &data))
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

fn parse_response(word: &str, data: &serde_json::Value) -> TranslationResult {
    let mut translation = String::new();
    let mut phonetic = String::new();
    let mut definitions: Vec<Definition> = Vec::new();
    let examples: Vec<String> = Vec::new();

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

    let audio_url = get_audio_url(word);

    println!("[Translate] 解析完成: {} => {} ({})", word, translation, phonetic);

    TranslationResult {
        word: word.to_string(),
        phonetic,
        translation,
        definitions,
        examples,
        audio_url,
    }
}
