use cocoa::base::id;
use std::os::raw::c_char;

pub const K_TIS_PROPERTY_INPUT_SOURCE_ID: &str = "TISPropertyInputSourceID";
pub const K_TIS_PROPERTY_LOCALIZED_NAME: &str = "TISPropertyLocalizedName";
pub const K_UTF8_ENCODING: u32 = 0x08000100;

#[allow(non_snake_case)]
extern "C" {
    pub fn CFRunLoopRun();
    pub fn TISCopyCurrentKeyboardInputSource() -> id;
    pub fn TISGetInputSourceProperty(input_source: id, property_key: id) -> id;
    pub fn TISCopyInputSourceForLanguage(language: id) -> id;
    pub fn TISSelectInputSource(input_source: id) -> i32;
    pub fn TISCreateInputSourceList(properties: id, include_all_installed: bool) -> id;
    pub fn CFStringGetCString(
        the_string: id,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
    pub fn CFStringGetLength(the_string: id) -> isize;
    pub fn CFGetTypeID(cf: id) -> usize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFRelease(cf: id);
    pub fn CFArrayGetCount(the_array: id) -> isize;
    pub fn CFArrayGetValueAtIndex(the_array: id, idx: isize) -> id;
    pub fn CFStringCreateWithCString(alloc: id, c_str: *const c_char, encoding: u32) -> id;
}

pub fn run_main_loop() {
    unsafe { CFRunLoopRun() }
}
