use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyProhibited};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde_json;

extern "C" {
    fn CFRunLoopRun();
    // Carbon/HIToolbox.framework
    fn TISCopyCurrentKeyboardInputSource() -> id;
    fn TISGetInputSourceProperty(input_source: id, property_key: id) -> id;
    fn TISNotifyEnabledKeyboardInputSourcesChanged();
    fn TISCopyInputSourceForLanguage(language: id) -> id;
    fn TISSelectInputSource(input_source: id) -> i32;
    fn TISCreateInputSourceList(properties: id, include_all_installed: bool) -> id;
    // CoreFoundation
    fn CFStringGetCString(the_string: id, buffer: *mut c_char, buffer_size: isize, encoding: u32) -> bool;
    fn CFStringGetLength(the_string: id) -> isize;
    fn CFGetTypeID(cf: id) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFRelease(cf: id);
    fn CFArrayGetCount(the_array: id) -> isize;
    fn CFArrayGetValueAtIndex(the_array: id, idx: isize) -> id;
    fn CFStringCreateWithCString(alloc: id, c_str: *const c_char, encoding: u32) -> id;
}

// Константы для Text Input Sources
const K_TIS_PROPERTY_INPUT_SOURCE_ID: &str = "TISPropertyInputSourceID";
const K_TIS_PROPERTY_LOCALIZED_NAME: &str = "TISPropertyLocalizedName";
const K_UTF8_ENCODING: u32 = 0x08000100;

static mut CURRENT_APP: Option<String> = None;
static mut CURRENT_KEYBOARD_LAYOUT: Option<String> = None;
static mut APP_LAYOUT_RULES: Option<HashMap<String, String>> = None;

fn get_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("keyboard-layout-monitor")
        .join("config.json")
}

fn create_default_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    config.insert("Terminal".to_string(), "US".to_string());
    config.insert("iTerm2".to_string(), "US".to_string());
    config.insert("iTerm".to_string(), "US".to_string());
    config.insert("Code".to_string(), "US".to_string());
    config.insert("Visual Studio Code".to_string(), "US".to_string());
    config.insert("Xcode".to_string(), "US".to_string());
    config
}

fn load_or_create_config() -> HashMap<String, String> {
    let config_path = get_config_path();
    
    // Создаем директорию если она не существует
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("Предупреждение: не удалось создать директорию конфига: {}", e);
                return create_default_config();
            }
        }
    }
    
    // Пытаемся прочитать существующий файл
    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match serde_json::from_str::<HashMap<String, String>>(&content) {
                    Ok(config) => {
                        println!("Загружена конфигурация из: {}", config_path.display());
                        println!("Правила переключения:");
                        for (app, layout) in &config {
                            println!("  {} -> {}", app, layout);
                        }
                        return config;
                    }
                    Err(e) => {
                        println!("Ошибка парсинга JSON конфига: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Ошибка чтения файла конфига: {}", e);
            }
        }
    }
    
    // Создаем конфиг по умолчанию
    let default_config = create_default_config();
    
    match serde_json::to_string_pretty(&default_config) {
        Ok(json_content) => {
            if let Err(e) = fs::write(&config_path, json_content) {
                println!("Предупреждение: не удалось сохранить конфиг по умолчанию: {}", e);
            } else {
                println!("Создан файл конфигурации по умолчанию: {}", config_path.display());
                println!("Правила переключения по умолчанию:");
                for (app, layout) in &default_config {
                    println!("  {} -> {}", app, layout);
                }
            }
        }
        Err(e) => {
            println!("Ошибка сериализации конфига по умолчанию: {}", e);
        }
    }
    
    default_config
}

extern "C" fn app_changed_callback(
    _self: &Object,
    _cmd: objc::runtime::Sel,
    _notification: id,
) {
    unsafe {
        update_active_window();
        update_keyboard_layout();
        
        // Проверяем правила автоматического переключения
        check_and_switch_layout_by_rules();
        
        if let (Some(app), Some(layout)) = (&*std::ptr::addr_of!(CURRENT_APP), &*std::ptr::addr_of!(CURRENT_KEYBOARD_LAYOUT)) {
            println!("Активное окно: {} | Раскладка: {}", app, layout);
        }
    }
}

extern "C" fn keyboard_changed_callback(
    _self: &Object,
    _cmd: objc::runtime::Sel,
    _notification: id,
) {
    unsafe {
        update_keyboard_layout();
        
        if let (Some(app), Some(layout)) = (&*std::ptr::addr_of!(CURRENT_APP), &*std::ptr::addr_of!(CURRENT_KEYBOARD_LAYOUT)) {
            println!("Раскладка изменена: {} | {}", app, layout);
        }
    }
}

unsafe fn update_active_window() {
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let active_app: id = msg_send![workspace, frontmostApplication];
    
    if active_app != nil {
        let app_name: id = msg_send![active_app, localizedName];
        if app_name != nil {
            let c_string: *const c_char = msg_send![app_name, UTF8String];
            if !c_string.is_null() {
                let rust_string = CStr::from_ptr(c_string).to_string_lossy().to_string();
                CURRENT_APP = Some(rust_string);
            }
        }
    }
}

unsafe fn update_keyboard_layout() {
    // Получаем текущий источник ввода клавиатуры
    let input_source = TISCopyCurrentKeyboardInputSource();
    if input_source == nil {
        CURRENT_KEYBOARD_LAYOUT = Some("Unknown".to_string());
        return;
    }

    // Создаем ключи для получения свойств
    let id_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_INPUT_SOURCE_ID);
    let name_key = NSString::alloc(nil).init_str(K_TIS_PROPERTY_LOCALIZED_NAME);

    // Получаем ID источника ввода
    let source_id = TISGetInputSourceProperty(input_source, id_key);
    let mut layout_info = String::new();

    if source_id != nil && CFGetTypeID(source_id) == CFStringGetTypeID() {
        let length = CFStringGetLength(source_id);
        if length > 0 {
            let buffer_size = (length + 1) * 4; // UTF-8 может занимать до 4 байт на символ
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

    // Также получаем локализованное имя для более читаемого вывода
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

    // Освобождаем ресурсы
    CFRelease(input_source);

    if layout_info.is_empty() {
        layout_info = "Unknown".to_string();
    }

    CURRENT_KEYBOARD_LAYOUT = Some(layout_info);
}

unsafe fn check_and_switch_layout_by_rules() {
    if let (Some(ref app_name), Some(ref rules)) = (
        &*std::ptr::addr_of!(CURRENT_APP),
        &*std::ptr::addr_of!(APP_LAYOUT_RULES)
    ) {
        // Проверяем точное совпадение имени приложения
        if let Some(target_layout) = rules.get(app_name) {
            if let Some(ref current_layout) = *std::ptr::addr_of!(CURRENT_KEYBOARD_LAYOUT) {
                if !is_target_layout(current_layout, target_layout) {
                    println!("Приложение '{}' активно, переключаем на раскладку '{}'...", app_name, target_layout);
                    switch_to_layout(target_layout);
                }
            }
            return;
        }
        
        // Проверяем частичные совпадения (например, для "Terminal" будет работать с "iTerm2")
        for (rule_app, target_layout) in rules {
            if app_name.contains(rule_app) || rule_app.contains(app_name) {
                if let Some(ref current_layout) = *std::ptr::addr_of!(CURRENT_KEYBOARD_LAYOUT) {
                    if !is_target_layout(current_layout, target_layout) {
                        println!("Приложение '{}' (правило: '{}') активно, переключаем на раскладку '{}'...", 
                               app_name, rule_app, target_layout);
                        switch_to_layout(target_layout);
                    }
                }
                return;
            }
        }
    }
}

unsafe fn is_target_layout(current_layout: &str, target_layout: &str) -> bool {
    match target_layout.to_uppercase().as_str() {
        "US" | "EN" | "ENGLISH" => {
            current_layout.contains("U.S.") ||
            current_layout.contains("US") ||
            current_layout.contains("English") ||
            current_layout.contains("com.apple.keylayout.US") ||
            current_layout.contains("ABC")
        }
        "RU" | "RUSSIAN" => {
            current_layout.contains("Russian") ||
            current_layout.contains("Русская") ||
            current_layout.contains("com.apple.keylayout.Russian")
        }
        _ => current_layout.to_uppercase().contains(&target_layout.to_uppercase())
    }
}

unsafe fn switch_to_layout(target_layout: &str) {
    let target_upper = target_layout.to_uppercase();
    
    // Определяем какую раскладку ищем
    let search_patterns = match target_upper.as_str() {
        "US" | "EN" | "ENGLISH" => vec!["com.apple.keylayout.US", "com.apple.keylayout.ABC", "US"],
        "RU" | "RUSSIAN" => vec!["com.apple.keylayout.Russian", "Russian"],
        _ => vec![target_layout]
    };
    
    // Сначала попробуем через язык для стандартных раскладок
    if target_upper == "US" || target_upper == "EN" || target_upper == "ENGLISH" {
        let us_lang = CFStringCreateWithCString(nil, "en\0".as_ptr() as *const c_char, K_UTF8_ENCODING);
        if us_lang != nil {
            let us_source = TISCopyInputSourceForLanguage(us_lang);
            CFRelease(us_lang);
            
            if us_source != nil {
                let result = TISSelectInputSource(us_source);
                CFRelease(us_source);
                
                if result == 0 {
                    println!("Успешно переключено на раскладку: {}", target_layout);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    update_keyboard_layout();
                    return;
                }
            }
        }
    }
    
    // Если не получилось через язык, ищем в списке всех источников
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
                
                // Проверяем ID источника
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
                
                // Проверяем имя источника
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
                            if let Ok(name_str) = CStr::from_ptr(buffer.as_ptr() as *const c_char).to_str() {
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
                        println!("Успешно переключено на раскладку: {}", target_layout);
                        CFRelease(input_sources);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        update_keyboard_layout();
                        return;
                    }
                }
            }
        }
        
        CFRelease(input_sources);
    }
    
    println!("Не удалось переключить на раскладку: {}", target_layout);
}

// Создаем класс для обработки уведомлений
fn create_observer_class() -> *const objc::runtime::Class {
    use objc::declare::ClassDecl;
    use objc::runtime::Sel;
    
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("WindowObserver", superclass).unwrap();
    
    unsafe {
        decl.add_method(
            sel!(appChanged:),
            app_changed_callback as extern "C" fn(&Object, Sel, id),
        );
        
        decl.add_method(
            sel!(keyboardChanged:),
            keyboard_changed_callback as extern "C" fn(&Object, Sel, id),
        );
    }
    
    decl.register()
}

fn main() {
    // Загружаем конфигурацию
    let config = load_or_create_config();
    unsafe {
        APP_LAYOUT_RULES = Some(config);
    }
    
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyProhibited);

        // Создаем наблюдателя
        let observer_class = create_observer_class();
        let observer: id = msg_send![observer_class, new];

        // NSWorkspace уведомления для смены активного приложения
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let notification_center: id = msg_send![workspace, notificationCenter];
        
        let app_notification_name = NSString::alloc(nil).init_str("NSWorkspaceDidActivateApplicationNotification");
        let _: () = msg_send![notification_center, 
            addObserver: observer
            selector: sel!(appChanged:)
            name: app_notification_name
            object: nil
        ];

        // Уведомления о смене источника ввода клавиатуры
        let default_center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
        
        // Это основное уведомление о смене раскладки клавиатуры
        let keyboard_notification_name = NSString::alloc(nil).init_str("NSTextInputContextKeyboardSelectionDidChangeNotification");
        let _: () = msg_send![default_center,
            addObserver: observer
            selector: sel!(keyboardChanged:)
            name: keyboard_notification_name
            object: nil
        ];

        // Дополнительное уведомление для более надежного отслеживания
        let input_source_notification = NSString::alloc(nil).init_str("kTISNotifySelectedKeyboardInputSourceChanged");
        let _: () = msg_send![default_center,
            addObserver: observer
            selector: sel!(keyboardChanged:)
            name: input_source_notification
            object: nil
        ];

        // Получить начальные значения
        update_active_window();
        update_keyboard_layout();
        
        if let (Some(app), Some(layout)) = (&*std::ptr::addr_of!(CURRENT_APP), &*std::ptr::addr_of!(CURRENT_KEYBOARD_LAYOUT)) {
            println!("Запуск - Активное окно: {} | Раскладка: {}", app, layout);
        }

        println!("\nМониторинг запущен. Нажмите Ctrl+C для выхода.");
        println!("Файл конфигурации: {}", get_config_path().display());
        println!("Автоматическое переключение раскладок активно");
        
        CFRunLoopRun();
    }
}