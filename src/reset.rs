#![allow(non_camel_case_types)]
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResetSequence {
    pub quantum_ms: u32,
    pub mask: u16,
    pub values: Vec<u16>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GpioResetStatus {
    /// Sequence executed via CP210x latch GPIO.
    UsedCp210x,
    /// Sequence executed via classic serial DTR/RTS pins.
    UsedDtrRts,
}

#[derive(Debug)]
pub enum GpioResetError {
    /// Port could not be opened (device not found or access denied).
    PortOpenFailed(String),
    /// Port opened, but something went wrong during the sequence
    /// (CP210x latch or DTR/RTS control).
    SequenceFailed(String),
}

impl fmt::Display for GpioResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpioResetError::PortOpenFailed(msg) => write!(f, "port open failed: {msg}"),
            GpioResetError::SequenceFailed(msg) => write!(f, "reset sequence failed: {msg}"),
        }
    }
}

impl std::error::Error for GpioResetError {}

pub fn cp210x_gpio_reset(port: &str, rst_seq: &ResetSequence) -> Result<GpioResetStatus, GpioResetError> {
    #[cfg(windows)]
    {
        platform::cp210x_gpio_reset_windows(port, rst_seq)
    }
    #[cfg(unix)]
    {
        platform::cp210x_gpio_reset_unix(port, rst_seq)
    }
}

#[cfg(windows)]
mod platform {
    use super::{GpioResetError, GpioResetStatus, ResetSequence};
    //use std::error::Error;
    use std::ffi::c_void;
    use std::fs::OpenOptions;
    use std::os::windows::io::AsRawHandle;
    use std::time::Duration;
    use std::thread::sleep;

    use libloading::Library;

    type HANDLE = *mut c_void;
    type CP210x_STATUS = i32;
    const CP210X_SUCCESS: CP210x_STATUS = 0;
    // const CP210X_INVALID_HANDLE: CP210x_STATUS = 1;
    // const CP210X_DEVICE_IO_FAILED: CP210x_STATUS = 2;
    // const CP210X_FUNCTION_NOT_SUPPORTED: CP210x_STATUS = 3;
    // const CP210X_INVALID_PARAMETER: CP210x_STATUS = 4;
    // const CP210X_DEVICE_NOT_FOUND: CP210x_STATUS = 5;
    // const CP210X_INVALID_ACCESS_TYPE: CP210x_STATUS = 6;

    // EscapeCommFunction codes (WinAPI)
    const SETRTS: u32 = 3;
    const CLRRTS: u32 = 4;
    const SETDTR: u32 = 5;
    const CLRDTR: u32 = 6;

    unsafe extern "system" {
        fn EscapeCommFunction(h_file: HANDLE, dw_func: u32) -> i32;
    }

    pub fn cp210x_gpio_reset_windows(port: &str, rst_seq: &ResetSequence) -> Result<GpioResetStatus, GpioResetError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(port)
            .map_err(|e| GpioResetError::PortOpenFailed(format!("{port}: {e}")))?;

        let handle = file.as_raw_handle() as HANDLE;
        let quantum = Duration::from_millis(rst_seq.quantum_ms as u64);

        match run_sequence_cp210x(handle, rst_seq, quantum) {
            Ok(true) => {
                return Ok(GpioResetStatus::UsedCp210x);
            }
            Ok(false) => {
                if let Err(msg) = run_sequence_dtr_rts(handle, rst_seq, quantum) {
                    return Err(GpioResetError::SequenceFailed(msg));
                } else {
                    return  Ok(GpioResetStatus::UsedDtrRts)
                }
            }
            Err(msg) => {
                eprintln!("CP210x latch path failed: {msg}");
                return Err(GpioResetError::SequenceFailed(msg));
            }
        }
        // file closed by Drop.        
    }

    fn run_sequence_cp210x(handle: HANDLE, rst_seq: &ResetSequence, quantum: Duration) -> Result<bool, String> {
        let lib_name = "CP210xRuntime.dll";
        unsafe {
            let lib = match Library::new(lib_name) {
                Ok(l) => l,
                Err(_) => {
                    return Err(format!("ERROR: {lib_name} not found"));
                }
            };

            type FnWrite = unsafe extern "system" fn(HANDLE, u16, u16) -> CP210x_STATUS;

            let func: libloading::Symbol<FnWrite> = match lib.get(b"CP210xRT_WriteLatch\0") {
                Ok(f) => f,
                Err(_) => {
                    return Err(format!("ERROR: CP210xRT_WriteLatch not found inside {lib_name}"));
                }
            };

            for &val in &rst_seq.values {
                let status = func(handle, rst_seq.mask, val);
                if status != CP210X_SUCCESS {
                    eprintln!("CP210xRT_WriteLatch error {status}");
                    return Ok(false);
                }
                if quantum.as_millis() > 0 {
                    sleep(quantum);
                }
            }
        }

        Ok(true)
    }

    fn run_sequence_dtr_rts(handle: HANDLE, rst_seq: &ResetSequence, quantum: Duration) -> Result<(), String> {
        for &val in &rst_seq.values {
            let dtr = (val & 0x0001) != 0;
            let rts = (val & 0x0002) != 0;

            unsafe {
                let res_dtr = if dtr {
                    EscapeCommFunction(handle, SETDTR)
                } else {
                    EscapeCommFunction(handle, CLRDTR)
                };
                if res_dtr == 0 {
                    return Err("EscapeCommFunction for DTR failed".into());
                }

                let res_rts = if rts {
                    EscapeCommFunction(handle, SETRTS)
                } else {
                    EscapeCommFunction(handle, CLRRTS)
                };
                if res_rts == 0 {
                    return Err("EscapeCommFunction for RTS failed".into());
                }
            }

            if quantum.as_millis() > 0 {
                sleep(quantum);
            }
        }

        Ok(())
    }
}

#[cfg(unix)]
mod platform {
    use super::{GpioResetError, GpioResetStatus, ResetSequence};
    use std::fs::OpenOptions;
    use std::io;
    use std::os::unix::io::AsRawFd;
    use std::time::Duration;
    use std::thread::sleep;

    use libc::{self, c_int};
    use rusb::{Context, DeviceHandle, UsbContext, Direction, RequestType, Recipient};

    const VID: u16 = 0x10c4; // Silicon Labs
    const PID: u16 = 0xea60; // CP2102N (UART bridge)
    const REQ_VENDOR_SPEC: u8 = 0xFF;
    const WRITE_LATCH: u16 = 0x37E1; // wValue, wIndex = (MASK << 8) | STATE

    pub fn cp210x_gpio_reset_unix(
        port: &str,
        rst_seq: &ResetSequence,
    ) -> Result<GpioResetStatus, GpioResetError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(port)
            .map_err(|e| GpioResetError::PortOpenFailed(format!("{port}: {e}")))?;

        let fd = file.as_raw_fd();
        let quantum = Duration::from_millis(rst_seq.quantum_ms as u64);

        match run_sequence_cp210x_usb(rst_seq, quantum) {
            Ok(true) => {
                return Ok(GpioResetStatus::UsedCp210x);
            }
            Ok(false) => {
            }
            Err(msg) => {
                eprintln!("CP2102N USB latch path failed: {msg}");                
            }
        }

        // 2) Fallback to DTR/RTS 
        if let Err(e) = run_sequence_dtr_rts_unix(fd, rst_seq, quantum) {
            return Err(GpioResetError::SequenceFailed(format!(
                "DTR/RTS ioctl failed on {port}: {e}"
            )));
        }

        Ok(GpioResetStatus::UsedDtrRts)
        // file closed by drop Drop, with fd 
    }

    fn run_sequence_cp210x_usb(
        rst_seq: &ResetSequence,
        quantum: Duration,
    ) -> Result<bool, String> {
        let ctx = Context::new()
            .map_err(|e| format!("rusb Context::new failed: {e}"))?;
        let devices = ctx
            .devices()
            .map_err(|e| format!("rusb devices() failed: {e}"))?;
    
        let mut handle_opt: Option<DeviceHandle<Context>> = None;

        for dev in devices.iter() {
            let desc = dev
                .device_descriptor()
                .map_err(|e| format!("device_descriptor() failed: {e}"))?;
            if desc.vendor_id() == VID && desc.product_id() == PID {
                let handle = dev
                    .open()
                    .map_err(|e| format!("cannot open CP2102N device: {e}"))?;
                handle_opt = Some(handle);
                break;
            }
        }

        let mut handle = match handle_opt {
            Some(h) => h,
            None => return Ok(false), 
        };

        let req_type =
            rusb::request_type(Direction::Out, RequestType::Vendor, Recipient::Device);

        for &val in &rst_seq.values {
            // host-level semantics: mask = какие биты обновлять, val = новое состояние
            let update_mask = rst_seq.mask & 0x00FF;
            let new_state_bits = val & rst_seq.mask & 0x00FF;

            // CP2102N semantics
            //  - low byte (state/update mask)  = update_mask
            //  - high byte (mask/new state)    = new_state_bits
            let windex: u16 = ((new_state_bits as u16) << 8) | (update_mask as u16);

            handle
                .write_control(
                    req_type,
                    REQ_VENDOR_SPEC,
                    WRITE_LATCH,
                    windex,
                    &[],
                    Duration::from_millis(200),
                )
                .map_err(|e| format!("write_control failed: {e}"))?;
        
            if quantum.as_millis() > 0 {
                sleep(quantum);
            }
        }

        Ok(true)
    }

    fn run_sequence_dtr_rts_unix(
        fd: i32,
        rst_seq: &ResetSequence,
        quantum: Duration,
    ) -> io::Result<()> {
        for &val in &rst_seq.values {
            let dtr = (val & 0x0001) != 0;
            let rts = (val & 0x0002) != 0;
            set_dtr_rts(fd, dtr, rts)?;

            if quantum.as_millis() > 0 {
                sleep(quantum);
            }
        }
        Ok(())
    }

    fn set_dtr_rts(fd: i32, dtr: bool, rts: bool) -> io::Result<()> {
        unsafe {
            let mut flags: c_int = 0;

            // Get current modem control flags.
            if libc::ioctl(fd, libc::TIOCMGET, &mut flags) == -1 {
                return Err(io::Error::last_os_error());
            }

            if dtr {
                flags |= libc::TIOCM_DTR;
            } else {
                flags &= !libc::TIOCM_DTR;
            }

            if rts {
                flags |= libc::TIOCM_RTS;
            } else {
                flags &= !libc::TIOCM_RTS;
            }

            if libc::ioctl(fd, libc::TIOCMSET, &flags) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}