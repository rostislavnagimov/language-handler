use crate::core::macos_api::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFGetTypeID, CFRelease, CFStringCreateWithCString,
    CFStringGetCString, CFStringGetLength, CFStringGetTypeID, TISCopyInputSourceForLanguage,
    TISCreateInputSourceList, TISGetInputSourceProperty, TISSelectInputSource,
    K_TIS_PROPERTY_INPUT_SOURCE_ID, K_TIS_PROPERTY_LOCALIZED_NAME, K_UTF8_ENCODING,
};
use crate::core::monitor;
use crate::state;

use cocoa::base::nil;
use cocoa::foundation::NSString;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::thread;
use std::time::Duration;

pub fn is_target_layout(current_layout: &str, target_layout: &str) -> bool {
    match target_layout.to_uppercase().as_str() {
        "US" | "EN" | "ENGLISH" => {
            current_layout.contains("U.S.")
                || current_layout.contains("US")
                || current_layout.contains("English")
                || current_layout.contains("com.apple.keylayout.US")
                || current_layout.contains("ABC")
        }
        "RU" | "RUSSIAN" => {
            current_layout.contains("Russian")
                || current_layout.contains("Русская")
                || current_layout.contains("com.apple.keylayout.Russian")
        }
        "CN" | "CHINESE" | "PINYIN" | "ZH" => {
            current_layout.to_uppercase().contains("PINYIN")
                || current_layout.to_uppercase().contains("SIMPLIFIED")
                || current_layout.contains("简体")
                || current_layout.contains("com.apple.keylayout.PinyinSimplified")
                || current_layout.contains("com.apple.inputmethod.SCIM.Pinyin")
        }
        "HI" | "HINDI" | "DEVANAGARI" => {
            current_layout.to_uppercase().contains("HINDI")
                || current_layout.to_uppercase().contains("DEVANAGARI")
                || current_layout.contains("देवनागरी")
                || current_layout.contains("com.apple.keylayout.Devanagari-QWERTY")
                || current_layout.contains("com.apple.keylayout.Hindi-QWERTY")
        }
        _ => current_layout
            .to_uppercase()
            .contains(&target_layout.to_uppercase()),
    }
}

/// Attempts to switch the system keyboard layout to the specified target layout string.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It calls numerous FFI functions (TIS... and CF...) to interact with macOS APIs.
/// 2. It directly manipulates system-wide state (the active keyboard layout).
/// 3. It calls `monitor::update_keyboard_layout()`, which is also unsafe.
///     The caller is responsible for ensuring that `target_layout` represents a valid
///     intended layout name or identifier.
pub unsafe fn switch_to_layout(target_layout: &str) {
    let target_upper = target_layout.to_uppercase();

    let search_patterns = match target_upper.as_str() {
        "US" | "EN" | "ENGLISH" => vec!["com.apple.keylayout.US", "com.apple.keylayout.ABC", "US"],
        "RU" | "RUSSIAN" => vec!["com.apple.keylayout.Russian", "Russian"],
        "CN" | "CHINESE" | "PINYIN" | "ZH" => vec![
            "com.apple.keylayout.PinyinSimplified",
            "com.apple.inputmethod.SCIM.Pinyin",
            "Pinyin",
            "简体",
        ],
        "HI" | "HINDI" | "DEVANAGARI" => vec![
            "com.apple.keylayout.Devanagari-QWERTY",
            "com.apple.keylayout.Hindi-QWERTY",
            "Hindi",
            "Devanagari",
            "देवनागरी",
        ],
        _ => vec![target_layout],
    };

    if target_upper == "US" || target_upper == "EN" || target_upper == "ENGLISH" {
        let us_lang =
            CFStringCreateWithCString(nil, c"en".as_ptr() as *const c_char, K_UTF8_ENCODING);
        if us_lang != nil {
            let us_source = TISCopyInputSourceForLanguage(us_lang);
            CFRelease(us_lang);

            if us_source != nil {
                let result = TISSelectInputSource(us_source);
                CFRelease(us_source);

                if result == 0 {
                    println!("Successfully switched to layout: {}", target_layout);
                    thread::sleep(Duration::from_millis(100));
                    monitor::update_keyboard_layout();
                    return;
                }
            }
        }
    }

    let input_sources = TISCreateInputSourceList(nil, true);
    if input_sources != nil {
        let count = CFArrayGetCount(input_sources);

        for i in 0..count {
            let source = CFArrayGetValueAtIndex(input_sources, i);
            if source != nil {
                let id_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_INPUT_SOURCE_ID);
                let name_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_LOCALIZED_NAME);

                let source_id = TISGetInputSourceProperty(source, id_key);
                let source_name = TISGetInputSourceProperty(source, name_key);

                let mut found = false;

                if source_id != nil && CFGetTypeID(source_id) == CFStringGetTypeID() {
                    let length = CFStringGetLength(source_id);
                    if length > 0 {
                        let buffer_size = (length + 1) * 4;
                        let mut buffer = vec![0u8; buffer_size as usize];

                        if CFStringGetCString(
                            source_id,
                            buffer.as_mut_ptr() as *mut c_char,
                            buffer_size,
                            K_UTF8_ENCODING,
                        ) {
                            if let Ok(id_str) =
                                CStr::from_ptr(buffer.as_ptr() as *const c_char).to_str()
                            {
                                for pattern in &search_patterns {
                                    if id_str.contains(pattern) {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                if !found && source_name != nil && CFGetTypeID(source_name) == CFStringGetTypeID() {
                    let length = CFStringGetLength(source_name);
                    if length > 0 {
                        let buffer_size = (length + 1) * 4;
                        let mut buffer = vec![0u8; buffer_size as usize];

                        if CFStringGetCString(
                            source_name,
                            buffer.as_mut_ptr() as *mut c_char,
                            buffer_size,
                            K_UTF8_ENCODING,
                        ) {
                            if let Ok(name_str) =
                                CStr::from_ptr(buffer.as_ptr() as *const c_char).to_str()
                            {
                                for pattern in &search_patterns {
                                    if name_str.to_uppercase().contains(&pattern.to_uppercase()) {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                if found {
                    let result = TISSelectInputSource(source);
                    if result == 0 {
                        println!("Successfully switched to layout: {}", target_layout);
                        CFRelease(input_sources);
                        thread::sleep(Duration::from_millis(100));
                        monitor::update_keyboard_layout();
                        return;
                    }
                }
            }
        }

        CFRelease(input_sources);
    }

    println!("Failed to switch to layout: {}", target_layout);
}

/// Checks configured rules and, if necessary, initiates a keyboard layout switch.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It reads from `static mut` variables (`state::CURRENT_APP`,
///    `state::APP_LAYOUT_RULES`, `state::CURRENT_KEYBOARD_LAYOUT`) via raw pointers.
/// 2. It calls `is_target_layout` and `switch_to_layout`, which are part of an unsafe API.
///
///     The caller must ensure that access to `state` variables is synchronized if
///     the application is or becomes multi-threaded.
pub unsafe fn check_and_switch_layout_by_rules() {
    if let (Some(ref app_name), Some(ref rules)) = (
        &*std::ptr::addr_of!(state::CURRENT_APP),
        &*std::ptr::addr_of!(state::APP_LAYOUT_RULES),
    ) {
        if let Some(target_layout) = rules.get(app_name) {
            if let Some(ref current_layout) = *std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT) {
                if !is_target_layout(current_layout, target_layout) {
                    println!(
                        "Application '{}' is active, switching to layout '{}'...",
                        app_name, target_layout
                    );
                    switch_to_layout(target_layout);
                }
            }
            return;
        }

        for (rule_app, target_layout) in rules {
            if app_name.contains(rule_app) || rule_app.contains(app_name) {
                if let Some(ref current_layout) =
                    *std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT)
                {
                    if !is_target_layout(current_layout, target_layout) {
                        println!(
                            "Application '{}' (rule: '{}') is active, switching to layout '{}'...",
                            app_name, rule_app, target_layout
                        );
                        switch_to_layout(target_layout);
                    }
                }
                return;
            }
        }
    }
}
