use std::fs;
use std::io;
use std::path::PathBuf;

use log::warn;
use serde::Deserialize;
use winit::event::VirtualKeyCode;

const DEFAULT_SENSITIVITY: f32 = 0.05;

#[derive(Clone)]
pub struct AppConfig {
    pub mouse_sensitivity: f32,
    pub key_bindings: KeyBindings,
    pub present_mode: PresentModeSetting,
    pub max_fps: Option<f32>,
}

impl AppConfig {
    pub fn load() -> Self {
        let path = default_config_path();
        match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<RawConfig>(&bytes) {
                Ok(raw) => AppConfig::from_raw(raw),
                Err(err) => {
                    warn!("Failed to parse config file {}: {}", path.display(), err);
                    AppConfig::default()
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => AppConfig::default(),
            Err(err) => {
                warn!("Failed to read config file {}: {}", path.display(), err);
                AppConfig::default()
            }
        }
    }

    fn from_raw(raw: RawConfig) -> Self {
        let defaults = KeyBindings::default();
        let key_bindings = KeyBindings {
            forward: parse_key(raw.keymap.move_forward.as_deref(), defaults.forward),
            backward: parse_key(raw.keymap.move_backward.as_deref(), defaults.backward),
            left: parse_key(raw.keymap.move_left.as_deref(), defaults.left),
            right: parse_key(raw.keymap.move_right.as_deref(), defaults.right),
            up: parse_key(raw.keymap.move_up.as_deref(), defaults.up),
            down: parse_key(raw.keymap.move_down.as_deref(), defaults.down),
        };

        let mut sensitivity = raw.mouse_sensitivity.unwrap_or(DEFAULT_SENSITIVITY);
        if !sensitivity.is_finite() || sensitivity <= 0.0 {
            warn!(
                "Invalid mouse_sensitivity {}; falling back to default",
                sensitivity
            );
            sensitivity = DEFAULT_SENSITIVITY;
        }

        let present_mode = PresentModeSetting::from_raw(raw.present_mode);
        let max_fps = raw.max_fps.and_then(|v| {
            if v.is_finite() && v > 0.0 {
                Some(v.min(2400.0))
            } else {
                warn!("Invalid max_fps {}; ignoring", v);
                None
            }
        });

        Self {
            mouse_sensitivity: sensitivity,
            key_bindings,
            present_mode,
            max_fps,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: DEFAULT_SENSITIVITY,
            key_bindings: KeyBindings::default(),
            present_mode: PresentModeSetting::VSync,
            max_fps: None,
        }
    }
}

#[derive(Clone)]
pub struct KeyBindings {
    pub forward: VirtualKeyCode,
    pub backward: VirtualKeyCode,
    pub left: VirtualKeyCode,
    pub right: VirtualKeyCode,
    pub up: VirtualKeyCode,
    pub down: VirtualKeyCode,
}

impl KeyBindings {
    pub fn default() -> Self {
        Self {
            forward: VirtualKeyCode::W,
            backward: VirtualKeyCode::S,
            left: VirtualKeyCode::A,
            right: VirtualKeyCode::D,
            up: VirtualKeyCode::Space,
            down: VirtualKeyCode::LShift,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawConfig {
    mouse_sensitivity: Option<f32>,
    keymap: RawKeyMap,
    present_mode: Option<String>,
    max_fps: Option<f32>,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: Some(DEFAULT_SENSITIVITY),
            keymap: RawKeyMap::default(),
            present_mode: Some("vsync".into()),
            max_fps: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawKeyMap {
    move_forward: Option<String>,
    move_backward: Option<String>,
    move_left: Option<String>,
    move_right: Option<String>,
    move_up: Option<String>,
    move_down: Option<String>,
}

impl Default for RawKeyMap {
    fn default() -> Self {
        Self {
            move_forward: None,
            move_backward: None,
            move_left: None,
            move_right: None,
            move_up: None,
            move_down: None,
        }
    }
}

fn parse_key(name: Option<&str>, fallback: VirtualKeyCode) -> VirtualKeyCode {
    let Some(name) = name else {
        return fallback;
    };

    match key_from_str(name) {
        Some(code) => code,
        None => {
            warn!("Unknown key '{}' in config; using {:?}", name, fallback);
            fallback
        }
    }
}

fn key_from_str(name: &str) -> Option<VirtualKeyCode> {
    let normalized = name.trim();
    if normalized.len() == 1 {
        let ch = normalized.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            let upper = ch.to_ascii_uppercase();
            return Some(match upper {
                'A' => VirtualKeyCode::A,
                'B' => VirtualKeyCode::B,
                'C' => VirtualKeyCode::C,
                'D' => VirtualKeyCode::D,
                'E' => VirtualKeyCode::E,
                'F' => VirtualKeyCode::F,
                'G' => VirtualKeyCode::G,
                'H' => VirtualKeyCode::H,
                'I' => VirtualKeyCode::I,
                'J' => VirtualKeyCode::J,
                'K' => VirtualKeyCode::K,
                'L' => VirtualKeyCode::L,
                'M' => VirtualKeyCode::M,
                'N' => VirtualKeyCode::N,
                'O' => VirtualKeyCode::O,
                'P' => VirtualKeyCode::P,
                'Q' => VirtualKeyCode::Q,
                'R' => VirtualKeyCode::R,
                'S' => VirtualKeyCode::S,
                'T' => VirtualKeyCode::T,
                'U' => VirtualKeyCode::U,
                'V' => VirtualKeyCode::V,
                'W' => VirtualKeyCode::W,
                'X' => VirtualKeyCode::X,
                'Y' => VirtualKeyCode::Y,
                'Z' => VirtualKeyCode::Z,
                _ => return None,
            });
        }
        if ch.is_ascii_digit() {
            return Some(match ch {
                '0' => VirtualKeyCode::Key0,
                '1' => VirtualKeyCode::Key1,
                '2' => VirtualKeyCode::Key2,
                '3' => VirtualKeyCode::Key3,
                '4' => VirtualKeyCode::Key4,
                '5' => VirtualKeyCode::Key5,
                '6' => VirtualKeyCode::Key6,
                '7' => VirtualKeyCode::Key7,
                '8' => VirtualKeyCode::Key8,
                '9' => VirtualKeyCode::Key9,
                _ => return None,
            });
        }
    }

    match normalized.to_ascii_uppercase().as_str() {
        "SPACE" => Some(VirtualKeyCode::Space),
        "LSHIFT" | "SHIFT" => Some(VirtualKeyCode::LShift),
        "RSHIFT" => Some(VirtualKeyCode::RShift),
        "LCTRL" | "CTRL" => Some(VirtualKeyCode::LControl),
        "RCTRL" => Some(VirtualKeyCode::RControl),
        "LALT" | "ALT" => Some(VirtualKeyCode::LAlt),
        "RALT" => Some(VirtualKeyCode::RAlt),
        "TAB" => Some(VirtualKeyCode::Tab),
        "CAPSLOCK" => Some(VirtualKeyCode::Capital),
        "ESC" | "ESCAPE" => Some(VirtualKeyCode::Escape),
        "ENTER" | "RETURN" => Some(VirtualKeyCode::Return),
        "BACKSPACE" => Some(VirtualKeyCode::Back),
        "UP" => Some(VirtualKeyCode::Up),
        "DOWN" => Some(VirtualKeyCode::Down),
        "LEFT" => Some(VirtualKeyCode::Left),
        "RIGHT" => Some(VirtualKeyCode::Right),
        _ => None,
    }
}

fn default_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.json")
}

#[derive(Clone, Copy)]
pub enum PresentModeSetting {
    Immediate,
    Mailbox,
    VSync,
}

impl PresentModeSetting {
    fn from_raw(raw: Option<String>) -> Self {
        match raw
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("immediate") | Some("unlocked") | Some("off") => Self::Immediate,
            Some("mailbox") | Some("relaxed") => Self::Mailbox,
            Some("vsync") | Some("fifo") | Some("on") | None => Self::VSync,
            Some(other) => {
                warn!("Unknown present_mode '{}'; falling back to vsync", other);
                Self::VSync
            }
        }
    }
}
