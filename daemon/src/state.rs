use std::collections::HashMap;

pub static mut CURRENT_APP: Option<String> = None;
pub static mut CURRENT_KEYBOARD_LAYOUT: Option<String> = None;
pub static mut APP_LAYOUT_RULES: Option<HashMap<String, String>> = None;