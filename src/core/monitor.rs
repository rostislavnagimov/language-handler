use crate::core::macos_api::{
    CFGetTypeID, CFRelease, CFStringGetCString, CFStringGetLength, CFStringGetTypeID,
    TISCopyCurrentKeyboardInputSource, TISGetInputSourceProperty, K_TIS_PROPERTY_INPUT_SOURCE_ID,
    K_TIS_PROPERTY_LOCALIZED_NAME, K_UTF8_ENCODING,
};
use crate::state;

use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Updates the global state with the name of the current active window.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It calls FFI functions (Objective-C runtime via `msg_send!`) to interact with macOS APIs.
/// 2. It writes to the `static mut` variable `state::CURRENT_APP`.
///
///     The caller must ensure that access to `state::CURRENT_APP` is synchronized if
///     the application is or becomes multi-threaded.
pub unsafe fn update_active_window() {
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let active_app: id = msg_send![workspace, frontmostApplication];

    if active_app != nil {
        let app_name: id = msg_send![active_app, localizedName];
        if app_name != nil {
            let c_string: *const c_char = msg_send![app_name, UTF8String];
            if !c_string.is_null() {
                let rust_string = CStr::from_ptr(c_string).to_string_lossy().to_string();
                state::CURRENT_APP = Some(rust_string);
            }
        }
    }
}

/// Updates the global state with information about the current keyboard layout.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It calls numerous FFI functions (TIS... and CF...) to interact with macOS APIs.
/// 2. It writes to the `static mut` variable `state::CURRENT_KEYBOARD_LAYOUT`.
///     The caller must ensure that access to `state` (specifically
///     `state::CURRENT_KEYBOARD_LAYOUT`) is synchronized if the application is or becomes multi-threaded.
pub unsafe fn update_keyboard_layout() {
    let input_source = TISCopyCurrentKeyboardInputSource();
    if input_source == nil {
        state::CURRENT_KEYBOARD_LAYOUT = Some("Unknown".to_string());
        return;
    }

    let id_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_INPUT_SOURCE_ID);
    let name_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_LOCALIZED_NAME);

    let source_id = TISGetInputSourceProperty(input_source, id_key);
    let mut layout_info = String::new();

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
                if let Ok(id_str) = CStr::from_ptr(buffer.as_ptr() as *const c_char).to_str() {
                    layout_info = id_str.to_string();
                }
            }
        }
    }

    let localized_name = TISGetInputSourceProperty(input_source, name_key);
    if localized_name != nil && CFGetTypeID(localized_name) == CFStringGetTypeID() {
        let length = CFStringGetLength(localized_name);
        if length > 0 {
            let buffer_size = (length + 1) * 4;
            let mut buffer = vec![0u8; buffer_size as usize];

            if CFStringGetCString(
                localized_name,
                buffer.as_mut_ptr() as *mut c_char,
                buffer_size,
                K_UTF8_ENCODING,
            ) {
                if let Ok(name_str) = CStr::from_ptr(buffer.as_ptr() as *const c_char).to_str() {
                    if !layout_info.is_empty() {
                        layout_info = format!("{} ({})", name_str, layout_info);
                    } else {
                        layout_info = name_str.to_string();
                    }
                }
            }
        }
    }

    CFRelease(input_source);

    if layout_info.is_empty() {
        layout_info = "Unknown".to_string();
    }

    state::CURRENT_KEYBOARD_LAYOUT = Some(layout_info);
}
