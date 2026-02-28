use std::thread;
use std::time::Duration;

/// Capture selected text: UIA first, then screenshot+OCR fallback.
/// NO keyboard simulation. NO clipboard manipulation. Ever.
pub fn capture_selected_text(mouse_x: f64, mouse_y: f64, ocr_enabled: bool) -> Option<String> {
    thread::sleep(Duration::from_millis(50));

    // --- Strategy A: UIA (fast, works for browsers/terminals/editors) ---
    match uia_get_selected_text() {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                println!("[Capture] UIA 获取选中文本: \"{}\"", trimmed);
                return Some(trimmed);
            }
        }
        Err(e) => {
            println!("[Capture] UIA 失败: {}", e);
        }
    }

    // --- Strategy B: Screenshot + OCR (works for PDF/any app) ---
    if ocr_enabled {
        match ocr_capture(mouse_x, mouse_y) {
            Ok(word) => {
                if !word.is_empty() {
                    println!("[Capture] OCR 获取到单词: \"{}\"", word);
                    return Some(word);
                }
            }
            Err(e) => {
                println!("[Capture] OCR 失败: {}", e);
            }
        }
    } else {
        println!("[Capture] OCR 兜底已关闭，跳过");
    }

    None
}

// ===========================================================================
// UIA  (策略 1→2→3)
// ===========================================================================

fn uia_get_selected_text() -> Result<String, String> {
    #[cfg(windows)]
    {
        uia_impl()
    }
    #[cfg(not(windows))]
    {
        Err("UIA is only available on Windows".into())
    }
}

#[cfg(windows)]
fn uia_impl() -> Result<String, String> {
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    unsafe {
        if let Err(e) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
            return Err(format!("CoInitializeEx failed: {e}"));
        }

        let result = (|| -> Result<String, String> {
            let uia: IUIAutomation =
                windows::Win32::System::Com::CoCreateInstance(
                    &CUIAutomation,
                    None,
                    windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
                )
                .map_err(|e| format!("CoCreateInstance failed: {e}"))?;

            let focused = uia
                .GetFocusedElement()
                .map_err(|e| format!("GetFocusedElement failed: {e}"))?;

            // Strategy 1: TextPattern on focused element
            if let Ok(text) = try_get_selection(&focused) {
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }

            // Strategy 2: Walk UP ancestors
            let walker = uia
                .RawViewWalker()
                .map_err(|e| format!("RawViewWalker: {e}"))?;

            let mut current = focused.clone();
            for _depth in 1..=15 {
                match walker.GetParentElement(&current) {
                    Ok(parent) => {
                        if let Ok(text) = try_get_selection(&parent) {
                            if !text.trim().is_empty() {
                                return Ok(text);
                            }
                        }
                        current = parent;
                    }
                    Err(_) => break,
                }
            }

            // Strategy 3: Walk DOWN children
            if let Ok(cw) = uia.RawViewWalker() {
                if let Ok(first) = cw.GetFirstChildElement(&focused) {
                    let mut child = first;
                    for _ in 0..20 {
                        if let Ok(text) = try_get_selection(&child) {
                            if !text.trim().is_empty() {
                                return Ok(text);
                            }
                        }
                        match cw.GetNextSiblingElement(&child) {
                            Ok(next) => child = next,
                            Err(_) => break,
                        }
                    }
                }
            }

            Err("UIA: 未找到 TextPattern".into())
        })();

        CoUninitialize();
        result
    }
}

#[cfg(windows)]
unsafe fn try_get_selection(
    element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) -> Result<String, String> {
    use windows::Win32::UI::Accessibility::{
        IUIAutomationTextPattern, UIA_TextPatternId,
    };
    use windows::core::Interface;

    let pat = element
        .GetCurrentPattern(UIA_TextPatternId)
        .map_err(|e| format!("{e}"))?;
    let tp: IUIAutomationTextPattern = pat.cast().map_err(|e| format!("{e}"))?;
    let ranges = tp.GetSelection().map_err(|e| format!("{e}"))?;
    let len = ranges.Length().map_err(|e| format!("{e}"))?;
    if len == 0 {
        return Err("empty".into());
    }
    let range = ranges.GetElement(0).map_err(|e| format!("{e}"))?;
    let bstr = range.GetText(1024).map_err(|e| format!("{e}"))?;
    Ok(bstr.to_string())
}

// ===========================================================================
// Screenshot + Windows OCR  (适用于 PDF / 任何应用)
// ===========================================================================

fn ocr_capture(mouse_x: f64, mouse_y: f64) -> Result<String, String> {
    #[cfg(windows)]
    {
        ocr_capture_impl(mouse_x, mouse_y)
    }
    #[cfg(not(windows))]
    {
        let _ = (mouse_x, mouse_y);
        Err("OCR is only available on Windows".into())
    }
}

#[cfg(windows)]
fn ocr_capture_impl(mouse_x: f64, mouse_y: f64) -> Result<String, String> {
    const W: i32 = 300;
    const H: i32 = 60;

    let cx = mouse_x as i32;
    let cy = mouse_y as i32;
    let left = cx - W / 2;
    let top = cy - H / 2;

    println!("[Capture] OCR: 截图 {}x{} @ ({}, {})", W, H, left, top);

    // 1. Screenshot via GDI
    let pixels = screenshot_region(left, top, W, H)?;

    // 2. Build SoftwareBitmap and run OCR
    let word = run_ocr(&pixels, W, H)?;

    Ok(word)
}

/// Capture a screen region and return BGRA pixels (top-down).
#[cfg(windows)]
fn screenshot_region(left: i32, top: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Gdi::*;

    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, hbm.into());

        let _ = BitBlt(hdc_mem, 0, 0, w, h, Some(hdc_screen), left, top, SRCCOPY);

        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // negative = top-down
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };

        let buf_size = (w * h * 4) as usize;
        let mut pixels = vec![0u8; buf_size];

        GetDIBits(
            hdc_mem,
            hbm,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        // Set alpha to 255 (GetDIBits leaves it undefined for 32-bit)
        for i in (3..buf_size).step_by(4) {
            pixels[i] = 255;
        }

        Ok(pixels)
    }
}

/// Run Windows OCR on pixel data, return the word closest to the center.
#[cfg(windows)]
fn run_ocr(pixels: &[u8], w: i32, h: i32) -> Result<String, String> {
    use windows::core::{Interface, HSTRING};
    use windows::Foundation::AsyncStatus;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::*;
    use windows::Media::Ocr::OcrEngine;
    use windows::Win32::System::WinRT::IMemoryBufferByteAccess;

    unsafe {
        // Create SoftwareBitmap
        let bmp = SoftwareBitmap::Create(
            BitmapPixelFormat::Bgra8,
            w,
            h,
        )
        .map_err(|e| format!("SoftwareBitmap::Create: {e}"))?;

        // Copy pixels into bitmap
        {
            let buffer = bmp
                .LockBuffer(BitmapBufferAccessMode::Write)
                .map_err(|e| format!("LockBuffer: {e}"))?;
            let reference = buffer
                .CreateReference()
                .map_err(|e| format!("CreateReference: {e}"))?;
            let byte_access: IMemoryBufferByteAccess = reference
                .cast()
                .map_err(|e| format!("cast IMemoryBufferByteAccess: {e}"))?;

            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut capacity: u32 = 0;
            byte_access
                .GetBuffer(&mut data_ptr, &mut capacity)
                .map_err(|e| format!("GetBuffer: {e}"))?;

            let copy_len = pixels.len().min(capacity as usize);
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), data_ptr, copy_len);
        } // buffer lock released here

        // Create OCR engine for English
        let lang = Language::CreateLanguage(&HSTRING::from("en"))
            .map_err(|e| format!("Language: {e}"))?;
        let engine = OcrEngine::TryCreateFromLanguage(&lang)
            .map_err(|e| format!("OcrEngine: {e}"))?;

        // Run recognition (async → spin-wait with timeout)
        let op = engine
            .RecognizeAsync(&bmp)
            .map_err(|e| format!("RecognizeAsync: {e}"))?;

        let start = std::time::Instant::now();
        loop {
            match op.Status().map_err(|e| format!("Status: {e}"))? {
                AsyncStatus::Completed => break,
                AsyncStatus::Error => return Err("OCR async error".into()),
                AsyncStatus::Canceled => return Err("OCR canceled".into()),
                _ => {
                    if start.elapsed() > std::time::Duration::from_secs(3) {
                        return Err("OCR timeout".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }

        let result = op.GetResults().map_err(|e| format!("GetResults: {e}"))?;

        // Find the word closest to the center of the capture region
        let center_x = w as f32 / 2.0;
        let center_y = h as f32 / 2.0;

        let mut best_word = String::new();
        let mut best_dist = f32::MAX;

        let lines = result.Lines().map_err(|e| format!("Lines: {e}"))?;
        for line in &lines {
            let words = line.Words().map_err(|e| format!("Words: {e}"))?;
            for word in &words {
                let text = word.Text().map_err(|e| format!("Text: {e}"))?;
                let rect = word.BoundingRect().map_err(|e| format!("Rect: {e}"))?;

                let word_cx = rect.X + rect.Width / 2.0;
                let word_cy = rect.Y + rect.Height / 2.0;
                let dist = ((word_cx - center_x).powi(2) + (word_cy - center_y).powi(2)).sqrt();

                let s = text.to_string();
                if dist < best_dist && !s.is_empty() {
                    best_dist = dist;
                    best_word = s;
                }
            }
        }

        if best_word.is_empty() {
            let full = result.Text().map(|t| t.to_string()).unwrap_or_default();
            println!("[Capture] OCR 全文: \"{}\"", full);
            Err("OCR 未识别到单词".into())
        } else {
            println!(
                "[Capture] OCR 最近单词: \"{}\" (距中心 {:.0}px)",
                best_word, best_dist
            );
            Ok(best_word)
        }
    }
}

// ===========================================================================
// Utility
// ===========================================================================

pub fn is_english_word(text: &str) -> bool {
    if text.len() < 2 || text.len() > 45 {
        return false;
    }
    text.chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
        && text.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
        && text.chars().last().map_or(false, |c| c.is_ascii_alphabetic())
}
