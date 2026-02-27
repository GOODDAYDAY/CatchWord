use std::thread;
use std::time::Duration;

/// Capture selected text via Windows UI Automation only.
/// NO keyboard simulation. NO clipboard manipulation. Ever.
pub fn capture_selected_text() -> Option<String> {
    // Small delay to let the OS finish processing the double-click selection
    thread::sleep(Duration::from_millis(50));

    match uia_get_selected_text() {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                println!("[Capture] UIA 获取选中文本: \"{}\"", trimmed);
                Some(trimmed)
            } else {
                println!("[Capture] UIA 返回空文本");
                None
            }
        }
        Err(e) => {
            println!("[Capture] UIA 失败: {}", e);
            None
        }
    }
}

// ===========================================================================
// Windows UI Automation  (策略 1→2→3)
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

            log_element_info("focused", &focused);

            // Strategy 1: TextPattern on focused element directly
            if let Ok(text) = try_get_selection(&focused) {
                if !text.trim().is_empty() {
                    println!("[Capture] 策略1: 焦点元素命中");
                    return Ok(text);
                }
            }

            // Strategy 2: Walk UP — browsers often focus a child of the
            // Document element that carries TextPattern.
            let walker = uia
                .RawViewWalker()
                .map_err(|e| format!("RawViewWalker: {e}"))?;

            let mut current = focused.clone();
            for depth in 1..=15 {
                match walker.GetParentElement(&current) {
                    Ok(parent) => {
                        if let Ok(text) = try_get_selection(&parent) {
                            if !text.trim().is_empty() {
                                println!("[Capture] 策略2: ancestor[{depth}] 命中");
                                return Ok(text);
                            }
                        }
                        current = parent;
                    }
                    Err(_) => break,
                }
            }

            // Strategy 3: Walk DOWN — some apps expose TextPattern on a
            // child document element.
            if let Ok(child_walker) = uia.RawViewWalker() {
                if let Ok(first_child) = child_walker.GetFirstChildElement(&focused) {
                    let mut child = first_child;
                    for idx in 0..20 {
                        if let Ok(text) = try_get_selection(&child) {
                            if !text.trim().is_empty() {
                                println!("[Capture] 策略3: child[{idx}] 命中");
                                return Ok(text);
                            }
                        }
                        match child_walker.GetNextSiblingElement(&child) {
                            Ok(next) => child = next,
                            Err(_) => break,
                        }
                    }
                }
            }

            Err("未找到支持 TextPattern 的元素".into())
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

    let pattern_obj = element
        .GetCurrentPattern(UIA_TextPatternId)
        .map_err(|e| format!("no TextPattern: {e}"))?;

    let text_pattern: IUIAutomationTextPattern = pattern_obj
        .cast()
        .map_err(|e| format!("cast: {e}"))?;

    let ranges = text_pattern
        .GetSelection()
        .map_err(|e| format!("GetSelection: {e}"))?;

    let len = ranges.Length().map_err(|e| format!("Length: {e}"))?;
    if len == 0 {
        return Err("no selection ranges".into());
    }

    let range = ranges
        .GetElement(0)
        .map_err(|e| format!("GetElement(0): {e}"))?;

    let bstr = range
        .GetText(1024)
        .map_err(|e| format!("GetText: {e}"))?;

    Ok(bstr.to_string())
}

#[cfg(windows)]
unsafe fn log_element_info(
    label: &str,
    element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) {
    let ctrl_type = element.CurrentControlType().unwrap_or_default();
    let name = element
        .CurrentName()
        .map(|b| b.to_string())
        .unwrap_or_default();
    let class = element
        .CurrentClassName()
        .map(|b| b.to_string())
        .unwrap_or_default();
    println!(
        "[Capture]   {}: type={} class=\"{}\" name=\"{}\"",
        label, ctrl_type.0, class, name
    );
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
