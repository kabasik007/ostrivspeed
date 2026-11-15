use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct GameProcess {
    pub pid: u32,
    pub path: PathBuf,
    pub module_base: usize,
}

pub trait ProcessBackend {
    fn find_game(&self) -> Result<Option<GameProcess>, String>;
}

pub struct NativeProcessBackend;

#[cfg(windows)]
impl ProcessBackend for NativeProcessBackend {
    fn find_game(&self) -> Result<Option<GameProcess>, String> {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };
        use windows_sys::Win32::System::ProcessStatus::K32EnumProcessModules;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, QueryFullProcessImageNameW,
        };

        // SAFETY: Win32 handles are checked before use and always closed. The string buffers
        // are sized to the API's declared length and no process memory is accessed.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(format!(
                    "Process snapshot: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut found = None;
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    if name.eq_ignore_ascii_case("ostriv.exe") {
                        let process = OpenProcess(
                            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                            0,
                            entry.th32ProcessID,
                        );
                        if !process.is_null() {
                            let mut path = [0u16; 32768];
                            let mut size = path.len() as u32;
                            if QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut size)
                                != 0
                            {
                                let mut module = std::ptr::null_mut();
                                let mut needed = 0u32;
                                let base = if K32EnumProcessModules(
                                    process,
                                    &mut module,
                                    std::mem::size_of_val(&module) as u32,
                                    &mut needed,
                                ) != 0
                                    && needed >= std::mem::size_of_val(&module) as u32
                                {
                                    module as usize
                                } else {
                                    0
                                };
                                found = Some(GameProcess {
                                    pid: entry.th32ProcessID,
                                    path: std::ffi::OsString::from_wide(&path[..size as usize])
                                        .into(),
                                    module_base: base,
                                });
                            }
                            CloseHandle(process);
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            Ok(found)
        }
    }
}

#[cfg(not(windows))]
impl ProcessBackend for NativeProcessBackend {
    fn find_game(&self) -> Result<Option<GameProcess>, String> {
        Ok(None)
    }
}
