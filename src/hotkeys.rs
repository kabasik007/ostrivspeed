//! Global Ctrl+Alt hotkeys for the trainer UI. This registers exact key chords
//! with Windows; it does not install a keyboard hook or change game input.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Preset(u32),
    Next,
    Previous,
}

pub const PRESETS: [u32; 7] = [1, 5, 12, 15, 20, 30, 50];

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
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            let (action_sender, receiver) = std::sync::mpsc::channel();
            let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_DOWN,
                    VK_UP,
                };
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, WM_HOTKEY,
                };
                let mut message: MSG = unsafe { std::mem::zeroed() };
                // SAFETY: This creates this worker's message queue before the thread id is shared.
                unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE) };
                let thread_id =
                    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
                let modifiers = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
                let mut registered = Vec::new();
                let mut failed = Vec::new();
                let bindings: [(i32, u32, Action); 10] = [
                    (100, b'0' as u32, Action::Preset(1)),
                    (101, b'1' as u32, Action::Preset(1)),
                    (102, b'2' as u32, Action::Preset(5)),
                    (103, b'3' as u32, Action::Preset(12)),
                    (104, b'4' as u32, Action::Preset(15)),
                    (105, b'5' as u32, Action::Preset(20)),
                    (106, b'6' as u32, Action::Preset(30)),
                    (107, b'7' as u32, Action::Preset(50)),
                    (108, VK_UP as u32, Action::Next),
                    (109, VK_DOWN as u32, Action::Previous),
                ];
                for (id, key, _) in bindings {
                    // SAFETY: HWND null registers a thread hotkey; ids are unique in this thread.
                    if unsafe { RegisterHotKey(std::ptr::null_mut(), id, modifiers, key) } != 0 {
                        registered.push(id);
                    } else {
                        failed.push(id);
                    }
                }
                let _ = ready_sender.send((thread_id, registered.len(), failed));
                loop {
                    // SAFETY: message is valid for this thread's message queue.
                    let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
                    if result <= 0 {
                        break;
                    }
                    if message.message == WM_HOTKEY
                        && let Some((_, _, action)) = bindings
                            .iter()
                            .find(|(id, _, _)| *id == message.wParam as i32)
                        && action_sender.send(*action).is_err()
                    {
                        break;
                    }
                }
                for id in registered {
                    // SAFETY: removes only this thread's own registrations.
                    unsafe { UnregisterHotKey(std::ptr::null_mut(), id) };
                }
            });
            let (thread_id, count, failed) = ready_receiver.recv().unwrap_or((0, 0, vec![]));
            let status = if failed.is_empty() {
                format!("{count} global shortcuts active")
            } else {
                format!("{count} shortcuts active; {} unavailable", failed.len())
            };
            Self {
                status,
                receiver,
                thread_id,
                worker: Some(worker),
            }
        }
        #[cfg(not(windows))]
        Self {
            status: "Global shortcuts require Windows".into(),
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
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            // SAFETY: thread_id came from the worker after its queue was created.
            let posted = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) } != 0;
            if posted && let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}
