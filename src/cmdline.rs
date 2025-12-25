use std::env;
use std::error::Error;

use super::reset::ResetSequence;

#[derive(Debug, Clone)]
pub struct CmdConfig {
    pub port: String,
    pub baud_init: u32,
    pub baud_main: u32,
    pub firmware_path: Option<String>,

    pub info_only: bool,
    pub erase_only: bool,
    
    pub no_prewrite: bool,

    pub reset: Option<ResetSequence>,
}

const DEFAULT_BAUD: u32 = 921600;

pub fn parse_cmdline_from_env() -> Option<CmdConfig> {
    let args: Vec<String> = env::args().collect();
    parse_cmdline(&args)
}

pub fn parse_cmdline(args: &[String]) -> Option<CmdConfig> {
    let mut port: Option<String> = None;
    let mut baud_init: Option<u32> = None;
    let mut baud_main: Option<u32> = None;
    let mut firmware_path: Option<String> = None;

    let mut info_only = false;
    let mut erase_only = false;
    let mut no_prewrite = false;

    let mut reset: Option<ResetSequence> = None;

    let mut i = 1; // skip program name

    while i < args.len() {
        let arg = &args[i];

        if arg == "-p" || arg == "--port" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: -p/--port requires an argument");
                print_usage();
                return None;
            }
            let raw_port = &args[i];
            port = Some(normalize_port(raw_port));
        } else if arg == "-s" || arg == "--speed" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: -s/--speed requires an argument");
                print_usage();
                return None;
            }
            match args[i].parse::<u32>() {
                Ok(v) => {
                    baud_init = Some(v);
                    baud_main = Some(v);
                }                    
                Err(e) => {
                    eprintln!("Error: invalid baud rate '{}': {e}", args[i]);
                    print_usage();
                    return None;
                }
            }
        }else if arg == "-si" || arg == "--init-speed" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: -si/--init-speed requires an argument");
                print_usage();
                return None;
            }
            match args[i].parse::<u32>() {
                Ok(v) => baud_init = Some(v),
                Err(e) => {
                    eprintln!("Error: invalid baud rate '{}': {e}", args[i]);
                    print_usage();
                    return None;
                }
            }
        }else if arg == "-sm" || arg == "--main-speed" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: -sm/--main-speed requires an argument");
                print_usage();
                return None;
            }
            match args[i].parse::<u32>() {
                Ok(v) => baud_main = Some(v),
                Err(e) => {
                    eprintln!("Error: invalid baud rate '{}': {e}", args[i]);
                    print_usage();
                    return None;
                }
            }
        } else if arg == "--info-only" {
            info_only = true;
        } else if arg == "--erase-only" {
            erase_only = true;
        } else if arg == "--no-prewrite" {
            no_prewrite = true;
        } else if arg == "-r" || arg == "--reset" {
            if reset.is_some() {
                eprintln!("Error: reset sequence specified more than once");
                print_usage();
                return None;
            }
            i += 1;
            if i >= args.len() {
                eprintln!("Error: -r/--reset requires at least 3 arguments (quantum, mask, values...)");
                print_usage();
                return None;
            }

            // First: quantum in ms (decimal)
            let quantum_str = &args[i];
            let quantum_ms: u32 = match quantum_str.parse() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: invalid reset quantum '{}': {e}", quantum_str);
                    print_usage();
                    return None;
                }
            };

            i += 1;
            if i >= args.len() {
                eprintln!("Error: -r/--reset requires mask and at least one value");
                print_usage();
                return None;
            }

            // Second: mask (binary or hex, default hex)
            let mask_str = &args[i];
            let mask_u32 = match parse_bin_or_hex(mask_str) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: invalid reset mask '{}': {e}", mask_str);
                    print_usage();
                    return None;
                }
            };
            if mask_u32 > 0xFFFF {
                eprintln!(
                    "Error: reset mask '{}' out of range (> 0xFFFF)",
                    mask_str
                );
                print_usage();
                return None;
            }
            let mask = mask_u32 as u16;

            // Remaining args until next option or end are values
            let mut values: Vec<u16> = Vec::new();
            i += 1;
            while i < args.len() {
                let s = &args[i];
                if s.starts_with('-') {
                    break;
                }
                let val_u32 = match parse_bin_or_hex(s) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Error: invalid reset GPIO value '{}': {e}", s);
                        print_usage();
                        return None;
                    }
                };
                if val_u32 > 0xFFFF {
                    eprintln!(
                        "Error: reset GPIO value '{}' out of range (> 0xFFFF)",
                        s
                    );
                    print_usage();
                    return None;
                }
                values.push(val_u32 as u16);
                i += 1;
            }

            if values.len() < 2 {
                eprintln!("Error: -r/--reset requires at least two GPIO values (after mask)");
                print_usage();
                return None;
            }

            reset = Some(ResetSequence {
                quantum_ms,
                mask,
                values,
            });

            // continue loop without i += 1 here, т.к. мы его уже сдвигали внутри
            continue;
        } else if arg.starts_with('-') {
            eprintln!("Error: unknown option '{}'", arg);
            print_usage();
            return None;
        } else {
            // Positional argument: firmware file path
            if firmware_path.is_some() {
                eprintln!("Error: multiple firmware file paths specified ('{}' and '{}')",
                          firmware_path.as_ref().unwrap(), arg);
                print_usage();
                return None;
            }
            firmware_path = Some(arg.clone());
        }

        i += 1;
    }

    // Check mandatory firmware file depending on context
    let special_mode = info_only || erase_only;
    if firmware_path.is_none() && !special_mode {
        eprintln!("Error: firmware file is required unless --info-only/--erase-only is specified");
        print_usage();
        return None;
    }

    // Check mandatory firmware file depending on context
    if port.is_none() {
        eprintln!("Error: serial port is required (with -p/--port)");
        print_usage();
        return None;
    }

    let port = port.unwrap();
    let baud_init = baud_init.unwrap_or(DEFAULT_BAUD);
    let baud_main = baud_main.unwrap_or(baud_init);

    Some(CmdConfig {
        port,
        baud_init,
        baud_main,
        firmware_path,
        info_only,
        erase_only,
        no_prewrite,
        reset,
    })
}

fn parse_bin_or_hex(s: &str) -> Result<u32, Box<dyn Error>> {
    let s = s.trim();

    if let Some(stripped) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        // Binary
        let mut value: u32 = 0;
        if stripped.is_empty() {
            return Err("empty binary literal".into());
        }
        for ch in stripped.chars() {
            value = match ch {
                '0' => value << 1,
                '1' => (value << 1) | 1,
                _ => {
                    return Err(format!("invalid binary digit '{}' in '{}'", ch, s).into());
                }
            };
        }
        Ok(value)
    } else {
        // Hex by default, allow optional 0x prefix
        let stripped = if let Some(x) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            x
        } else {
            s
        };
        if stripped.is_empty() {
            return Err("empty hex literal".into());
        }
        Ok(u32::from_str_radix(stripped, 16)?)
    }
}

fn normalize_port(raw: &str) -> String {
    #[cfg(windows)]
    {
        // If already in "\\.\COMx" form, keep as is.
        if raw.starts_with(r"\\.\") {
            return raw.to_string();
        }

        // If it looks like "COMx" without prefix, add "\\.\"
        let upper = raw.to_ascii_uppercase();
        if upper.starts_with("COM") && !upper.contains('\\') && !upper.contains('/') {
            return format!(r"\\.\{}", raw);
        }

        // Otherwise, assume user provided a full/valid path.
        raw.to_string()
    }

    #[cfg(unix)]
    {
        // If path already contains '/', treat it as full path.
        if raw.contains('/') {
            raw.to_string()
        } else {
            // If it's something like "ttyUSB0", "ttyACM0", etc., assume /dev/<name>.
            format!("/dev/{}", raw)
        }
    }
}

fn print_usage() {
    const GIT_HASH: &str = env!("GIT_HASH");
    const BUILD_TIME: &str = env!("BUILD_TIME");
    const BUILD_PROFILE: &str = env!("BUILD_PROFILE");

    eprintln!(
        r#"Usage:
  sfu-cli-uploader [options] <firmware_file>

Options:
  -p, --port <PORT>        Serial port name (e.g. COM5, /dev/ttyUSB0)
  -s, --speed <BAUD>       Baud rate (decimal) for booth speeds I/M, default {DEFAULT_BAUD} bod
  -si, --init-speed <BAUD> Baud rate (decimal) for Initialization,  default {DEFAULT_BAUD} bod
  -sm, --main-speed <BAUD> Baud rate (decimal) for Main uploading, default {DEFAULT_BAUD} bod

  --info-only             Query device info only, no firmware file required
  --erase-only            Erase only, no firmware file required
  --no-prewrite           Disabling sending data for writing while erasing is in progress

  -r, --reset <T> <MASK> <VAL> [VAL ...]
      T       - GPIO quantum time, decimal (e.g. 50 = 50 ms)
      MASK    - GPIO mask, binary (0b...) or hex (0x... or plain hex, default hex)
      VAL...  - at least two GPIO values, each in binary or hex (same rules)      

Examples:
  sfu-cli-uploader -p COM5 -s 1000000 firmware.bin
  sfu-cli-uploader --port /dev/ttyUSB0 --info-only
  sfu-cli-uploader -p COM3 -r 50 0x0003 0b01 0b10 0b00 --erase-only
  
  commit: {}
  build:  {} ({})
"#, GIT_HASH, BUILD_TIME, BUILD_PROFILE
    );
}
