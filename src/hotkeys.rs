//! User configurable global hotkeys. Only RegisterHotKey is used: no keyboard
//! hook, timer interception, or modification of game input.

use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Preset(u32),
    Next,
    Previous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub action: Action,
    pub key: u32,
    pub modifiers: u32,
}

pub const PRESETS: [u32; 7] = [1, 5, 12, 15, 20, 30, 50];
const ACTIONS: [Action; 10] = [
    Action::Preset(1),
    Action::Preset(1),
    Action::Preset(5),
    Action::Preset(12),
    Action::Preset(15),
    Action::Preset(20),
    Action::Preset(30),
    Action::Preset(50),
    Action::Next,
    Action::Previous,
];
const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;

pub fn defaults() -> Vec<Binding> {
    let keys = [
        b'0' as u32,
        b'1' as u32,
        b'2' as u32,
        b'3' as u32,
        b'4' as u32,
        b'5' as u32,
        b'6' as u32,
        b'7' as u32,
        0x26,
        0x28,
    ];
    ACTIONS
        .iter()
        .zip(keys)
        .map(|(&action, key)| Binding {
            action,
            key,
            modifiers: MOD_CONTROL | MOD_ALT,
        })
        .collect()
}

pub fn label(binding: Binding) -> String {
    let mut parts = Vec::new();
    if binding.modifiers & MOD_CONTROL != 0 {
        parts.push("Ctrl".to_string());
    }
    if binding.modifiers & MOD_ALT != 0 {
        parts.push("Alt".to_string());
    }
    if binding.modifiers & MOD_SHIFT != 0 {
        parts.push("Shift".to_string());
    }
    let key = match binding.key {
        0x26 => "Up".to_string(),
        0x28 => "Down".to_string(),
        0x70..=0x87 => format!("F{}", binding.key - 0x6f),
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(binding.key).unwrap_or('?').to_string(),
        _ => "?".to_string(),
    };
    parts.push(key);
    parts.join(" + ")
}

pub fn capture(key: eframe::egui::Key, modifiers: eframe::egui::Modifiers) -> Option<(u32, u32)> {
    let name = format!("{key:?}");
    let vk = if name.len() == 1 && name.as_bytes()[0].is_ascii_uppercase() {
        name.as_bytes()[0] as u32
    } else if let Some(digit) = name.strip_prefix("Num") {
        if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
            digit.as_bytes()[0] as u32
        } else {
            return None;
        }
    } else if let Some(function) = name.strip_prefix('F') {
        let n = function.parse::<u32>().ok()?;
        if !(1..=24).contains(&n) {
            return None;
        }
        0x6f + n
    } else {
        match key {
            eframe::egui::Key::ArrowUp => 0x26,
            eframe::egui::Key::ArrowDown => 0x28,
            _ => return None,
        }
    };
    let mut mods = 0;
    if modifiers.ctrl {
        mods |= MOD_CONTROL;
    }
    if modifiers.alt {
        mods |= MOD_ALT;
    }
    if modifiers.shift {
        mods |= MOD_SHIFT;
    }
    // A bare letter, digit, or arrow would steal ordinary game input globally.
    if mods == 0 && !(0x70..=0x87).contains(&vk) {
        return None;
    }
    Some((vk, mods))
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|root| PathBuf::from(root).join("OstrivSpeed").join("hotkeys.txt"))
}

fn parse_config(source: &str) -> Option<Vec<Binding>> {
    let rows: Vec<&str> = source.lines().collect();
    if rows.len() != ACTIONS.len() {
        return None;
    }
    let mut bindings = Vec::new();
    for (action, row) in ACTIONS.into_iter().zip(rows) {
        let (key, modifiers) = row.split_once(',')?;
        let key = key.parse::<u32>().ok()?;
        let modifiers = modifiers.parse::<u32>().ok()?;
        if !matches!(key, 0x26 | 0x28 | 0x30..=0x39 | 0x41..=0x5a | 0x70..=0x87)
            || modifiers & !(MOD_ALT | MOD_CONTROL | MOD_SHIFT) != 0
            || (modifiers == 0 && !(0x70..=0x87).contains(&key))
            || bindings
                .iter()
                .any(|b: &Binding| b.key == key && b.modifiers == modifiers)
        {
            return None;
        }
        bindings.push(Binding {
            action,
            key,
            modifiers,
        });
    }
    Some(bindings)
}

pub fn load() -> Vec<Binding> {
    config_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|source| parse_config(&source))
        .unwrap_or_else(defaults)
}

pub fn save(bindings: &[Binding]) -> Result<(), String> {
    let path = config_path().ok_or("LOCALAPPDATA is unavailable")?;
    let parent = path.parent().ok_or("Invalid hotkey config path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let text = bindings
        .iter()
        .map(|b| format!("{},{}\n", b.key, b.modifiers))
        .collect::<String>();
    fs::write(path, text).map_err(|e| e.to_string())
}

pub struct HotkeyService {
    pub status: String,
    #[cfg(windows)]
    receiver: std::sync::mpsc::Receiver<Action>,
    #[cfg(windows)]
    thread_id: u32,
    #[cfg(windows)]
    worker: Option<std::thread::JoinHandle<()>>,
}

impl HotkeyService {
    pub fn inactive() -> Self {
        #[cfg(windows)]
        {
            let (_sender, receiver) = std::sync::mpsc::channel();
            Self {
                status: "Hotkeys inactive until attach".into(),
                receiver,
                thread_id: 0,
                worker: None,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                status: "Global hotkeys require Windows".into(),
            }
        }
    }

    pub fn new(bindings: &[Binding]) -> Self {
        #[cfg(windows)]
        {
            let bindings = bindings.to_vec();
            let (action_sender, receiver) = std::sync::mpsc::channel();
            let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                    MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
                };
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, WM_HOTKEY,
                };
                let mut message: MSG = unsafe { std::mem::zeroed() };
                // SAFETY: create the worker message queue before sharing its thread id.
                unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE) };
                let thread_id =
                    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
                let mut registered = Vec::new();
                let mut failed = Vec::new();
                for (index, binding) in bindings.iter().enumerate() {
                    let id = 100 + index as i32;
                    // SAFETY: null HWND registers a thread hotkey with a unique id.
                    if unsafe {
                        RegisterHotKey(
                            std::ptr::null_mut(),
                            id,
                            binding.modifiers | MOD_NOREPEAT,
                            binding.key,
                        )
                    } != 0
                    {
                        registered.push(id);
                    } else {
                        failed.push(index);
                    }
                }
                let _ = ready_sender.send((thread_id, registered.len(), failed));
                loop {
                    // SAFETY: message is valid for this worker's queue.
                    let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
                    if result <= 0 {
                        break;
                    }
                    if message.message == WM_HOTKEY {
                        let index = message.wParam as usize;
                        if let Some(binding) = index.checked_sub(100).and_then(|i| bindings.get(i))
                            && action_sender.send(binding.action).is_err()
                        {
                            break;
                        }
                    }
                }
                for id in registered {
                    // SAFETY: unregister only this worker's own hotkeys.
                    unsafe { UnregisterHotKey(std::ptr::null_mut(), id) };
                }
            });
            let (thread_id, count, failed) = ready_receiver.recv().unwrap_or((0, 0, vec![]));
            let status = if failed.is_empty() {
                format!("{count} hotkeys active")
            } else {
                format!(
                    "{count} hotkeys active; unavailable: {}",
                    failed
                        .iter()
                        .map(|i| (i + 1).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Self {
                status,
                receiver,
                thread_id,
                worker: Some(worker),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = bindings;
            Self {
                status: "Global hotkeys require Windows".into(),
            }
        }
    }

    pub fn poll(&self) -> Vec<Action> {
        #[cfg(windows)]
        {
            self.receiver.try_iter().collect()
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }

    pub fn stop(&mut self) {
        #[cfg(windows)]
        if let Some(worker) = self.worker.take() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            // SAFETY: the id belongs to the worker after its queue was initialized.
            if unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) } != 0 {
                let _ = worker.join();
            }
        }
        self.status = "Hotkeys inactive until attach".into();
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip_and_duplicate_rejection() {
        let source = defaults()
            .iter()
            .map(|b| format!("{},{}\n", b.key, b.modifiers))
            .collect::<String>();
        assert_eq!(parse_config(&source), Some(defaults()));
        let duplicate = source.replacen("49,3", "48,3", 1);
        assert!(parse_config(&duplicate).is_none());
    }

    #[test]
    fn capture_supports_function_and_modified_letter_keys() {
        use eframe::egui::{Key, Modifiers};
        assert_eq!(capture(Key::F2, Modifiers::NONE), Some((0x71, 0)));
        assert_eq!(
            capture(
                Key::F,
                Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                }
            ),
            Some((b'F' as u32, MOD_CONTROL | MOD_SHIFT))
        );
        assert_eq!(capture(Key::F, Modifiers::NONE), None);
    }
}
