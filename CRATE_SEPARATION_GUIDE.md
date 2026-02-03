# Пошаговый план разделения на 3 crate

Данный план описывает **только разделение кода** на 3 crate без реализации новой функциональности (IPC, CLI, autostart). После выполнения всех шагов проект должен компилироваться и работать как раньше.

---

## Текущая структура

```
language-handler/
├── Cargo.toml
├── src/
│   ├── main.rs          # Точка входа
│   ├── lib.rs           # run() + state
│   ├── gui.rs           # Cocoa окно
│   └── core/
│       ├── mod.rs
│       ├── config.rs    # JSON конфиг
│       ├── macos_api.rs # FFI биндинги
│       ├── monitor.rs   # Отслеживание окна/раскладки
│       ├── observer.rs  # macOS observers
│       └── switcher.rs  # Переключение раскладки
```

## Целевая структура

```
language-handler/
├── Cargo.toml           # Workspace root
├── common/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config.rs    # Общий конфиг
│       ├── macos_api.rs # FFI биндинги
│       └── paths.rs     # Пути к файлам
├── daemon/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs      # Точка входа daemon
│       ├── monitor.rs
│       ├── observer.rs
│       ├── switcher.rs
│       └── state.rs
└── gui/
    ├── Cargo.toml
    └── src/
        ├── main.rs      # Точка входа GUI
        └── window.rs
```

---

## Шаг 1: Создать структуру workspace

### 1.1 Создать директории

```bash
mkdir -p common/src daemon/src gui/src
```

### 1.2 Преобразовать корневой Cargo.toml в workspace

**Было** (`Cargo.toml`):
```toml
[package]
name = "language-handler"
version = "0.1.0"
edition = "2021"

[dependencies]
cocoa = "0.24"
objc = "0.2"
serde_json = "1.0.140"
```

**Стало**:
```toml
[workspace]
members = ["common", "daemon", "gui"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
cocoa = "0.24"
objc = "0.2"
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

---

## Шаг 2: Создать crate `common`

### 2.1 Создать `common/Cargo.toml`

```toml
[package]
name = "common"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
```

### 2.2 Переместить файлы

```bash
cp src/core/config.rs common/src/config.rs
cp src/core/macos_api.rs common/src/macos_api.rs
```

### 2.3 Создать `common/src/lib.rs`

```rust
pub mod config;
pub mod macos_api;
pub mod paths;
```

### 2.4 Создать `common/src/paths.rs`

```rust
use std::path::PathBuf;

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("language-handler")
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn get_launchagent_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.languagehandler.daemon.plist")
}
```

### 2.5 Обновить `common/src/config.rs`

Убрать `get_config_path()` (переехала в `paths.rs`), добавить `use crate::paths`:

```rust
use crate::paths::get_config_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub rules: HashMap<String, String>,
}

impl Config {
    pub fn load() -> Self {
        let path = get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::create_default()
    }
    
    pub fn save(&self) -> std::io::Result<()> {
        let path = get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.rules)?;
        fs::write(path, json)
    }
    
    fn create_default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("Terminal".to_string(), "US".to_string());
        rules.insert("Code".to_string(), "US".to_string());
        Self { rules }
    }
}
```

### 2.6 Обновить `common/src/macos_api.rs`

Заменить `use cocoa::base::id` на полный путь или добавить зависимость от cocoa в common.

**Вариант A**: Добавить cocoa в common/Cargo.toml:
```toml
[dependencies]
cocoa.workspace = true
```

**Вариант B**: Использовать `*mut std::ffi::c_void` вместо `id` (более чистый подход для общей библиотеки).

Рекомендую **Вариант A** для минимальных изменений.

---

## Шаг 3: Создать crate `daemon`

### 3.1 Создать `daemon/Cargo.toml`

```toml
[package]
name = "lh-daemon"
version.workspace = true
edition.workspace = true

[[bin]]
name = "lh-daemon"
path = "src/main.rs"

[dependencies]
common = { path = "../common" }
cocoa.workspace = true
objc.workspace = true

[profile.release]
opt-level = 'z'
lto = true
codegen-units = 1
panic = 'abort'
strip = true
```

### 3.2 Переместить файлы

```bash
cp src/core/monitor.rs daemon/src/monitor.rs
cp src/core/observer.rs daemon/src/observer.rs
cp src/core/switcher.rs daemon/src/switcher.rs
```

### 3.3 Создать `daemon/src/state.rs`

```rust
use std::collections::HashMap;

pub static mut CURRENT_APP: Option<String> = None;
pub static mut CURRENT_KEYBOARD_LAYOUT: Option<String> = None;
pub static mut APP_LAYOUT_RULES: Option<HashMap<String, String>> = None;
```

### 3.4 Создать `daemon/src/main.rs`

```rust
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyProhibited};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use objc::{msg_send, sel, sel_impl};

mod monitor;
mod observer;
mod state;
mod switcher;

fn main() {
    let config = common::config::Config::load();

    unsafe {
        state::APP_LAYOUT_RULES = Some(config.rules);

        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        
        // Запуск без GUI (headless)
        app.setActivationPolicy_(NSApplicationActivationPolicyProhibited);

        observer::setup_observers();
        monitor::update_active_window();
        monitor::update_keyboard_layout();

        if let (Some(app_name), Some(layout)) = (
            &*std::ptr::addr_of!(state::CURRENT_APP),
            &*std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT),
        ) {
            println!("Startup - Active window: {} | Layout: {}", app_name, layout);
        }

        println!("\nDaemon started. Press Ctrl+C to exit.");
        println!("Config file: {}", common::paths::get_config_path().display());

        let _: () = msg_send![app, run];
    }
}
```

### 3.5 Обновить импорты в перемещённых файлах

В каждом файле (`monitor.rs`, `observer.rs`, `switcher.rs`) заменить:

| Было | Стало |
|------|-------|
| `use crate::core::macos_api::*` | `use common::macos_api::*` |
| `use crate::state` | `use crate::state` (без изменений) |
| `use crate::core::monitor` | `use crate::monitor` |
| `use crate::core::switcher` | `use crate::switcher` |

---

## Шаг 4: Создать crate `gui`

### 4.1 Создать `gui/Cargo.toml`

```toml
[package]
name = "lh-gui"
version.workspace = true
edition.workspace = true

[[bin]]
name = "lh-gui"
path = "src/main.rs"

[dependencies]
common = { path = "../common" }
cocoa.workspace = true
objc.workspace = true
serde_json.workspace = true
```

### 4.2 Скопировать gui.rs

```bash
cp src/gui.rs gui/src/gui.rs
```

### 4.3 Создать `gui/src/main.rs`

GUI пока будет только показывать окно настроек (без мониторинга):

```rust
use cocoa::appkit::{NSApp, NSApplication};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use objc::{class, msg_send, sel, sel_impl};

mod gui;

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        
        app.setActivationPolicy_(
            cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );

        let _window = gui::create_and_show_window();

        let current_app: id = msg_send![class!(NSRunningApplication), currentApplication];
        let _: () = msg_send![current_app, activateWithOptions: 1 << 1];

        println!("GUI started.");
        println!("Config file: {}", common::paths::get_config_path().display());

        let _: () = msg_send![app, run];
    }
}
```

### 4.4 Обновить `gui/src/gui.rs`

Файл gui.rs содержит функцию `create_and_show_window()` и вспомогательные функции `get_available_languages()`, `get_installed_apps()`.

**Что изменить:**

1. Убрать все `use crate::...` — GUI не имеет доступа к core модулям
2. GUI не должен зависеть от state daemon

Текущие импорты в gui.rs:
```rust
use cocoa::appkit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::fs;
use std::path::Path;
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;
```

**Изменения не требуются** — gui.rs уже self-contained и не использует импорты из `crate::core`.

---

## Шаг 5: Удалить старые файлы

```bash
rm -rf src/
rm build.rs  # Если не нужен для новой структуры
```

---

## Шаг 6: Проверить компиляцию

```bash
# Собрать все crate
cargo build --workspace

# Собрать только daemon
cargo build -p lh-daemon

# Собрать только GUI
cargo build -p lh-gui

# Запустить daemon
cargo run -p lh-daemon

# Запустить GUI
cargo run -p lh-gui
```

---

## Шаг 7: Обновить .gitignore

Добавить:
```
/target/
/common/target/
/daemon/target/
/gui/target/
```

---

## Чеклист завершения

- [ ] `cargo build --workspace` без ошибок
- [ ] `cargo run -p lh-daemon` запускает headless мониторинг
- [ ] `cargo run -p lh-gui` открывает окно настроек
- [ ] Конфиг читается/пишется по пути `~/Library/Application Support/language-handler/config.json`
- [ ] Старая директория `src/` удалена

---

## Возможные проблемы и решения

| Проблема | Решение |
|----------|---------|
| `unresolved import crate::core` | Заменить на `common::` или `crate::` в зависимости от crate |
| `cannot find type id` в common | Добавить `cocoa.workspace = true` в common/Cargo.toml |
| `duplicate lang item` | Убедиться что `panic = 'abort'` только в daemon release profile |
| GUI зависит от state | Убрать `use crate::state` из window.rs, GUI пока не использует state |

---

## Известные ошибки текущей реализации (требуют исправления)

При проверке `cargo check -p lh-daemon` обнаружены следующие ошибки:

### Ошибка 1: `unresolved imports common::monitor, common::switcher`

**Файл**: `daemon/src/observer.rs` строка 1
```rust
use common::{monitor, switcher};  // НЕПРАВИЛЬНО!
```

**Причина**: `monitor` и `switcher` — это модули daemon, а не common.

**Исправление**:
```rust
use crate::{monitor, switcher};  // ПРАВИЛЬНО
```

---

### Ошибка 2: `unresolved import common::monitor`

**Файл**: `daemon/src/switcher.rs` строка 7
```rust
use common::monitor;  // НЕПРАВИЛЬНО!
```

**Исправление**:
```rust
use crate::monitor;  // ПРАВИЛЬНО
```

---

### Ошибка 3: `could not find Config in config`

**Файл**: `daemon/src/main.rs` строка 12
```rust
let config = common::config::Config::load();  // НЕПРАВИЛЬНО!
```

**Причина**: В `common/src/config.rs` нет структуры `Config` — там функция `load_or_create_config()` возвращающая `HashMap<String, String>`.

**Исправление**:
```rust
let config = common::config::load_or_create_config();  // ПРАВИЛЬНО
```

И строка 15:
```rust
state::APP_LAYOUT_RULES = Some(config.rules);  // НЕПРАВИЛЬНО!
```
заменить на:
```rust
state::APP_LAYOUT_RULES = Some(config);  // ПРАВИЛЬНО (config уже HashMap)
```

---

## Команды для исправления

```bash
# Исправить observer.rs
sed -i '' 's/use common::{monitor, switcher};/use crate::{monitor, switcher};/' daemon/src/observer.rs

# Исправить switcher.rs  
sed -i '' 's/use common::monitor;/use crate::monitor;/' daemon/src/switcher.rs

# Исправить main.rs (вручную или):
# Заменить common::config::Config::load() на common::config::load_or_create_config()
# Заменить config.rules на config
```

После исправлений запустить:
```bash
cargo check --workspace
```
