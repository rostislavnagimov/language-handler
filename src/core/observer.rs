use crate::core::{monitor, switcher};
use crate::state;

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

fn create_observer_class() -> *const objc::runtime::Class {
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

extern "C" fn app_changed_callback(_self: &Object, _cmd: Sel, _notification: id) {
    let _pool = unsafe { NSAutoreleasePool::new(nil) };
    unsafe {
        monitor::update_active_window();
        monitor::update_keyboard_layout();
        switcher::check_and_switch_layout_by_rules();

        if let (Some(app), Some(layout)) = (
            &*std::ptr::addr_of!(state::CURRENT_APP),
            &*std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT),
        ) {
            println!("Active window: {} | Layout: {}", app, layout);
        }
    }
}

extern "C" fn keyboard_changed_callback(_self: &Object, _cmd: Sel, _notification: id) {
    let _pool = unsafe { NSAutoreleasePool::new(nil) };
    unsafe {
        monitor::update_keyboard_layout();

        if let (Some(app), Some(layout)) = (
            &*std::ptr::addr_of!(state::CURRENT_APP),
            &*std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT),
        ) {
            println!("Layout changed: {} | App: {}", layout, app);
        }
    }
}
/// Creates an observer and subscribes it to system notifications for application
/// activation and keyboard layout changes.
///
/// # Safety
///
/// This function is unsafe because:
/// 1. It calls FFI functions (Objective-C runtime via `msg_send!`) to interact with Cocoa.
/// 2. It dynamically creates and registers an Objective-C class (`WindowObserver`) at runtime.
/// 3. It interacts with global system notification centers.
///     This function should ideally be called only once during application initialization
///     and from the main thread.
pub unsafe fn setup_observers() {
    let observer_class = create_observer_class();
    let observer: id = msg_send![observer_class, new];

    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let notification_center: id = msg_send![workspace, notificationCenter];

    let app_notification_name =
        NSString::alloc(nil).init_str("NSWorkspaceDidActivateApplicationNotification");
    let _: () = msg_send![notification_center,
        addObserver: observer
        selector: sel!(appChanged:)
        name: app_notification_name
        object: nil
    ];

    let default_center: id = msg_send![class!(NSNotificationCenter), defaultCenter];

    let keyboard_notification_name =
        NSString::alloc(nil).init_str("NSTextInputContextKeyboardSelectionDidChangeNotification");
    let _: () = msg_send![default_center,
        addObserver: observer
        selector: sel!(keyboardChanged:)
        name: keyboard_notification_name
        object: nil
    ];

    let tis_notification_name =
        NSString::alloc(nil).init_str("kTISNotifySelectedKeyboardInputSourceChanged");
    let _: () = msg_send![default_center,
        addObserver: observer
        selector: sel!(keyboardChanged:)
        name: tis_notification_name
        object: nil
    ];
}
