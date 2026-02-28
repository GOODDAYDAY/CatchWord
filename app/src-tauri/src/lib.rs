mod capture;
mod hook;
mod translator;
mod types;
mod wordbook;

use hook::HookEvent;
use std::sync::{Arc, Mutex, mpsc};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use wordbook::Wordbook;

#[tauri::command]
fn get_word(state: State<'_, Arc<AppState>>, word: String) -> Option<types::WordEntry> {
    state.wordbook.get_word(&word)
}

#[tauri::command]
fn set_favorited(state: State<'_, Arc<AppState>>, word: String, favorited: bool) -> bool {
    state.wordbook.update_favorited(&word, favorited)
}

#[tauri::command]
fn set_mastered(state: State<'_, Arc<AppState>>, word: String, mastered: bool) -> bool {
    state.wordbook.update_mastered(&word, mastered)
}

#[tauri::command]
fn list_words(state: State<'_, Arc<AppState>>) -> Vec<types::WordEntry> {
    state.wordbook.list_words()
}

#[tauri::command]
fn delete_word(state: State<'_, Arc<AppState>>, word: String) -> bool {
    state.wordbook.delete_word(&word)
}

#[tauri::command]
fn update_translation(state: State<'_, Arc<AppState>>, word: String, translation: String) -> bool {
    state.wordbook.update_translation(&word, &translation)
}

struct AppState {
    wordbook: Wordbook,
    capture_enabled: Mutex<bool>,
    auto_pronounce: Mutex<bool>,
    ocr_enabled: Mutex<bool>,
    wordbook_enabled: Mutex<bool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_word, set_favorited, set_mastered,
            list_words, delete_word, update_translation,
        ])
        .setup(|app| {
            println!("[CatchWord] ===== 应用启动 =====");

            // Initialize wordbook
            let app_data_dir = app.path().app_data_dir()?;
            println!("[CatchWord] 数据目录: {:?}", app_data_dir);
            let wordbook = Wordbook::new(app_data_dir);
            let state = Arc::new(AppState {
                wordbook,
                capture_enabled: Mutex::new(true),
                auto_pronounce: Mutex::new(true),
                ocr_enabled: Mutex::new(true),
                wordbook_enabled: Mutex::new(true),
            });
            app.manage(state.clone());

            // Setup system tray
            setup_tray(app)?;
            println!("[CatchWord] 系统托盘已创建");

            // Start global mouse hook
            let app_handle = app.handle().clone();
            start_capture_loop(app_handle.clone(), state);
            println!("[CatchWord] 全局鼠标钩子已启动");

            // Self-test popup: only in debug builds
            #[cfg(debug_assertions)]
            {
                println!("[CatchWord] 启动自测: 翻译 hello...");
                let test_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    std::thread::sleep(std::time::Duration::from_secs(2));

                    let result = match translator::translate("hello").await {
                        Ok(r) => {
                            println!("[CatchWord] 自测翻译成功: {} => {}", r.word, r.translation);
                            r
                        }
                        Err(e) => {
                            eprintln!("[CatchWord] 自测翻译失败: {}", e);
                            types::TranslationResult {
                                word: "hello".to_string(),
                                phonetic: "həˈloʊ".to_string(),
                                translation: "你好；喂（自测 mock）".to_string(),
                                definitions: vec![
                                    types::Definition {
                                        part_of_speech: "interj.".to_string(),
                                        meaning: "用于问候".to_string(),
                                    },
                                ],
                                examples: vec!["Hello, how are you?".to_string()],
                                audio_url: String::new(),
                            }
                        }
                    };

                    if let Some(popup) = test_handle.get_webview_window("popup") {
                        show_popup(&popup, &result, 500.0, 300.0, true, false, false);
                        println!("[CatchWord] 自测浮窗已弹出！5秒后自动关闭");
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        let _ = popup.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_capture = MenuItemBuilder::with_id("toggle_capture", "取词：已开启").build(app)?;
    let toggle_pronounce = MenuItemBuilder::with_id("toggle_pronounce", "自动发音：已开启").build(app)?;
    let toggle_ocr = MenuItemBuilder::with_id("toggle_ocr", "OCR 兜底：已开启").build(app)?;
    let toggle_wordbook = MenuItemBuilder::with_id("toggle_wordbook", "自动记单词：已开启").build(app)?;
    let open_wordbook = MenuItemBuilder::with_id("open_wordbook", "生词本").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&toggle_capture, &toggle_pronounce, &toggle_ocr, &toggle_wordbook, &open_wordbook, &quit])
        .build()?;

    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("CatchWord - 全局取词翻译")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let Some(state) = app.try_state::<Arc<AppState>>() else { return };
            match event.id().as_ref() {
                "toggle_capture" => {
                    let mut v = state.capture_enabled.lock().unwrap();
                    *v = !*v;
                    let label = if *v { "取词：已开启" } else { "取词：已关闭" };
                    println!("[CatchWord] {}", label);
                    let _ = toggle_capture.set_text(label);
                }
                "toggle_pronounce" => {
                    let mut v = state.auto_pronounce.lock().unwrap();
                    *v = !*v;
                    let label = if *v { "自动发音：已开启" } else { "自动发音：已关闭" };
                    println!("[CatchWord] {}", label);
                    let _ = toggle_pronounce.set_text(label);
                }
                "toggle_ocr" => {
                    let mut v = state.ocr_enabled.lock().unwrap();
                    *v = !*v;
                    let label = if *v { "OCR 兜底：已开启" } else { "OCR 兜底：已关闭" };
                    println!("[CatchWord] {}", label);
                    let _ = toggle_ocr.set_text(label);
                }
                "toggle_wordbook" => {
                    let mut v = state.wordbook_enabled.lock().unwrap();
                    *v = !*v;
                    let label = if *v { "自动记单词：已开启" } else { "自动记单词：已关闭" };
                    println!("[CatchWord] {}", label);
                    let _ = toggle_wordbook.set_text(label);
                }
                "open_wordbook" => {
                    if let Some(win) = app.get_webview_window("wordbook") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        return;
                    }
                    let _ = WebviewWindowBuilder::new(app, "wordbook", WebviewUrl::App("wordbook.html".into()))
                        .title("CatchWord - 生词本")
                        .inner_size(900.0, 650.0)
                        .min_inner_size(600.0, 400.0)
                        .resizable(true)
                        .decorations(true)
                        .center()
                        .build();
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

fn start_capture_loop(app_handle: tauri::AppHandle, state: Arc<AppState>) {
    let (tx, rx) = mpsc::channel::<HookEvent>();
    hook::start_hook(tx);

    std::thread::spawn(move || {
        let is_translating = Arc::new(Mutex::new(false));

        while let Ok(event) = rx.recv() {
            {
                let enabled = state.capture_enabled.lock().unwrap();
                if !*enabled {
                    continue;
                }
            }

            match event {
                HookEvent::PossibleSelection { x, y } => {
                    // If the click landed inside the popup, ignore it —
                    // the user is interacting with the popup (e.g. play button),
                    // not selecting a new word.
                    if let Some(popup) = app_handle.get_webview_window("popup") {
                        if popup.is_visible().unwrap_or(false)
                            && is_point_in_window(&popup, x, y)
                        {
                            println!("[CatchWord] 点击在弹窗内，跳过选词");
                            continue;
                        }
                    }

                    {
                        let translating = is_translating.lock().unwrap();
                        if *translating {
                            println!("[CatchWord] 跳过：正在翻译中...");
                            continue;
                        }
                    }

                    // Hide existing popup
                    if let Some(popup) = app_handle.get_webview_window("popup") {
                        let _ = popup.hide();
                    }

                    println!("[CatchWord] 检测到选词事件 ({}, {}), 正在捕获文本...", x, y);

                    // Capture selected text
                    let ocr_on = *state.ocr_enabled.lock().unwrap();
                    let text = match capture::capture_selected_text(x, y, ocr_on) {
                        Some(t) => {
                            println!("[CatchWord] 捕获到文本: \"{}\"", t);
                            if capture::is_english_word(&t) {
                                t
                            } else {
                                println!("[CatchWord] 不是英文单词，跳过");
                                continue;
                            }
                        }
                        None => {
                            println!("[CatchWord] 未捕获到文本");
                            continue;
                        }
                    };

                    println!("[CatchWord] 开始翻译: {}", text);

                    let handle = app_handle.clone();
                    let state_ref = state.clone();
                    let word = text.clone();
                    let translating_flag = is_translating.clone();
                    let save_word = *state.wordbook_enabled.lock().unwrap();
                    let auto_play = *state.auto_pronounce.lock().unwrap();

                    {
                        let mut translating = translating_flag.lock().unwrap();
                        *translating = true;
                    }

                    tauri::async_runtime::spawn(async move {
                        match translator::translate(&word).await {
                            Ok(result) => {
                                println!("[CatchWord] 翻译成功: {} => {}", result.word, result.translation);

                                if save_word {
                                    state_ref.wordbook.add_word(
                                        &result.word,
                                        &result.translation,
                                        &result.phonetic,
                                        &result.examples,
                                        "",
                                    );
                                }

                                // Query current wordbook state for the popup
                                let (favorited, mastered) = state_ref.wordbook.get_word(&result.word)
                                    .map(|e| (e.favorited, e.mastered))
                                    .unwrap_or((false, false));

                                if let Some(popup) = handle.get_webview_window("popup") {
                                    show_popup(&popup, &result, x, y, auto_play, favorited, mastered);
                                    println!("[CatchWord] 浮窗已显示");
                                } else {
                                    println!("[CatchWord] 错误: 找不到 popup 窗口");
                                }
                            }
                            Err(e) => {
                                eprintln!("[CatchWord] 翻译失败: {}", e);
                            }
                        }
                        let mut translating = translating_flag.lock().unwrap();
                        *translating = false;
                    });
                }
                HookEvent::SingleClick { x, y } => {
                    if let Some(popup) = app_handle.get_webview_window("popup") {
                        if popup.is_visible().unwrap_or(false) {
                            if !is_point_in_window(&popup, x, y) {
                                let _ = popup.hide();
                            }
                        }
                    }
                }
            }
        }
    });
}

fn show_popup(popup: &WebviewWindow, result: &types::TranslationResult, mouse_x: f64, mouse_y: f64, auto_play: bool, favorited: bool, mastered: bool) {
    use tauri::PhysicalPosition;

    let offset = 15.0;

    let x = mouse_x + offset;
    let y = mouse_y + offset;

    // rdev gives physical pixels, so use PhysicalPosition
    let _ = popup.set_position(PhysicalPosition::new(x as i32, y as i32));

    // Send result + auto_play flag + wordbook state to frontend
    let payload = serde_json::json!({
        "word": result.word,
        "phonetic": result.phonetic,
        "translation": result.translation,
        "definitions": result.definitions,
        "examples": result.examples,
        "audio_url": result.audio_url,
        "auto_play": auto_play,
        "favorited": favorited,
        "mastered": mastered,
    });
    let _ = popup.emit("translation-result", payload);
    let _ = popup.show();
    // Do NOT call set_focus() — the popup is alwaysOnTop so it's visible,
    // and stealing focus would break UIA's GetFocusedElement() on next capture.
}

fn is_point_in_window(window: &WebviewWindow, x: f64, y: f64) -> bool {
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let wx = pos.x as f64;
        let wy = pos.y as f64;
        let ww = size.width as f64;
        let wh = size.height as f64;
        x >= wx && x <= wx + ww && y >= wy && y <= wy + wh
    } else {
        false
    }
}
