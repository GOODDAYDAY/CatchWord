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
    Emitter, Manager, WebviewWindow,
};
use wordbook::Wordbook;

struct AppState {
    wordbook: Wordbook,
    capture_enabled: Mutex<bool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            println!("[CatchWord] ===== 应用启动 =====");

            // Initialize wordbook
            let app_data_dir = app.path().app_data_dir()?;
            println!("[CatchWord] 数据目录: {:?}", app_data_dir);
            let wordbook = Wordbook::new(app_data_dir);
            let state = Arc::new(AppState {
                wordbook,
                capture_enabled: Mutex::new(true),
            });
            app.manage(state.clone());

            // Setup system tray
            setup_tray(app)?;
            println!("[CatchWord] 系统托盘已创建");

            // Start global mouse hook
            let app_handle = app.handle().clone();
            start_capture_loop(app_handle.clone(), state);
            println!("[CatchWord] 全局鼠标钩子已启动");

            // Startup self-test: translate "hello" and show popup at screen center
            println!("[CatchWord] 启动自测: 翻译 hello...");
            let test_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for the webview to be ready
                std::thread::sleep(std::time::Duration::from_secs(2));

                let result = match translator::translate("hello").await {
                    Ok(r) => {
                        println!("[CatchWord] 自测翻译成功: {} => {}", r.word, r.translation);
                        r
                    }
                    Err(e) => {
                        eprintln!("[CatchWord] 自测翻译失败（网络问题？）: {}", e);
                        println!("[CatchWord] 使用 mock 数据展示浮窗...");
                        types::TranslationResult {
                            word: "hello".to_string(),
                            phonetic: "həˈloʊ".to_string(),
                            translation: "你好；喂（自测 mock 数据）".to_string(),
                            definitions: vec![
                                types::Definition {
                                    part_of_speech: "interj.".to_string(),
                                    meaning: "用于问候、接电话或引起注意".to_string(),
                                },
                                types::Definition {
                                    part_of_speech: "n.".to_string(),
                                    meaning: "招呼；问候".to_string(),
                                },
                            ],
                            examples: vec!["Hello, how are you?".to_string()],
                            audio_url: String::new(),
                        }
                    }
                };

                if let Some(popup) = test_handle.get_webview_window("popup") {
                    show_popup(&popup, &result, 500.0, 300.0);
                    println!("[CatchWord] 自测浮窗已弹出！5秒后自动关闭");
                    // Auto-hide after 5 seconds so it doesn't block the user
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let _ = popup.hide();
                    println!("[CatchWord] 自测浮窗已关闭，进入正常工作模式");
                } else {
                    eprintln!("[CatchWord] 自测失败: 找不到 popup 窗口");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let toggle = MenuItemBuilder::with_id("toggle", "取词：已开启").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&toggle, &quit]).build()?;

    let icon_data: Vec<u8> = vec![66, 133, 244, 255].repeat(16 * 16);
    let icon = Image::new_owned(icon_data, 16, 16);

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("CatchWord - 全局取词翻译")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let mut enabled = state.capture_enabled.lock().unwrap();
                    *enabled = !*enabled;
                    let label = if *enabled {
                        "取词：已开启"
                    } else {
                        "取词：已关闭"
                    };
                    println!("[CatchWord] 取词状态: {}", label);
                    let _ = toggle.set_text(label);
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
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
                    let text = match capture::capture_selected_text(x, y) {
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

                    {
                        let mut translating = translating_flag.lock().unwrap();
                        *translating = true;
                    }

                    tauri::async_runtime::spawn(async move {
                        match translator::translate(&word).await {
                            Ok(result) => {
                                println!("[CatchWord] 翻译成功: {} => {}", result.word, result.translation);

                                state_ref.wordbook.add_word(
                                    &result.word,
                                    &result.translation,
                                    &result.phonetic,
                                    &result.examples,
                                    "",
                                );

                                if let Some(popup) = handle.get_webview_window("popup") {
                                    show_popup(&popup, &result, x, y);
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

fn show_popup(popup: &WebviewWindow, result: &types::TranslationResult, mouse_x: f64, mouse_y: f64) {
    use tauri::PhysicalPosition;

    let offset = 15.0;

    let x = mouse_x + offset;
    let y = mouse_y + offset;

    // rdev gives physical pixels, so use PhysicalPosition
    let _ = popup.set_position(PhysicalPosition::new(x as i32, y as i32));
    let _ = popup.emit("translation-result", result);
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
