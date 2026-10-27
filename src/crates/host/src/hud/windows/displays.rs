//! Map Tauri's GDI display names to Windows monitor device paths. These paths
//! survive enumeration-order changes; they are not portable hardware serials.
use super::super::{generic, ScreenMeasurement};
use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    },
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, WIN32_ERROR},
};

#[derive(Debug)]
struct Identity {
    source: String,
    device_path: Option<String>,
    name: Option<String>,
}

fn utf16_string(value: &[u16]) -> Option<String> {
    let length = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16(&value[..length])
        .ok()
        .filter(|value| !value.is_empty())
}

fn active_paths() -> Result<Vec<DISPLAYCONFIG_PATH_INFO>, WIN32_ERROR> {
    // A display can connect between the size query and the snapshot. Windows
    // asks callers to obtain the sizes again on ERROR_INSUFFICIENT_BUFFER.
    for _ in 0..3 {
        let (mut path_count, mut mode_count) = (0, 0);
        let status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        if path_count == 0 {
            return Ok(Vec::new());
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        let status = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if status == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        paths.truncate(path_count as usize);
        return Ok(paths);
    }
    Err(ERROR_INSUFFICIENT_BUFFER)
}

fn identity(path: &DISPLAYCONFIG_PATH_INFO) -> Option<Identity> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: path.sourceInfo.adapterId,
            id: path.sourceInfo.id,
        },
        ..Default::default()
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut source.header) } != 0 {
        return None;
    }
    let source = utf16_string(&source.viewGdiDeviceName)?;
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
            adapterId: path.targetInfo.adapterId,
            id: path.targetInfo.id,
        },
        ..Default::default()
    };
    let target_available = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) } == 0;
    // Keep the source even when its target query fails: another successful path
    // for the same mirrored source must not be mistaken for a unique display.
    Some(Identity {
        source,
        device_path: target_available
            .then(|| utf16_string(&target.monitorDevicePath))
            .flatten(),
        name: target_available
            .then(|| utf16_string(&target.monitorFriendlyDeviceName))
            .flatten(),
    })
}

fn unique_identity<'a>(identities: &'a [Identity], source: &str) -> Option<&'a Identity> {
    let mut matches = identities
        .iter()
        .filter(|identity| identity.source.eq_ignore_ascii_case(source));
    let candidate = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let path = candidate.device_path.as_deref()?;
    // Never give two independently placed HUD windows the same persistent key.
    (identities
        .iter()
        .filter(|identity| {
            identity
                .device_path
                .as_deref()
                .is_some_and(|other| other.eq_ignore_ascii_case(path))
        })
        .count()
        == 1)
        .then_some(candidate)
}

fn persistent_id(identity: &Identity) -> Option<String> {
    identity
        .device_path
        .as_ref()
        .map(|path| format!("windows-display:{}", path.to_ascii_lowercase()))
}

fn runtime_id(source: Option<&str>, index: usize) -> String {
    match source {
        Some(source) => format!("monitor:{}", source.to_ascii_lowercase()),
        None => format!("monitor:{index}"),
    }
}

pub(in crate::hud) fn measure_screens(
    app: &tauri::AppHandle,
) -> Result<Vec<ScreenMeasurement>, String> {
    // Remote sessions and non-WDDM drivers can deny this optional API. Keep
    // rendering through Tauri with nullable persistent IDs in those cases.
    let Ok(paths) = active_paths() else {
        return generic::measure_screens(app);
    };
    let identities: Vec<_> = paths.iter().filter_map(identity).collect();
    app.available_monitors()
        .map_err(|error| error.to_string())?
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let mut measured = generic::screen(&[], monitor, index)?;
            let source = monitor.name().map(String::as_str);
            measured.runtime_id = runtime_id(source, index);
            if let Some(identity) = source.and_then(|source| unique_identity(&identities, source)) {
                measured.persistent_id = persistent_id(identity);
                if identity.name.is_some() {
                    measured.name.clone_from(&identity.name);
                }
            }
            Ok(measured)
        })
        .collect()
}

pub(in crate::hud) fn screen(
    screens: &[ScreenMeasurement],
    monitor: &tauri::Monitor,
    index: usize,
) -> Result<ScreenMeasurement, String> {
    let mut measured = generic::screen(&[], monitor, index)?;
    measured.runtime_id = runtime_id(monitor.name().map(String::as_str), index);
    if let Some(native) = screens
        .iter()
        .find(|native| native.runtime_id == measured.runtime_id)
    {
        measured.persistent_id.clone_from(&native.persistent_id);
        measured.name.clone_from(&native.name);
    }
    Ok(measured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_SUPPORTED};

    fn display(source: &str, device_path: &str) -> Identity {
        Identity {
            source: source.into(),
            device_path: Some(device_path.into()),
            name: Some("Same model".into()),
        }
    }

    #[test]
    fn identity_follows_device_path_when_display_order_changes() {
        let before = [
            display(r"\\.\DISPLAY1", r"\\?\DISPLAY#ABC#first"),
            display(r"\\.\DISPLAY2", r"\\?\DISPLAY#ABC#second"),
        ];
        let after = [
            display(r"\\.\DISPLAY2", r"\\?\display#abc#FIRST"),
            display(r"\\.\DISPLAY1", r"\\?\DISPLAY#ABC#second"),
        ];
        let id = |identities: &[Identity], source: &str| {
            persistent_id(unique_identity(identities, source).unwrap()).unwrap()
        };
        assert_eq!(id(&before, r"\\.\DISPLAY1"), id(&after, r"\\.\display2"));
        assert_ne!(id(&before, r"\\.\DISPLAY1"), id(&before, r"\\.\DISPLAY2"));
    }

    #[test]
    fn mirrored_missing_and_duplicate_targets_use_session_identity() {
        let source = r"\\.\DISPLAY1";
        let mut identities = vec![display(source, "first"), display(source, "second")];
        assert!(unique_identity(&identities, source).is_none());
        identities[1].device_path = None;
        assert!(unique_identity(&identities, source).is_none());
        identities.remove(0);
        assert!(unique_identity(&identities, source).is_none());
        identities = vec![display(source, "same"), display(r"\\.\DISPLAY2", "SAME")];
        assert!(unique_identity(&identities, source).is_none());
        assert!(unique_identity(&identities, "unknown").is_none());
        assert_eq!(runtime_id(None, 2), "monitor:2");
    }

    #[test]
    fn native_display_query_accepts_active_or_unavailable_console() {
        match active_paths() {
            Ok(paths) => {
                for identity in paths.iter().filter_map(identity) {
                    assert!(!identity.source.is_empty());
                    if let Some(path) = identity.device_path {
                        assert!(!path.is_empty());
                    }
                }
            }
            // Windows documents these for remote/non-console sessions and
            // drivers without WDDM. A hosted runner need not have a display.
            Err(status) => assert!(
                matches!(status, ERROR_ACCESS_DENIED | ERROR_NOT_SUPPORTED),
                "unexpected display configuration error: {status:?}"
            ),
        }
    }
}
