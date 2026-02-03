use crate::monitor;
use crate::state;
use common::macos_api::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFGetTypeID, CFRelease, CFStringGetCString,
    CFStringGetLength, CFStringGetTypeID, TISCreateInputSourceList, TISGetInputSourceProperty,
    TISSelectInputSource, K_TIS_PROPERTY_INPUT_SOURCE_ID, K_TIS_PROPERTY_LOCALIZED_NAME,
    K_UTF8_ENCODING,
};

use cocoa::base::nil;
use cocoa::foundation::NSString;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::thread;
use std::time::Duration;

pub fn is_target_layout(current_layout: &str, target_layout: &str) -> bool {
    let target_upper = target_layout.to_uppercase();
    let current_tokens: Vec<&str> = current_layout
        .split(|c: char| !c.is_alphanumeric())
        .collect();

    for token in current_tokens {
        if token.to_uppercase() == target_upper {
            return true;
        }
    }

    if target_layout.contains(' ') {
        return current_layout.to_uppercase().contains(&target_upper);
    }

    false
}

pub unsafe fn switch_to_layout(target_layout: &str) {
    println!("DEBUG: switch_to_layout requested for '{}'", target_layout);
    let target_upper = target_layout.to_uppercase();

    let input_sources = TISCreateInputSourceList(nil, true);
    if input_sources != nil {
        let count = CFArrayGetCount(input_sources);
        println!("DEBUG: Found {} input sources", count);

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
                                println!("DEBUG: Inspecting Source ID: '{}'", id_str);
                                let tokens: Vec<&str> =
                                    id_str.split(|c: char| !c.is_alphanumeric()).collect();
                                for token in tokens {
                                    if token.to_uppercase() == target_upper {
                                        println!(
                                            "DEBUG: MATCH! ID token '{}' == target '{}'",
                                            token, target_upper
                                        );
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
                                println!("DEBUG: Inspecting Source Name: '{}'", name_str);
                                let tokens: Vec<&str> =
                                    name_str.split(|c: char| !c.is_alphanumeric()).collect();
                                for token in tokens {
                                    if token.to_uppercase() == target_upper {
                                        println!(
                                            "DEBUG: MATCH! Name token '{}' == target '{}'",
                                            token, target_upper
                                        );
                                        found = true;
                                        break;
                                    }
                                }

                                if !found && target_layout.contains(' ') {
                                    if name_str.to_uppercase().contains(&target_upper) {
                                        println!(
                                            "DEBUG: MATCH! Name '{}' contains target '{}'",
                                            name_str, target_upper
                                        );
                                        found = true;
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

pub unsafe fn check_and_switch_layout_by_rules() {
    if let (Some(ref app_name), Some(ref rules)) = (
        &*std::ptr::addr_of!(state::CURRENT_APP),
        &*std::ptr::addr_of!(state::APP_LAYOUT_RULES),
    ) {
        if let Some(target_layout) = rules.get(app_name) {
            if let Some(ref current_layout) = *std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT) {
                if !is_target_layout(current_layout, target_layout) {
                    println!(
                        "Decision: App '{}' requires '{}', but current is '{}'. Switching...",
                        app_name, target_layout, current_layout
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
