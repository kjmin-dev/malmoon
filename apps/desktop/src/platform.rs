//! Window behavior that an overlay needs and GPUI does not expose.
//!
//! GPUI's pop-up window already floats above full-screen apps on every Space without taking focus.
//! Click-through and exclusion from screen capture have no GPUI API, so they are set on the native
//! window directly. Only macOS is implemented; elsewhere these are no-ops for now.

use gpui_kit::Window;

#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayOptions {
    /// Let clicks fall through to whatever is underneath.
    pub click_through: bool,
    /// Keep the window out of screen shares and recordings.
    pub hide_from_capture: bool,
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn configure_overlay(window: &Window, options: OverlayOptions) {
    use objc2_app_kit::{NSView, NSWindowSharingType};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: GPUI hands out the live content view of this window, and windows are only
    // created and configured on the main thread.
    let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let Some(native) = view.window() else {
        return;
    };

    native.setIgnoresMouseEvents(options.click_through);
    native.setSharingType(if options.hide_from_capture {
        NSWindowSharingType::None
    } else {
        NSWindowSharingType::ReadOnly
    });
}

#[cfg(not(target_os = "macos"))]
pub fn configure_overlay(_window: &Window, _options: OverlayOptions) {}
