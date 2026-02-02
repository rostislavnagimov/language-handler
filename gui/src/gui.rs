use cocoa::appkit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;


use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};

// Глобальные указатели на UI элементы (храним как usize)
static mut STATUS_BAR_VIEW: usize = 0;
static mut STATUS_LABEL: usize = 0;
static mut CONTROL_BUTTON: usize = 0;

pub fn is_daemon_running() -> bool {
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("lh-daemon")
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

pub fn start_daemon() {
    // 1. Пытаемся найти daemon рядом с текущим исполняемым файлом (для App Bundle и release)
    if let Ok(mut path) = std::env::current_exe() {
        path.pop(); // Убираем имя бинарника gui
        path.push("lh-daemon"); // Добавляем имя демона

        if path.exists() {
            let _ = Command::new(path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return;
        }
    }

    // 2. Fallback для разработки (если запуск через cargo run и пути отличаются)
    let _ = Command::new("./target/release/lh-daemon")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    // Fallback 2: может мы в корне проекта
    let _ = Command::new("./target/debug/lh-daemon")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn(); 
}

pub fn stop_daemon() {
    let _ = Command::new("pkill")
        .arg("-x")
        .arg("lh-daemon")
        .output();
}

unsafe fn get_available_languages() -> Vec<String> {
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

unsafe fn get_installed_apps() -> Vec<(String, id)> {
    let mut apps = Vec::new();

    // println!("Get apps func");

    // Читаем /Applications
    if let Ok(entries) = fs::read_dir("/Applications") {
        // println!("reading");
        for entry in entries.flatten() {
            // println!("{:#?}", entry);
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("app") {
                if let Some(app_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Получаем иконку приложения
                    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
                    let path_str = NSString::alloc(nil).init_str(&path.to_string_lossy());
                    let icon: id = msg_send![workspace, iconForFile: path_str];

                    if icon == nil {
                        println!("WARNING: nil icon for {}", app_name);
                        continue; // пропускаем это приложение
                    }

                    // Устанавливаем размер иконки 32x32
                    let size = NSSize::new(32.0, 32.0);
                    let _: () = msg_send![icon, setSize: size];
                    let _: id = msg_send![icon, retain];

                    apps.push((app_name.to_string(), icon));
                }
            }
        }
    }

    apps.sort_by(|a, b| a.0.cmp(&b.0));
    apps
}

#[repr(C)]
struct NSEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

// Класс для обновления статуса по таймеру
fn register_status_updater_class() -> *const objc::runtime::Class {
    // Проверяем, существует ли уже класс
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
    }

    decl.register()
}

extern "C" fn status_updater_button_clicked(_self: &Object, _cmd: Sel, _sender: id) {
    if is_daemon_running() {
        stop_daemon();
    } else {
        start_daemon();
    }
    // Force immediate update (optional, timer will catch it too but with delay)
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

pub unsafe fn create_and_show_window() -> id {
    let _pool = NSAutoreleasePool::new(nil);

    let window: id = msg_send![class!(NSWindow), alloc];

    let frame = NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(600.0, 400.0));

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
        initWithFrame: NSRect::new(NSPoint::new(0.0, 370.0), NSSize::new(600.0, 30.0))
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

    // Настраиваем таймер и обработчик кнопки
    let updater_class = register_status_updater_class();
    let updater: id = msg_send![updater_class, new];

    // Назначаем target/action для кнопки
    let _: () = msg_send![control_btn, setTarget: updater];
    let _: () = msg_send![control_btn, setAction: sel!(buttonClicked:)];
    
    // Создаем таймер: каждые 5.0 секунд
    let _: id = msg_send![class!(NSTimer), 
        scheduledTimerWithTimeInterval: 5.0
        target: updater
        selector: sel!(timerFired:)
        userInfo: nil
        repeats: true
    ];

    // REMOVED duplicate status bar code

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
    let scroll_frame = NSRect::new(NSPoint::new(20.0, 20.0), NSSize::new(560.0, 300.0));
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
