# macOS Language Handler

**A utility to automatically switch your macOS keyboard layout based on the active application.**

### Disclaimer: Test with caution! This is a pre-alpha version!

## Why?

Anyone who uses more than one keyboard layout daily knows the struggle: the person who designed the layout switching flow for macOS probably only uses one language and doesn't rely on their own feature. Otherwise, I can't explain why it's still so cumbersome.

I'm someone who uses two layouts (Russian and English) every day and typically switches them about 10 times a minute – one chat in Russian, another in English, and so on. And yes, I'm the guy who types 'пше' instead of 'git' far too often.

So, I built this tool to save my time and nerves. I originally built it for myself, but you're welcome to use it too.

## How it Works

When you first run Language Handler, it creates a configuration file. This config file is simple: `"%APP_NAME%": "%LANGUAGE_CODE%"`.

* **`%APP_NAME%`**: The name of the application (e.g., "Terminal", "Google Chrome").
* **`%LANGUAGE_CODE%`**: A short code for your desired layout.

**Currently supported language codes (for the config file):**
* `EN` (English/US)
* `RU` (Russian)
* `CN` (Chinese - Simplified Pinyin)
* `HI` (Hindi - Devanagari QWERTY)

**Not sure about the application name?** Check the tool's log output (the Terminal window it opens). When you focus on an application window, its name will be shown there.

Once everything is set up, Language Handler will automatically change your keyboard layout when you focus on an application listed in your config file.

## How to Use

1.  **Download the Binary**

    Run this command in your Terminal:
    ```
    curl -LO https://github.com/rostislavnagimov/language-handler/releases/download/v0.0.1/language-handler-macos-arm.zip
    ```
    The `.zip` archive will be downloaded to your current working directory.
    *Note: If you download the file through a web browser, macOS might warn you about an "Unknown developer" or even prevent you from running the tool. Using `curl` as shown above usually avoids these issues.*

3.  **Unzip and Run**
    * Unzip the archive (e.g., by double-clicking it in Finder, or using `unzip language-handler-macos-arm.zip` in Terminal).
    * Run the `language-handler` binary (e.g., by double-clicking it or running `./language-handler` in Terminal if you are in the same directory).

    You will see a new Terminal window appear with logs from the tool. **The current version requires this Terminal window to remain open while the tool is working.** You can hide or minimize this window, but please don't close it if you want the tool to keep running.

    On the first launch, a `config.json` file will be created with default rules. This is a starting point, but you'll likely want to customize it.

    **Important:** To apply any changes made to the `config.json` file, you need to:
    1.  Stop the currently running version of Language Handler (by closing its Terminal window or pressing `Ctrl+C` in that window).
    2.  Run it again.

4.  **Edit the Configuration File**
    To edit your `config.json` file, copy and paste this command into your Terminal:
    ```
    open "$HOME/Library/Application Support/language-handler/config.json"
    ```
    This will open the config file in your default text editor. After you've set your rules, save the changes and restart Language Handler as described above.

    **Example `config.json`:**
    ```json
    {
      "Terminal": "EN",
      "iTerm2": "EN",
      "Google Chrome": "EN",
      "Telergam": "RU"
    }
    ```

## Building from Source (Optional)

If you prefer to build from source:
1.  Ensure you have Rust installed: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2.  Clone this repository.
3.  Navigate to the project directory and run `cargo build --release`.
4.  The executable will be in `target/release/language-handler`.

## Contributing

First you can donate me on Solana : ```pG9TZUjpmtbbvMU8MjKpjbdvBcXLcHWQsyM2Qqq4BpB```

Second you can offer me a job: [resume with all the contact data](https://drive.google.com/file/d/1o8lOwgBqpbccm-I-g5rkBLs4DM9qTqD7/view)

Third you can create a Pull Request or Issue with your ideas on optimization or founded bugs.

Also you can rate the repo, I will really appreciate that.

## License

 Working under my personal 'I Don't Care' licence, free to use for everyone, go ahead.
