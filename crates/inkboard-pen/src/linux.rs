//! Read the kernel's current Wacom pressure state, independent of compositor pointer grabs.

use crate::{PEN_FLAG_BARREL, PEN_FLAG_ERASER, PenSample, PointerKind};
use evdev::{AbsoluteAxisCode, Device, KeyCode};
use std::io;
use std::path::Path;

/// A read-only Wacom pen handle. Access is limited by the active user's device ACL.
#[derive(Debug)]
pub struct LinuxTablet {
    device: Device,
    sample: PenSample,
}

impl LinuxTablet {
    /// Open a Wacom pen exposing a pressure axis. Does not grab the device or modify its settings.
    ///
    /// # Errors
    /// Returns an error if no accessible Wacom pen exists. Install the scoped udev rule using
    /// `scripts/setup-wacom-linux.sh`, then restart the application.
    pub fn connect() -> io::Result<Self> {
        let mut last_error = io::Error::new(io::ErrorKind::NotFound, "no Wacom pressure device found");
        for entry in std::fs::read_dir("/sys/class/input")?.flatten() {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("event") || !is_wacom_pen(&entry.path()) {
                continue;
            }
            match Device::open(Path::new("/dev/input").join(name)) {
                Ok(device) if device.supported_absolute_axes().is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_PRESSURE)) => {
                    return Ok(Self { device, sample: PenSample::NONE });
                }
                Err(error) => last_error = error,
                Ok(_) => {}
            }
        }
        Err(last_error)
    }

    /// Query the kernel's live state before the canvas consumes this frame's pointer input.
    /// State queries keep updating during a compositor/window-library grab, unlike secondary X connections.
    pub fn poll(&mut self) -> Option<f32> {
        let (Ok(axes), Ok(keys)) = (self.device.get_abs_state(), self.device.get_key_state()) else {
            self.sample = PenSample::NONE;
            crate::record(self.sample);
            return None;
        };
        let pressure = &axes[usize::from(AbsoluteAxisCode::ABS_PRESSURE.0)];
        let sample = sample(
            pressure.value,
            pressure.minimum,
            pressure.maximum,
            keys.contains(KeyCode::BTN_TOOL_PEN),
            keys.contains(KeyCode::BTN_TOOL_RUBBER),
            keys.contains(KeyCode::BTN_STYLUS) || keys.contains(KeyCode::BTN_STYLUS2),
            (axes[usize::from(AbsoluteAxisCode::ABS_TILT_X.0)].value, axes[usize::from(AbsoluteAxisCode::ABS_TILT_Y.0)].value),
        );
        if sample != self.sample {
            self.sample = sample;
            crate::record(sample);
        }
        // A hovering pen must not turn a simultaneous mouse stroke into a pressure stroke.
        (sample.kind == PointerKind::Pen && sample.pressure > 0.0).then_some(sample.pressure)
    }
}

fn is_wacom_pen(path: &Path) -> bool {
    let vendor = std::fs::read_to_string(path.join("device/id/vendor")).unwrap_or_default();
    let name = std::fs::read_to_string(path.join("device/name")).unwrap_or_default().to_ascii_lowercase();
    vendor.trim() == "056a" && (name.contains("pen") || name.contains("stylus")) && !name.contains("pad")
}

#[allow(clippy::too_many_arguments, reason = "native pen state consists of pressure range, tool flags and tilt")]
fn sample(value: i32, min: i32, max: i32, pen: bool, eraser: bool, barrel: bool, tilt: (i32, i32)) -> PenSample {
    if !pen && !eraser {
        return PenSample::NONE;
    }
    PenSample {
        kind: PointerKind::Pen,
        pressure: normalized(value, min, max),
        flags: (if eraser { PEN_FLAG_ERASER } else { 0 }) | (if barrel { PEN_FLAG_BARREL } else { 0 }),
        tilt,
        seq: 0,
    }
}

#[allow(clippy::cast_possible_truncation, reason = "pressure is clamped to 0..1 before conversion")]
fn normalized(value: i32, min: i32, max: i32) -> f32 {
    if max <= min {
        return 0.0;
    }
    ((f64::from(value) - f64::from(min)) / (f64::from(max) - f64::from(min))).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_uses_the_hardware_range_and_is_clamped() {
        assert!((normalized(2_048, 0, 4_095) - 0.5).abs() < 0.001);
        assert!((normalized(32_768, 0, 65_535) - 0.5).abs() < 0.001);
        assert!(normalized(-1, 0, 4_095).abs() < f32::EPSILON);
        assert!((normalized(5_000, 0, 4_095) - 1.0).abs() < f32::EPSILON);
        assert!(normalized(10, 1, 1).abs() < f32::EPSILON);
    }

    #[test]
    fn held_pen_pressure_keeps_changing_without_new_pointer_events() {
        let pressures: Vec<_> = [100, 1_000, 3_500, 500].into_iter().map(|p| sample(p, 0, 4_095, true, false, false, (0, 0)).pressure).collect();
        assert!(pressures[0] < pressures[1] && pressures[1] < pressures[2] && pressures[2] > pressures[3]);
        assert_eq!(sample(0, 0, 4_095, false, false, false, (0, 0)), PenSample::NONE);
        let eraser = sample(2_000, 0, 4_095, false, true, true, (10, -5));
        assert!(eraser.eraser() && eraser.barrel());
        assert_eq!(eraser.tilt, (10, -5));
    }
}
