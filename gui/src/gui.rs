use cocoa::appkit::{NSBackingStoreType, NSWindowStyleMask};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::process::Command;

use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};

use crate::helpers::*;

static mut STATUS_BAR_VIEW: usize = 0;
static mut STATUS_LABEL: usize = 0;
static mut CONTROL_BUTTON: usize = 0;

pub fn register_status_updater_class() -> *const objc::runtime::Class {
    if let Some(cls) = objc::runtime::Class::get("StatusUpdater") {
        return cls;
    }

    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("StatusUpdater", superclass).unwrap();

    unsafe {
        decl.add_method(
            sel!(timerFired:),
            status_updater_timer_fired as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(buttonClicked:),
            status_updater_button_clicked as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(autostartClicked:),
            status_updater_autostart_clicked as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(popupAction:),
            status_updater_popup_action as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(applicationShouldTerminateAfterLastWindowClosed:),
            application_should_terminate_after_last_window_closed as extern "C" fn(&Object, Sel, id) -> bool,
        );
    }

    decl.register()
}

extern "C" fn status_updater_autostart_clicked(_self: &Object, _cmd: Sel, _sender: id) {
    let current = is_autostart_enabled();
    if current {
        if let Err(e) = disable_autostart() {
            println!("Error disabling autostart: {}", e);
        } else {
            println!("Autostart disabled.");
        }
    } else {
        if let Err(e) = enable_autostart() {
            println!("Error enabling autostart: {}", e);
        } else {
            println!("Autostart enabled.");
        }
    }

    unsafe {
        let enabled = is_autostart_enabled();
        if AUTOSTART_BAR_VIEW != 0 && AUTOSTART_LABEL != 0 && AUTOSTART_BUTTON != 0 {
            let bar = AUTOSTART_BAR_VIEW as id;
            let label = AUTOSTART_LABEL as id;
            let btn = AUTOSTART_BUTTON as id;

            let autostart_color: id = if enabled {
                msg_send![class!(NSColor), colorWithRed:0.2 green:0.6 blue:0.8 alpha:0.5]
            } else {
                msg_send![class!(NSColor), colorWithRed:0.5 green:0.5 blue:0.5 alpha:0.5]
            };
            let layer: id = msg_send![bar, layer];
            let cg: id = msg_send![autostart_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg];

            let text = if enabled {
                "🚀 Autostart: Enabled"
            } else {
                "🚀 Autostart: Disabled"
            };
            let ns_text = NSString::alloc(nil).init_str(text);
            let _: () = msg_send![label, setStringValue: ns_text];

            let btn_title = if enabled { "Disable" } else { "Enable" };
            let ns_btn = NSString::alloc(nil).init_str(btn_title);
            let _: () = msg_send![btn, setTitle: ns_btn];
        }
    }
}

extern "C" fn status_updater_popup_action(_self: &Object, _cmd: Sel, sender: id) {
    unsafe {
        let app_name_ns: id = msg_send![sender, identifier];
        if app_name_ns == nil {
            println!("Error: Popup has no app name attached (identifier is nil)");
            return;
        }

        let len: usize = msg_send![app_name_ns, length];
        let utf8_ptr: *const c_char = msg_send![app_name_ns, UTF8String];
        let app_name = if !utf8_ptr.is_null() {
            std::slice::from_raw_parts(utf8_ptr as *const u8, len)
        } else {
            println!("Error: unique app name UTF8");
            return;
        };
        let app_name = String::from_utf8_lossy(app_name).to_string();

        let selected_item: id = msg_send![sender, selectedItem];
        let title_ns: id = msg_send![selected_item, title];
        let len: usize = msg_send![title_ns, length];
        let utf8_ptr: *const c_char = msg_send![title_ns, UTF8String];
        let selected_layout = if !utf8_ptr.is_null() {
            String::from_utf8_lossy(std::slice::from_raw_parts(utf8_ptr as *const u8, len))
                .to_string()
        } else {
            "None".to_string()
        };

        println!(
            "Config change: App='{}' -> Layout='{}'",
            app_name, selected_layout
        );

        let mut config = common::config::load_or_create_config();

        if selected_layout == "None" {
            config.remove(&app_name);
        } else {
            config.insert(app_name, selected_layout);
        }

        if let Err(e) = common::config::save_config(&config) {
            println!("Error saving config: {}", e);
        } else {
            println!("Config saved successfully.");
            println!("Updated rules for verification:");
            for (app, layout) in &config {
                println!("  {} -> {}", app, layout);
            }

            if is_daemon_running() {
                println!("Restarting daemon to apply changes...");
                stop_daemon();
                thread::sleep(Duration::from_millis(200));
                start_daemon();
            }
        }
    }
}

extern "C" fn status_updater_button_clicked(_self: &Object, _cmd: Sel, _sender: id) {
    if is_daemon_running() {
        stop_daemon();
    } else {
        start_daemon();
    }
    status_updater_timer_fired(_self, _cmd, nil);
}

extern "C" fn status_updater_timer_fired(_self: &Object, _cmd: Sel, _timer: id) {
    unsafe {
        let is_running = is_daemon_running();

        let status_text = if is_running {
            "🟢 Background monitoring: Active"
        } else {
            "🔴 Background monitoring: Inactive"
        };

        let status_color: id = if is_running {
            msg_send![class!(NSColor), colorWithRed:0.2 green:0.8 blue:0.2 alpha:0.5]
        } else {
            msg_send![class!(NSColor), colorWithRed:0.8 green:0.2 blue:0.2 alpha:0.5]
        };

        if STATUS_LABEL != 0 {
            let label = STATUS_LABEL as id;
            let ns_str = NSString::alloc(nil).init_str(status_text);
            let _: () = msg_send![label, setStringValue: ns_str];
        }

        if STATUS_BAR_VIEW != 0 {
            let view = STATUS_BAR_VIEW as id;
            let layer: id = msg_send![view, layer];
            if layer != nil {
                let bg_color: id = msg_send![status_color, CGColor];
                let _: () = msg_send![layer, setBackgroundColor: bg_color];
            }
        }

        if CONTROL_BUTTON != 0 {
            let btn = CONTROL_BUTTON as id;
            let btn_title = if is_running { "Stop" } else { "Start" };
            let ns_title = NSString::alloc(nil).init_str(btn_title);
            let _: () = msg_send![btn, setTitle: ns_title];
        }
    }
}

extern "C" fn application_should_terminate_after_last_window_closed(
    _self: &Object,
    _cmd: Sel,
    _sender: id,
) -> bool {
    true
}

fn get_launch_agent_path() -> PathBuf {
    common::paths::get_launchagent_path()
}

fn get_daemon_path() -> PathBuf {
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push("lh-daemon");
        if path.exists() {
            return path;
        }
    }
    // Fallbacks
    let release = Path::new("./target/release/lh-daemon");
    if release.exists() {
        return release.canonicalize().unwrap_or(release.to_path_buf());
    }
    PathBuf::from("lh-daemon")
}

fn is_autostart_enabled() -> bool {
    get_launch_agent_path().exists()
}

fn enable_autostart() -> Result<(), String> {
    let daemon_path = get_daemon_path();
    if !daemon_path.exists() {
        return Err(format!("Daemon binary not found at {:?}", daemon_path));
    }
    let abs_path = daemon_path.canonicalize().unwrap_or(daemon_path);

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.languagehandler.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>/tmp/lh-daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/lh-daemon.err</string>
</dict>
</plist>"#,
        abs_path.display()
    );

    let path = get_launch_agent_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, plist_content).map_err(|e| e.to_string())?;

    Ok(())
}

fn disable_autostart() -> Result<(), String> {
    let path = get_launch_agent_path();
    if path.exists() {
        // To properly remove from "Login Items & Extensions", we should bootout it
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/com.languagehandler.daemon", uid);
        let _ = Command::new("launchctl")
            .args(["bootout", &service_target])
            .output();

        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Global pointers for Autostart UI
static mut AUTOSTART_BAR_VIEW: usize = 0;
static mut AUTOSTART_LABEL: usize = 0;
static mut AUTOSTART_BUTTON: usize = 0;

pub unsafe fn create_and_show_window() -> id {
    println!("DEBUG: create_and_show_window start");
    let _pool = NSAutoreleasePool::new(nil);

    let window: id = msg_send![class!(NSWindow), alloc];

    let frame = NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(600.0, 440.0));

    let style = NSWindowStyleMask::NSTitledWindowMask | NSWindowStyleMask::NSClosableWindowMask;

    let window: id = msg_send![window,
        initWithContentRect:frame
        styleMask:style
        backing:NSBackingStoreType::NSBackingStoreBuffered as u64
        defer:false
    ];

    let title = NSString::alloc(nil).init_str("Language Handler");
    let _: () = msg_send![window, setTitle: title];
    let _: () = msg_send![window, center];
    let _: () = msg_send![title, release];

    let is_running = is_daemon_running();
    println!(">>>>>>>>>> {} <<<<<<<<<<<<", is_running);

    // Статус бар внутри окна
    let content_view: id = msg_send![window, contentView];

    // Статус бар (зеленая полоска сверху)
    let status_bar: id = msg_send![class!(NSView), alloc];
    let status_bar: id = msg_send![status_bar,
        initWithFrame: NSRect::new(NSPoint::new(0.0, 410.0), NSSize::new(600.0, 30.0))
    ];

    let status_color: id = if is_running {
        msg_send![class!(NSColor), colorWithRed:0.2 green:0.8 blue:0.2 alpha:0.5]
    } else {
        msg_send![class!(NSColor), colorWithRed:0.8 green:0.2 blue:0.2 alpha:0.5]
    };
    let _: () = msg_send![status_bar, setWantsLayer: true];
    let layer: id = msg_send![status_bar, layer];
    let bg_color: id = msg_send![status_color, CGColor];
    let _: () = msg_send![layer, setBackgroundColor: bg_color]; // FIX: Added missing call

    // Текст "Status: Running"
    let status_label: id = msg_send![class!(NSTextField), alloc];
    let status_label: id = msg_send![status_label,
        initWithFrame: NSRect::new(NSPoint::new(10.0, 5.0), NSSize::new(580.0, 20.0))
    ];
    let status_text = if is_running {
        "🟢 Background monitoring: Active"
    } else {
        "🔴 Background monitoring: Inactive"
    };
    let _: () = msg_send![status_label, setStringValue: NSString::alloc(nil).init_str(status_text)];
    let _: () = msg_send![status_label, setBezeled: false];
    let _: () = msg_send![status_label, setDrawsBackground: false];
    let _: () = msg_send![status_label, setEditable: false];
    let white_color: id = msg_send![class!(NSColor), whiteColor];
    let _: () = msg_send![status_label, setTextColor: white_color];

    let _: () = msg_send![status_bar, addSubview: status_label];

    // Кнопка управления справа
    let control_btn: id = msg_send![class!(NSButton), alloc];
    let control_btn: id = msg_send![control_btn, initWithFrame:
        NSRect::new(NSPoint::new(500.0, 3.0), NSSize::new(90.0, 24.0))
    ];
    let _: () = msg_send![control_btn, setBezelStyle: 1]; // NSRoundedBezelStyle
    let btn_title = if is_running { "Stop" } else { "Start" };
    let ns_title = NSString::alloc(nil).init_str(btn_title);
    let _: () = msg_send![control_btn, setTitle: ns_title];

    let _: () = msg_send![status_bar, addSubview: control_btn];
    let _: () = msg_send![content_view, addSubview: status_bar];

    // Сохраняем указатели глобально
    STATUS_BAR_VIEW = status_bar as usize;
    STATUS_LABEL = status_label as usize;
    CONTROL_BUTTON = control_btn as usize;

    // --- Status Bar 2 (Autostart) Below Bar 1 ---
    let autostart_enabled = is_autostart_enabled();

    let autostart_bar: id = msg_send![class!(NSView), alloc];
    // Position: 410 - 30 - 5 = 375
    let autostart_bar: id = msg_send![autostart_bar,
        initWithFrame: NSRect::new(NSPoint::new(0.0, 375.0), NSSize::new(600.0, 30.0))
    ];

    let autostart_color: id = if autostart_enabled {
        msg_send![class!(NSColor), colorWithRed:0.2 green:0.6 blue:0.8 alpha:0.5]
    // Blueish for autostart
    } else {
        msg_send![class!(NSColor), colorWithRed:0.5 green:0.5 blue:0.5 alpha:0.5]
        // Gray if inactive
    };
    let _: () = msg_send![autostart_bar, setWantsLayer: true];
    let layer2: id = msg_send![autostart_bar, layer];
    let bg_color2: id = msg_send![autostart_color, CGColor];
    let _: () = msg_send![layer2, setBackgroundColor: bg_color2];

    let autostart_label: id = msg_send![class!(NSTextField), alloc];
    let autostart_label: id = msg_send![autostart_label,
        initWithFrame: NSRect::new(NSPoint::new(10.0, 5.0), NSSize::new(480.0, 20.0))
    ];
    let white_color: id = msg_send![class!(NSColor), whiteColor]; // Reuse existing white_color logic? It was local.
                                                                  // Need to re-acquire white_color or ensure it's available.
                                                                  // white_color was created at line 553. It is local scope but available in function block.
                                                                  // Wait, msg_send doesn't scope bind. Result `white_color` is `id`.

    let autostart_text = if autostart_enabled {
        "🚀 Autostart: Enabled"
    } else {
        "🚀 Autostart: Disabled"
    };
    let _: () =
        msg_send![autostart_label, setStringValue: NSString::alloc(nil).init_str(autostart_text)];
    let _: () = msg_send![autostart_label, setBezeled: false];
    let _: () = msg_send![autostart_label, setDrawsBackground: false];
    let _: () = msg_send![autostart_label, setEditable: false];
    let _: () = msg_send![autostart_label, setTextColor: white_color];

    let _: () = msg_send![autostart_bar, addSubview: autostart_label];

    let max_btn_x = 500.0;
    let autostart_btn: id = msg_send![class!(NSButton), alloc];
    let autostart_btn: id = msg_send![autostart_btn, initWithFrame:
        NSRect::new(NSPoint::new(max_btn_x, 3.0), NSSize::new(90.0, 24.0))
    ];
    let _: () = msg_send![autostart_btn, setBezelStyle: 1];
    let as_btn_title = if autostart_enabled {
        "Disable"
    } else {
        "Enable"
    };
    let as_ns_title = NSString::alloc(nil).init_str(as_btn_title);
    let _: () = msg_send![autostart_btn, setTitle: as_ns_title];

    let _: () = msg_send![autostart_bar, addSubview: autostart_btn];
    let _: () = msg_send![content_view, addSubview: autostart_bar];

    AUTOSTART_BAR_VIEW = autostart_bar as usize;
    AUTOSTART_LABEL = autostart_label as usize;
    AUTOSTART_BUTTON = autostart_btn as usize;

    // Настраиваем таймер и обработчик кнопки
    let updater_class = register_status_updater_class();
    let updater: id = msg_send![updater_class, new];

    // Назначаем target/action для кнопки
    let _: () = msg_send![control_btn, setTarget: updater];
    let _: () = msg_send![control_btn, setAction: sel!(buttonClicked:)];

    // Target/Action for Button 2 (Autostart)
    let _: () = msg_send![autostart_btn, setTarget: updater];
    let _: () = msg_send![autostart_btn, setAction: sel!(autostartClicked:)];

    // Создаем таймер: каждые 5.0 секунд
    let _: id = msg_send![class!(NSTimer),
        scheduledTimerWithTimeInterval: 5.0
        target: updater
        selector: sel!(timerFired:)
        userInfo: nil
        repeats: true
    ];

    // Загружаем конфиг
    let config = common::config::load_or_create_config();

    // Таблица правил
    // Получаем список приложений
    let apps = get_installed_apps();
    let languages = get_available_languages();
    println!("Available languages: {:?}", languages);

    let none_str = NSString::alloc(nil).init_str("None");

    let mut language_strings = Vec::new();
    for lang in &languages {
        let lang_str = NSString::alloc(nil).init_str(lang);
        language_strings.push(lang_str);
    }

    // println!("Apps count: {}", apps.len());
    // for (name, _) in &apps {
    //     println!("App: {}", name);
    // }

    // Создаем scroll view для таблицы
    let scroll_view: id = msg_send![class!(NSScrollView), alloc];
    let scroll_frame = NSRect::new(NSPoint::new(20.0, 20.0), NSSize::new(560.0, 335.0));
    let scroll_view: id = msg_send![scroll_view, initWithFrame: scroll_frame];
    let _: () = msg_send![scroll_view, setHasVerticalScroller: true];
    let _: () = msg_send![scroll_view, setHasHorizontalScroller: false];
    let _: () = msg_send![scroll_view, setBorderType: 1];
    let _: () = msg_send![scroll_view, setAutohidesScrollers: true];

    let _: () = msg_send![scroll_view, setWantsLayer: true];
    let scroll_layer: id = msg_send![scroll_view, layer];
    let _: () = msg_send![scroll_layer, setCornerRadius: 8.0];
    let _: () = msg_send![scroll_layer, setMasksToBounds: true];

    let _: () = msg_send![scroll_view, setDrawsBackground: true];
    let bg_color: id = msg_send![class!(NSColor), controlBackgroundColor]; // Системный фон
    let _: () = msg_send![scroll_view, setBackgroundColor: bg_color];

    // Создаём обычный контейнер
    let container: id = msg_send![class!(NSView), alloc];
    let container_height = apps.len() as f64 * 45.0;
    let container: id = msg_send![container, initWithFrame:
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(560.0, container_height))];

    let mut y_position = container_height - 45.0;

    let mut row_index = 0;

    for (app_name, icon) in &apps {
        let icon = *icon;
        // println!("Processing {}: icon={:p}", app_name, icon);

        if row_index > 0 {
            let top_border: id = msg_send![class!(NSView), alloc];
            let top_border: id = msg_send![top_border, initWithFrame:
                NSRect::new(NSPoint::new(0.0, y_position + 44.0), NSSize::new(560.0, 1.0))
            ];
            let _: () = msg_send![top_border, setWantsLayer: true];
            let border_layer: id = msg_send![top_border, layer];
            let separator_color: id = msg_send![class!(NSColor), colorWithWhite:0.4 alpha:1.0];
            let sep_cg: id = msg_send![separator_color, CGColor];
            let _: () = msg_send![border_layer, setBackgroundColor: sep_cg];
            let _: () = msg_send![container, addSubview: top_border];
        }

        if row_index % 2 == 0 {
            let row_bg: id = msg_send![class!(NSView), alloc];
            let row_bg: id = msg_send![row_bg, initWithFrame:
                NSRect::new(NSPoint::new(0.0, y_position), NSSize::new(560.0, 45.0))
            ];

            let _: () = msg_send![container, addSubview: row_bg];
            let _: () = msg_send![row_bg, setWantsLayer: true];
            let row_layer: id = msg_send![row_bg, layer];
            let light_gray: id = msg_send![class!(NSColor), colorWithWhite:0.2 alpha:1.0];
            let color_cg: id = msg_send![light_gray, CGColor];
            let _: () = msg_send![row_layer, setBackgroundColor: color_cg];
        }

        // Создаём ImageView
        let image_view: id = msg_send![class!(NSImageView), alloc];
        let image_view: id = msg_send![image_view, initWithFrame:
        NSRect::new(NSPoint::new(10.0, y_position + 5.0), NSSize::new(32.0, 32.0))];
        let _: () = msg_send![image_view, setImage: icon];
        let _: () = msg_send![image_view, setImageScaling: 0];
        let _: () = msg_send![container, addSubview: image_view];

        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, initWithFrame:
        NSRect::new(NSPoint::new(50.0, y_position + 12.0), NSSize::new(280.0, 20.0))];
        let _: () = msg_send![label, setStringValue: NSString::alloc(nil).init_str(app_name)];
        let _: () = msg_send![label, setBezeled: false];
        let _: () = msg_send![label, setEditable: false];
        let _: () = msg_send![label, setDrawsBackground: false];
        let _: () = msg_send![container, addSubview: label];

        let popup: id = msg_send![class!(NSPopUpButton), alloc];
        let popup: id = msg_send![popup, initWithFrame:
        NSRect::new(NSPoint::new(340.0, y_position + 8.0), NSSize::new(180.0, 28.0))
        pullsDown: false];

        let _: () = msg_send![popup, addItemWithTitle: none_str];
        for lang_str in &language_strings {
            let _: () = msg_send![popup, addItemWithTitle: *lang_str];
        }

        // Выбираем значение из конфига, если есть
        if let Some(saved_layout) = config.get(app_name) {
            let saved_layout_ns = NSString::alloc(nil).init_str(saved_layout);
            let _: () = msg_send![popup, selectItemWithTitle: saved_layout_ns];
            let _: () = msg_send![saved_layout_ns, release];
        }

        // Выбираем значение из конфига, если есть
        if let Some(saved_layout) = config.get(app_name) {
            let saved_layout_ns = NSString::alloc(nil).init_str(saved_layout);
            let _: () = msg_send![popup, selectItemWithTitle: saved_layout_ns];
            let _: () = msg_send![saved_layout_ns, release];
        }

        // Привязываем action и данные
        let app_name_ns = NSString::alloc(nil).init_str(app_name); // Create new NSString to store
                                                                   // Важно: representedObject не retain'ит автоматически в некоторых реализациях?
                                                                   // Обычно retain property.
                                                                   // Проверим docs Cocoa. representedObject is (retain).
                                                                   // Так что можно pass `app_name_ns` created via init_str.
                                                                   // Но `app_name` (rust) is temporary ref from loop.
                                                                   // `NSString::init_str` copies bytes to new obj.
                                                                   // BUT we need `autorelease` or just pass it if `representedObject` retains it.
                                                                   // `representedObject` setter retains.
                                                                   // So we create, set, delete? or autorelease?
                                                                   // `app_name_ns` created with alloc/init gives ownership (retain count +1).
                                                                   // Set representedObject -> it retains (+1).
                                                                   // If we don't release, leak.
                                                                   // We should release after setting.

        // Используем identifier (NSView property) вместо representedObject (Cell property)
        let _: () = msg_send![popup, setIdentifier: app_name_ns];
        let _: () = msg_send![app_name_ns, release]; // Balance alloc

        let _: () = msg_send![popup, setTarget: updater];
        let _: () = msg_send![popup, setAction: sel!(popupAction:)];

        let _: () = msg_send![container, addSubview: popup];

        row_index += 1;
        y_position -= 45.0;
    }

    let _: () = msg_send![none_str, release];
    for lang_str in language_strings {
        let _: () = msg_send![lang_str, release];
    }

    let _: () = msg_send![scroll_view, setDocumentView: container];
    let _: () = msg_send![container, scrollPoint: NSPoint::new(0.0, container_height)];

    let _: () = msg_send![content_view, addSubview: scroll_view];
    let _: () = msg_send![window, makeKeyAndOrderFront: nil];

    window
}
