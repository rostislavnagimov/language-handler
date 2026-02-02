# План реализации управления Daemon через GUI

Этот документ описывает шаги для реализации отображения статуса daemon и кнопок управления (Start/Stop) в GUI, а также отключения автозапуска.

---

## 1. Создать модуль управления (`gui/src/daemon_control.rs`)

Этот модуль будет отвечать за взаимодействие с процессом `lh-daemon`.

**Действие**: Создать файл `gui/src/daemon_control.rs` со следующим содержанием:

```rust
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Проверяет, запущен ли процесс lh-daemon с помощью pgrep
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

/// Запускает lh-daemon
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
}

/// Останавливает lh-daemon через pkill
pub fn stop_daemon() {
    let _ = Command::new("pkill")
        .arg("-x")
        .arg("lh-daemon")
        .output();
}
```

---

## 2. Подключить модуль в `gui/src/main.rs`

**Действие**: В файле `gui/src/main.rs` добавить объявление модуля:

```rust
mod gui;
mod daemon_control; // <-- Добавить эту строку
```

**Действие**: Убедиться, что в `main` нет логики запуска мониторинга.
*Текущее состояние*: Логика наблюдений (`observer::setup_observers`) уже удалена при разделении, так что автозапуск мониторинга при старте GUI **уже отсутствует**.

---

## 3. Обновить интерфейс в `gui/src/gui.rs`

Необходимо модифицировать функцию `create_and_show_window`, чтобы добавить кнопки и таймер обновления статуса.

### 3.1 Добавить импорты

```rust
use crate::daemon_control;
use std::thread;
use std::time::Duration;
use cocoa::appkit::{NSButton, NSBezelStyle}; // Для кнопок
```

### 3.2 Изменить создание первого статус-бара (верхний)

В секции создания первого статус-бара (где "Background monitoring"):

1. **Сохранить ссылку на Label статуса**, чтобы его можно было обновлять.
2. **Добавить кнопку Start/Stop**.

**Примерный код изменений в `create_and_show_window`**:

```rust
    // ... создание status_bar ...

    // Метка статуса (сделаем её переменной, чтобы обновлять, 
    // но в Cocoa update из другого потока требует трюков,
    // проще перерисовывать или использовать channel, 
    // но для простого примера сделаем polling в UI thread через таймер если бы могли,
    // или запустим поток, который будет просто знать статус)

    // Кнопка управления
    let control_btn: id = msg_send![class!(NSButton), alloc];
    let control_btn: id = msg_send![control_btn, initWithFrame: 
        NSRect::new(NSPoint::new(480.0, 3.0), NSSize::new(100.0, 24.0))
    ];
    let _: () = msg_send![control_btn, setBezelStyle: NSBezelStyle::NSRoundedBezelStyle];
    
    // Начальное состояние кнопки
    let is_running = daemon_control::is_daemon_running();
    let btn_title = if is_running { "Stop" } else { "Start" };
    let ns_title = NSString::alloc(nil).init_str(btn_title);
    let _: () = msg_send![control_btn, setTitle: ns_title];
    
    // Action для кнопки
    let _: () = msg_send![control_btn, setTarget: window]; // Или другой target
    // Тут сложный момент с Rust/Objective-C callbacks. 
    // Для обработки нажатия нужно создавать делегат или класс-обертку, как делали для observer.
    
    let _: () = msg_send![status_bar, addSubview: control_btn];
```

### 3.3 Реализация обработки нажатия и обновления статуса

Так как обработка событий в Rust с Cocoa требует создания Objective-C классов (как мы делали в `observer.rs`), **предлагается упрощенный вариант**:

Создать вспомогательный класс `GuiController` в `gui/src/gui.rs` (по аналогии с observer), который будет обрабатывать нажатие кнопки и таймер.

**Шаги для `gui/src/gui.rs`**:

1. **Создать класс `GuiController`**:
   ```rust
   // В начале файла
   use objc::declare::ClassDecl;
   use objc::runtime::{Object, Sel};

   pub extern "C" fn on_button_click(_self: &Object, _cmd: Sel, _sender: id) {
       if daemon_control::is_daemon_running() {
           daemon_control::stop_daemon();
       } else {
           daemon_control::start_daemon();
       }
       // Обновляем UI сразу (можно добавить задержку)
       thread::sleep(Duration::from_millis(200));
       update_ui_state(_self); 
   }

   pub extern "C" fn timer_callback(_self: &Object, _cmd: Sel, _timer: id) {
       update_ui_state(_self);
   }
   
   // Функция обновления UI (находит кнопку и label по тегам или instance vars)
   fn update_ui_state(controller: &Object) {
       // Получить доступ к окну/кнопкам и обновить текст
       // Это требует хранения ссылок на UI элементы
   }
   ```

2. **Инициализировать Controller в `create_and_show_window`**:
   - Создать инстанс `GuiController`.
   - Назначить его `target` для кнопки.
   - Запустить `NSTimer`, который вызывает метод контроллера каждую секунду для проверки `is_daemon_running` и обновления иконки/текста статуса.

---

## Итоговый чеклист для вас

1.  Создать `gui/src/daemon_control.rs` с логикой `pgrep` и `Command::spawn`.
2.  В `gui/src/main.rs` добавить `mod daemon_control;`.
3.  В `gui/src/gui.rs`:
    *   Реализовать Objective-C класс для обработки событий (клик, таймер).
    *   Добавить кнопку Start/Stop в верхний бар.
    *   Настроить таймер (NSTimer) для периодического (например, раз в 1 сек) вызова проверки статуса.
    *   В зависимости от результата `is_daemon_running()` менять:
        *   Текст статуса (Running/Stopped).
        *   Цвет фона статуса (Зеленый/Красный).
        *   Текст кнопки (Stop/Start).

Автозапуск при старте GUI сейчас **отключен** архитектурно.

---

## 4. Сборка Bundle (Важно!)

Чтобы `start_daemon` работал корректно в `.app` формате, файл `lh-daemon` должен находиться в папке `Contents/MacOS` рядом с `lh-gui`.

При использовании `cargo-bundle` для `lh-gui` он упакует только gui.
Вам нужно **вручную скопировать** скомпилированный daemon в бандл после сборки:

```bash
# 1. Собрать release версии
cargo build --release --workspace

# 2. Собрать bundle для GUI
cargo bundle --release -p lh-gui

# 3. Скопировать daemon внутрь бандла
cp target/release/lh-daemon "target/release/bundle/osx/Language Handler.app/Contents/MacOS/"
```

Теперь приложение `Language Handler.app` полностью автономно.
