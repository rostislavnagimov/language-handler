use cocoa::appkit::{NSBackingStoreType, NSWindowStyleMask};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;

use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};

pub fn is_daemon_running() -> bool {
    let output = Command::new("pgrep").arg("-x").arg("lh-daemon").output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

pub fn start_daemon() {
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push("lh-daemon");

        if path.exists() {
            let _ = Command::new(path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return;
        }
    }

    let _ = Command::new("./target/release/lh-daemon")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let _ = Command::new("./target/debug/lh-daemon")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn stop_daemon() {
    let _ = Command::new("pkill").arg("-x").arg("lh-daemon").output();
}

pub unsafe fn get_available_languages() -> Vec<String> {
    let mut languages = HashSet::new();

    extern "C" {
        fn TISCreateInputSourceList(properties: id, include_all_installed: bool) -> id;
        fn CFArrayGetCount(array: id) -> isize;
        fn CFArrayGetValueAtIndex(array: id, idx: isize) -> id;
        fn TISGetInputSourceProperty(source: id, property_key: id) -> id;
        fn CFRelease(cf: id);
        fn CFStringCreateWithCString(alloc: id, cstr: *const c_char, encoding: u32) -> id;
        fn CFBooleanGetValue(boolean: id) -> bool;
    }

    // ИЗМЕНЕНИЕ: include_all_installed = false для получения только enabled
    let input_sources = TISCreateInputSourceList(nil, false);

    if input_sources == nil {
        return Vec::new();
    }

    let count = CFArrayGetCount(input_sources);
    println!("Found {} enabled input sources", count);

    let category_key_cstr = b"TISPropertyInputSourceCategory\0";
    let category_key =
        CFStringCreateWithCString(nil, category_key_cstr.as_ptr() as *const c_char, 0x08000100);

    let id_key_cstr = b"TISPropertyInputSourceID\0";
    let id_key = CFStringCreateWithCString(nil, id_key_cstr.as_ptr() as *const c_char, 0x08000100);

    for i in 0..count {
        let source = CFArrayGetValueAtIndex(input_sources, i);

        if source != nil {
            let category = TISGetInputSourceProperty(source, category_key);

            if category != nil {
                let cat_ptr: *const c_char = msg_send![category, UTF8String];
                if !cat_ptr.is_null() {
                    if let Ok(cat_str) = CStr::from_ptr(cat_ptr).to_str() {
                        if cat_str == "TISCategoryKeyboardInputSource" {
                            let source_id = TISGetInputSourceProperty(source, id_key);

                            if source_id != nil {
                                let id_ptr: *const c_char = msg_send![source_id, UTF8String];
                                if !id_ptr.is_null() {
                                    if let Ok(id_str) = CStr::from_ptr(id_ptr).to_str() {
                                        println!("Enabled keyboard: {}", id_str);

                                        let name = if id_str.contains("keylayout.") {
                                            id_str.split("keylayout.").last().unwrap_or(id_str)
                                        } else {
                                            id_str
                                        };

                                        languages.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    CFRelease(category_key);
    CFRelease(id_key);
    CFRelease(input_sources);

    let mut result: Vec<String> = languages.into_iter().collect();
    result.sort();
    result
}

pub unsafe fn get_installed_apps() -> Vec<(String, id)> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    let mut paths = vec![
        "/Applications".to_string(),
        "/System/Applications".to_string(),
        "/System/Applications/Utilities".to_string(),
    ];

    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{}/Applications", home));
    }

    for dir_path in paths {
        let path = Path::new(&dir_path);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                    if let Some(app_name) = path.file_stem().and_then(|s| s.to_str()) {
                        // Пропускаем если уже добавили такое имя
                        if seen_names.contains(app_name) {
                            continue;
                        }

                        // Получаем иконку приложения
                        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
                        let path_str = NSString::alloc(nil).init_str(&path.to_string_lossy());
                        let icon: id = msg_send![workspace, iconForFile: path_str];
                        let _: () = msg_send![path_str, release]; // Good practice (if owned)

                        if icon == nil {
                            // println!("WARNING: nil icon for {}", app_name);
                            continue; // пропускаем это приложение
                        }

                        // Устанавливаем размер иконки 32x32
                        let size = NSSize::new(32.0, 32.0);
                        let _: () = msg_send![icon, setSize: size];
                        // let _: id = msg_send![icon, retain]; // iconForFile usually returns autoreleased object?
                        // If we store it in Vec, we should retain it if we want to keep it alive?
                        // Original code had retain.
                        let _: id = msg_send![icon, retain];

                        apps.push((app_name.to_string(), icon));
                        seen_names.insert(app_name.to_string());
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.0.cmp(&b.0));
    apps
}
