//! The Linux HUD host: an ordinary GTK/WebKit window. X11 placement depends on
//! the window manager; a Wayland layer-shell and global-input backend is not
//! implemented, so Wayland refuses an edge HUD.
pub(super) use super::generic::{
    begin_text_focus, end_text_focus, global_pointer, measure_screens, panel_builder, place,
    screen, set_drag_cursor, shown, TextFocusOrigin,
};
use super::{base_limitations, HudCapabilities, PointerKeys};

pub(super) const GLOBAL_POINTS: bool = false;
pub(super) const COORDINATE_SPACE: &str = "monitor-scaled-physical-top-left";
/// No global mouse-button backend is installed on Linux.
pub(super) const NATIVE_DRAG: bool = false;

pub(super) fn wayland_session() -> bool {
    std::env::var("GDK_BACKEND").ok().as_deref() != Some("x11")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland"))
}

pub(super) fn left_mouse_down() -> Result<bool, String> {
    Err("Native HUD dragging has no global mouse-button backend on Linux".into())
}

pub(super) fn pointer_keys() -> PointerKeys {
    PointerKeys::default()
}

pub(super) fn capabilities() -> HudCapabilities {
    let wayland = wayland_session();
    let mut limitations = base_limitations();
    if wayland {
        limitations.push("Wayland global cursor coordinates and absolute top-level positioning are unavailable here. Showing an edge HUD is refused until a layer-shell backend is implemented.");
    } else {
        limitations.push("X11 stacking, all-workspace visibility and transparency depend on the window manager and compositor.");
    }
    limitations.push(
        "Native HUD dragging is unsupported on Linux; no global mouse-button backend is installed.",
    );
    HudCapabilities {
        platform: std::env::consts::OS,
        backend: if wayland { "wayland" } else { "x11" },
        lazy_creation: true,
        single_window: false,
        per_screen_windows: true,
        transparent_surface: true,
        native_material: false,
        non_focusing_window: true,
        native_nonactivating_panel: false,
        keyboard_handoff: false,
        cursor_passthrough: true,
        interactive_regions: !wayland,
        native_drag: NATIVE_DRAG,
        global_pointer: if wayland {
            "unsupported"
        } else {
            "native_cursor_polling"
        },
        absolute_edge_position: if wayland {
            "unsupported"
        } else {
            "window_manager_dependent"
        },
        all_workspaces: if wayland {
            "unsupported"
        } else {
            "window_manager_dependent"
        },
        fullscreen_overlay: "not_guaranteed",
        hardware_notch: false,
        persistent_display_identity: false,
        limitations,
    }
}
