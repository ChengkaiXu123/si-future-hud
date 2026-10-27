//! Running applications, by bundle identifier, through NSRunningApplication.
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
use objc2_foundation::NSString;

/// This host can find and bring forward a running application.
pub(crate) const ACTIVATES_APPS: bool = true;

pub(crate) fn app_running(bundle_id: &str) -> bool {
    !NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle_id))
        .is_empty()
}

pub(crate) fn activate_app(bundle_id: &str) -> bool {
    let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
        bundle_id,
    ));
    let Some(app) = apps.firstObject() else {
        return false;
    };
    #[allow(deprecated)]
    app.activateWithOptions(NSApplicationActivationOptions::empty())
}
