//! Version-bound, reversible access to the native target-speed double.
//! This profile is experimental: target and transition are observed, but
//! camera/audio independence above the stock 12x mode is not yet validated.

use crate::{fingerprint::Fingerprint, process::GameProcess};

const TARGET_SPEED_RVA: usize = 0x64F860;
const CURRENT_SPEED_RVA: usize = 0x64F160;
const WORLD_TIME_RVA: usize = 0x64F170;
const VALID_SPEEDS: &[u32] = &[1, 5, 12, 15, 20, 30, 50];
const EXPERIMENTAL_SPEEDS: &[u32] = &[60, 70, 100];
const ATTACHABLE_SPEEDS: &[u32] = &[1, 5, 12, 15, 20, 30, 50, 60, 70, 100];

pub struct SpeedController {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    address: usize,
    module_base: usize,
    applied: Option<f64>,
}

impl SpeedController {
    pub fn attach(game: &GameProcess, fingerprint: &Fingerprint) -> Result<Self, String> {
        if !fingerprint.recognized {
            return Err("Unrecognized executable: speed control refused".into());
        }
        if game.module_base == 0 {
            return Err("Unknown module base: speed control refused".into());
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_OPERATION,
                PROCESS_VM_READ, PROCESS_VM_WRITE,
            };
            // SAFETY: OpenProcess is confined to the observed PID. The handle is closed in Drop.
            let handle = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION
                        | PROCESS_VM_OPERATION
                        | PROCESS_VM_READ
                        | PROCESS_VM_WRITE,
                    0,
                    game.pid,
                )
            };
            if handle.is_null() {
                return Err(format!("OpenProcess: {}", std::io::Error::last_os_error()));
            }
            let controller = Self {
                handle,
                address: game.module_base + TARGET_SPEED_RVA,
                module_base: game.module_base,
                applied: None,
            };
            let live = controller.read()?;
            if !ATTACHABLE_SPEEDS.iter().any(|&speed| live == speed as f64) {
                return Err(format!(
                    "Unexpected native target speed {live}; attach refused"
                ));
            }
            Ok(controller)
        }
        #[cfg(not(windows))]
        Err("Windows only".into())
    }

    pub fn read(&self) -> Result<f64, String> {
        self.read_at(self.address)
    }

    pub fn current_speed(&self) -> Result<f64, String> {
        self.read_at(self.module_base + CURRENT_SPEED_RVA)
    }

    pub fn world_time(&self) -> Result<f64, String> {
        self.read_at(self.module_base + WORLD_TIME_RVA)
    }

    pub fn cpu_seconds(&self) -> Result<f64, String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::FILETIME;
            use windows_sys::Win32::System::Threading::GetProcessTimes;
            let mut created = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let mut exited = created;
            let mut kernel = created;
            let mut user = created;
            // SAFETY: all four FILETIME outputs are valid and the process handle is held.
            let ok = unsafe {
                GetProcessTimes(
                    self.handle,
                    &mut created,
                    &mut exited,
                    &mut kernel,
                    &mut user,
                )
            };
            if ok == 0 {
                return Err(format!(
                    "GetProcessTimes: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let ticks =
                |time: FILETIME| ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64;
            Ok((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
        }
        #[cfg(not(windows))]
        Err("Windows only".into())
    }

    fn read_at(&self, address: usize) -> Result<f64, String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
            let mut value = 0f64;
            let mut read = 0usize;
            // SAFETY: address is the measured module RVA for the hashed build; buffer is 8 bytes.
            let ok = unsafe {
                ReadProcessMemory(
                    self.handle,
                    address as *const _,
                    (&mut value as *mut f64).cast(),
                    8,
                    &mut read,
                )
            };
            if ok == 0 || read != 8 {
                return Err(format!(
                    "ReadProcessMemory: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(value)
        }
        #[cfg(not(windows))]
        {
            let _ = address;
            Err("Windows only".into())
        }
    }

    pub fn set(&mut self, speed: u32) -> Result<(), String> {
        if !VALID_SPEEDS.contains(&speed) {
            return Err("Unsupported speed preset".into());
        }
        self.set_checked(speed)
    }

    /// Only called by the GUI's bounded experiment. These values have no
    /// demonstrated stable throughput and must never be normal presets.
    pub fn set_experimental(&mut self, speed: u32) -> Result<(), String> {
        if !EXPERIMENTAL_SPEEDS.contains(&speed) {
            return Err("Unsupported experimental speed".into());
        }
        self.set_checked(speed)
    }

    fn set_checked(&mut self, speed: u32) -> Result<(), String> {
        let before = self.read()?;
        if !before.is_finite() || before < 0.0 || before > 100.0 {
            return Err(format!(
                "Unexpected live target speed {before}; write refused"
            ));
        }
        if let Some(last) = self.applied
            && before != last
        {
            return Err(format!(
                "Game changed target speed to {before}; refresh before writing"
            ));
        }
        self.write(speed as f64)?;
        if self.read()? != speed as f64 {
            self.write(before)?;
            return Err("Speed readback failed; previous value restored".into());
        }
        self.applied = Some(speed as f64);
        Ok(())
    }

    pub fn restore(&mut self) -> Result<(), String> {
        if let Some(last) = self.applied {
            if self.read()? != last {
                self.applied = None;
                return Err("Game changed target speed; automatic restore skipped".into());
            }
            self.write(1.0)?;
            if self.read()? != 1.0 {
                return Err("1x restore readback failed".into());
            }
            self.applied = None;
        }
        Ok(())
    }

    fn write(&self, value: f64) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
            let mut written = 0usize;
            // SAFETY: exact 8-byte variable, observed as f64 on the hashed build; never code bytes.
            let ok = unsafe {
                WriteProcessMemory(
                    self.handle,
                    self.address as *mut _,
                    (&value as *const f64).cast(),
                    8,
                    &mut written,
                )
            };
            if ok == 0 || written != 8 {
                return Err(format!(
                    "WriteProcessMemory: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = value;
            Err("Windows only".into())
        }
    }
}

impl Drop for SpeedController {
    fn drop(&mut self) {
        let _ = self.restore();
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_build_is_refused_before_process_access() {
        let game = GameProcess {
            pid: 1,
            path: std::path::PathBuf::new(),
            module_base: 0,
        };
        let fp = Fingerprint {
            sha256: "other".into(),
            architecture: "x86-64".into(),
            bytes: 0,
            recognized: false,
        };
        assert!(SpeedController::attach(&game, &fp).is_err());
    }
}
