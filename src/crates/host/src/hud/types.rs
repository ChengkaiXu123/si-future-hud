//! Shared HUD data types, configuration contracts, and native capability reports.
use serde::{Deserialize, Serialize};

use super::Shared;

/// Logical viewport coordinates, as returned by DOM getBoundingClientRect().
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveRegion {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: f64,
    pub(super) height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudInputStatus {
    pub(super) region_count: usize,
    pub(super) ignore_cursor: bool,
    pub(super) pointer_polling: bool,
    pub(super) pointer: HudPointer,
}

/// The native input owner supplies hover independently of DOM enter/leave delivery.
/// The local HUD can also use outside points for Bot gaze. Unavailable button and
/// modifier state stays null; no outside keyboard metadata is sampled.
#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HudPointer {
    pub(super) screen_id: String,
    pub(super) inside: bool,
    pub(super) point: Option<Point>,
    pub(super) buttons: Option<u32>,
    pub(super) alt_key: Option<bool>,
    pub(super) ctrl_key: Option<bool>,
    pub(super) meta_key: Option<bool>,
    pub(super) shift_key: Option<bool>,
}

impl HudPointer {
    pub(super) fn outside(owner: &Shared) -> Self {
        Self {
            screen_id: owner.screen_id.clone(),
            inside: false,
            point: None,
            buttons: None,
            alt_key: None,
            ctrl_key: None,
            meta_key: None,
            shift_key: None,
        }
    }
}

#[derive(Default)]
pub(super) struct PointerKeys {
    pub(super) buttons: Option<u32>,
    pub(super) alt: Option<bool>,
    pub(super) ctrl: Option<bool>,
    pub(super) meta: Option<bool>,
    pub(super) shift: Option<bool>,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Notch,
    #[default]
    Logos,
    Simple,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}

impl Edge {
    pub(super) fn horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// CSS viewport coordinates of the original mouse-down, before the pickup threshold is met.
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragPointer {
    pub(super) x: f64,
    pub(super) y: f64,
}

/// Measured runs of the mode used after pickup (Logos when the original mode is Notch).
/// Simple's horizontal row and vertical stack have different lengths when labels are shown.
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragRuns {
    pub(super) horizontal: f64,
    pub(super) vertical: f64,
}

#[derive(Clone, Copy, Serialize, PartialEq)]
pub struct Point {
    pub(super) x: f64,
    pub(super) y: f64,
}

#[derive(Clone, Copy)]
pub(super) struct Bounds {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: f64,
    pub(super) height: f64,
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HudPlacement {
    pub(super) screen_id: String,
    pub(super) mode: Mode,
    pub(super) edge: Edge,
    pub(super) offset: f64,
    pub(super) monitor_index: usize,
}

#[derive(Clone, Serialize)]
pub(super) struct DragEnd {
    #[serde(rename = "screenId")]
    pub(super) screen_id: String,
    pub(super) cancelled: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct HudConfig {
    pub mode: Mode,
    pub edge: Edge,
    /// Content size in logical points; converted using the selected display's scale.
    pub width: f64,
    pub height: f64,
    /// Fraction of available travel along the edge, left-to-right or top-to-bottom.
    pub offset: f64,
    pub ignore_cursor: bool,
    pub visible: bool,
    /// Session-only monitor index from hud_monitors; it is not a persistent display ID.
    pub monitor_index: usize,
    /// Exact hud_monitors.screenId; preferred over a transient monitor index when present.
    pub screen_id: Option<String>,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Logos,
            edge: Edge::Top,
            width: 360.0,
            height: 36.0,
            offset: 0.5,
            ignore_cursor: true,
            visible: true,
            monitor_index: 0,
            screen_id: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

/// Coordinates use HudGeometry.coordinateSpace. macOS uses uniform global screen points;
/// other hosts divide both screen and canvas physical coordinates by the selected monitor's
/// scale, which preserves local differences but is not a uniform global logical desktop.
/// Viewport rectangles use the same point units, relative to the actual HUD canvas top-left.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalRect {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: f64,
    pub(super) height: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudGeometry {
    pub(super) screen_id: String,
    pub(super) monitor_index: usize,
    pub(super) scale_factor: f64,
    pub(super) coordinate_space: &'static str,
    pub(super) screen: LogicalRect,
    pub(super) menu_bar_height: f64,
    pub(super) has_notch: bool,
    pub(super) notch: Option<LogicalRect>,
    pub(super) canvas: LogicalRect,
    pub(super) notch_in_viewport: Option<LogicalRect>,
    pub(super) placement_constrained: bool,
}

/// A display as the host measures it: identity, points, and the top bar.
#[derive(Clone)]
pub(super) struct ScreenMeasurement {
    pub(super) runtime_id: String,
    pub(super) persistent_id: Option<String>,
    pub(super) name: Option<String>,
    pub(super) screen: LogicalRect,
    pub(super) scale_factor: f64,
    pub(super) menu_bar_height: f64,
    pub(super) notch: Option<LogicalRect>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudMonitor {
    pub(super) screen_id: String,
    pub(super) window_label: String,
    pub(super) index: usize,
    pub(super) runtime_id: String,
    pub(super) persistent_id: Option<String>,
    pub(super) name: Option<String>,
    pub(super) primary: bool,
    pub(super) bounds: Rect,
    pub(super) work_area: Rect,
    pub(super) scale_factor: f64,
    /// Global logical points, top-left origin; bounds/workArea remain native physical coordinates.
    pub(super) notch: Option<LogicalRect>,
    pub(super) notch_coordinate_space: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudCapabilities {
    pub(super) platform: &'static str,
    pub(super) backend: &'static str,
    pub(super) lazy_creation: bool,
    pub(super) single_window: bool,
    pub(super) per_screen_windows: bool,
    pub(super) transparent_surface: bool,
    pub(super) native_material: bool,
    pub(super) non_focusing_window: bool,
    pub(super) native_nonactivating_panel: bool,
    pub(super) keyboard_handoff: bool,
    pub(super) cursor_passthrough: bool,
    pub(super) interactive_regions: bool,
    pub(super) native_drag: bool,
    pub(super) global_pointer: &'static str,
    pub(super) absolute_edge_position: &'static str,
    pub(super) all_workspaces: &'static str,
    pub(super) fullscreen_overlay: &'static str,
    pub(super) hardware_notch: bool,
    pub(super) persistent_display_identity: bool,
    pub(super) limitations: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudWindowState {
    pub(super) screen_id: String,
    pub(super) window_label: String,
    pub(super) created: bool,
    pub(super) visible: bool,
    pub(super) config: HudConfig,
    pub(super) frame: Option<Rect>,
    pub(super) requested_frame: Option<Rect>,
    pub(super) capabilities: HudCapabilities,
    pub(super) geometry: Option<HudGeometry>,
}

#[tauri::command]
pub fn hud_native_capabilities() -> HudCapabilities {
    super::host::capabilities()
}

/// Limitations every host reports before its own.
pub(super) fn base_limitations() -> Vec<&'static str> {
    vec![
        "Native focus, fullscreen and Spaces behavior requires platform GUI qualification; compile checks do not certify it.",
        "Monitor indices are session-only. A non-null persistentId identifies the host's display device; hardware or driver changes can alter it.",
        "Each configured connected screen owns a separate lazy HUD window and pointer worker.",
        "Reported rectangles control input using a native 32 ms pointer poll; this is not the original event-driven hover state machine or per-pixel hit testing.",
    ]
}
