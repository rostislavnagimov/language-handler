# Инструкция по запуску Language Handler

После разделения на 3 crate у тебя теперь **два отдельных бинарника**:
- `lh-daemon` — фоновый мониторинг (headless, без GUI)
- `lh-gui` — окно настроек

---

## Вариант 1: Разработка (с логами в терминале)

### Запустить daemon с логами
```bash
cargo run -p lh-daemon
```
Вывод:
```
Loaded configuration from: ~/Library/Application Support/language-handler/config.json
Daemon started. Press Ctrl+C to exit.
Active window: Terminal | Layout: U.S. (com.apple.keylayout.US)
```

### Запустить GUI с логами
```bash
cargo run -p lh-gui
```
Откроется окно настроек, логи в терминале.

### Запустить оба одновременно
```bash
# Терминал 1: daemon в фоне
cargo run -p lh-daemon &

# Терминал 2: GUI
cargo run -p lh-gui
```

---

## Вариант 2: Release сборка (оптимизированная)

```bash
cargo build --release --workspace
```

Бинарники появятся в:
- `target/release/lh-daemon`
- `target/release/lh-gui`

Запуск:
```bash
./target/release/lh-daemon &
./target/release/lh-gui
```

---

## Вариант 3: macOS App Bundle (как раньше)

> ⚠️ После разделения `cargo bundle` требует дополнительной настройки для каждого crate.

### Установка cargo-bundle
```bash
cargo install cargo-bundle
```

### Добавить metadata в gui/Cargo.toml
```toml
[package.metadata.bundle]
name = "Language Handler"
identifier = "com.languagehandler.gui"
category = "Utility"
short_description = "Automatic keyboard layout switcher"
icon = ["../assets/icon.icns"]  # если есть иконка
```

### Собрать bundle GUI
```bash
cargo bundle --release -p lh-gui
```

Bundle появится в:
```
target/release/bundle/osx/Language Handler.app/
```

### Запуск bundle с логами
```bash
./target/release/bundle/osx/Language\ Handler.app/Contents/MacOS/lh-gui
```

### Запуск как обычное приложение
```bash
open target/release/bundle/osx/Language\ Handler.app
```

---

## Полное развёртывание (daemon + GUI)

Для production использования нужно:

1. **Daemon как LaunchAgent** (автозапуск при логине):
   ```bash
   # Скопировать daemon в /usr/local/bin
   sudo cp target/release/lh-daemon /usr/local/bin/
   
   # Создать LaunchAgent (см. IMPLEMENTATION_PLAN.md секция 4.4)
   ```

2. **GUI как обычное приложение**:
   ```bash
   # Скопировать в /Applications
   cp -r target/release/bundle/osx/Language\ Handler.app /Applications/
   ```

---

## Быстрая проверка (эквивалент старого способа)

**Раньше:**
```bash
cargo clean
cargo bundle --release
./target/release/bundle/osx/Language\ Handler.app/Contents/MacOS/language-handler
```

**Теперь (daemon + gui отдельно):**
```bash
# С логами (разработка)
cargo run -p lh-daemon &
cargo run -p lh-gui

# Или release
cargo build --release -p lh-daemon -p lh-gui
./target/release/lh-daemon &
./target/release/lh-gui
```

**Теперь (bundle GUI):**
```bash
cargo bundle --release -p lh-gui
./target/release/bundle/osx/Language\ Handler.app/Contents/MacOS/lh-gui
```

---

## Размеры бинарников (release)

| Бинарник | Размер |
|----------|--------|
| `lh-daemon` | 554 KB |
| `lh-gui` | 456 KB |

Проверить:
```bash
ls -lh target/release/lh-daemon target/release/lh-gui
```
