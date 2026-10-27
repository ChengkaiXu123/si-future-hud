//! The global ⌘⌥H shortcut through Carbon's RegisterEventHotKey.
use std::ffi::c_void;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EventHotKeyID {
    signature: u32,
    id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> *mut c_void;
    fn InstallEventHandler(
        target: *mut c_void,
        handler: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
        num_types: u32,
        list: *const EventTypeSpec,
        user_data: *mut c_void,
        out_ref: *mut *mut c_void,
    ) -> i32;
    fn RegisterEventHotKey(
        hot_key_code: u32,
        hot_key_modifiers: u32,
        hot_key_id: EventHotKeyID,
        target: *mut c_void,
        options: u32,
        out_ref: *mut *mut c_void,
    ) -> i32;
    fn GetEventParameter(
        event: *mut c_void,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        buffer_size: usize,
        actual_size: *mut usize,
        data: *mut c_void,
    ) -> i32;
}

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
const SIGNATURE: u32 = 0x4148_5544; // 'AHUD'
const KEY_H: u32 = 4; // kVK_ANSI_H
const CMD_OPTION: u32 = (1 << 8) | (1 << 11); // cmdKey | optionKey
const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7962; // 'keyb'
const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2D2D_2D2D; // '----'
const TYPE_EVENT_HOT_KEY_ID: u32 = 0x686B_6964; // 'hkid'

unsafe extern "C" fn hotkey_callback(
    _call_ref: *mut c_void,
    event: *mut c_void,
    _user_data: *mut c_void,
) -> i32 {
    let mut hk_id = EventHotKeyID::default();
    let status = unsafe {
        GetEventParameter(
            event,
            K_EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOT_KEY_ID,
            std::ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>(),
            std::ptr::null_mut(),
            (&mut hk_id as *mut EventHotKeyID).cast::<c_void>(),
        )
    };
    if status == 0 && hk_id.id == 1 {
        if let Some(app) = APP_HANDLE.get() {
            crate::coordinator::toggle_glow(app);
        }
    }
    0
}

pub fn install(app: &tauri::AppHandle) {
    if APP_HANDLE.set(app.clone()).is_err() {
        return;
    }
    let spec = EventTypeSpec {
        event_class: K_EVENT_CLASS_KEYBOARD,
        event_kind: K_EVENT_HOT_KEY_PRESSED,
    };
    let mut handler_ref: *mut c_void = std::ptr::null_mut();
    let mut hotkey_ref: *mut c_void = std::ptr::null_mut();
    let hk_id = EventHotKeyID {
        signature: SIGNATURE,
        id: 1,
    };
    unsafe {
        let target = GetApplicationEventTarget();
        let _ = InstallEventHandler(
            target,
            hotkey_callback,
            1,
            &spec,
            std::ptr::null_mut(),
            &mut handler_ref,
        );
        let _ = RegisterEventHotKey(KEY_H, CMD_OPTION, hk_id, target, 0, &mut hotkey_ref);
    }
}
