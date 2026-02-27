use rdev::{listen, Button, Event, EventType};
use std::sync::mpsc;
use std::time::Instant;

pub enum HookEvent {
    PossibleSelection { x: f64, y: f64 },
    SingleClick { x: f64, y: f64 },
}

const DOUBLE_CLICK_MS: u128 = 500;
const DRAG_THRESHOLD: f64 = 5.0;

pub fn start_hook(tx: mpsc::Sender<HookEvent>) {
    std::thread::spawn(move || {
        println!("[Hook] 全局鼠标监听线程启动");

        let mut mouse_pos = (0.0f64, 0.0f64);
        let mut mouse_down_pos = (0.0f64, 0.0f64);
        let mut last_click_time = Instant::now();
        let mut click_count: u32 = 0;

        let callback = move |event: Event| {
            match event.event_type {
                EventType::MouseMove { x, y } => {
                    mouse_pos = (x, y);
                }
                EventType::ButtonPress(Button::Left) => {
                    mouse_down_pos = mouse_pos;
                }
                EventType::ButtonRelease(Button::Left) => {
                    let now = Instant::now();
                    let elapsed = now.duration_since(last_click_time).as_millis();

                    if elapsed < DOUBLE_CLICK_MS {
                        click_count += 1;
                    } else {
                        click_count = 1;
                    }
                    last_click_time = now;

                    let is_double_click = click_count >= 2;
                    let dx = (mouse_pos.0 - mouse_down_pos.0).abs();
                    let dy = (mouse_pos.1 - mouse_down_pos.1).abs();
                    let is_drag = dx > DRAG_THRESHOLD || dy > DRAG_THRESHOLD;

                    if is_double_click || is_drag {
                        println!("[Hook] 检测到选词 (双击={}, 拖选={}) 位置=({:.0}, {:.0})",
                            is_double_click, is_drag, mouse_pos.0, mouse_pos.1);
                        let _ = tx.send(HookEvent::PossibleSelection {
                            x: mouse_pos.0,
                            y: mouse_pos.1,
                        });
                        click_count = 0;
                    } else {
                        let _ = tx.send(HookEvent::SingleClick {
                            x: mouse_pos.0,
                            y: mouse_pos.1,
                        });
                    }
                }
                _ => {}
            }
        };

        if let Err(e) = listen(callback) {
            eprintln!("[Hook] 全局钩子启动失败: {:?}", e);
        }
    });
}
