# SFU CLI Uploader (RP2040/STM32F4/STM32F7/STM32H7)

A command-line firmware uploader for **RP2040/STM32F4/STM32F7/STM32H7** devices using a custom high-speed UART protocol and a dedicated bootloader (SFU).
The tool is designed for **developers**, production flashing, and automated workflows — not for end users.

---

## Key Features

- High-speed UART firmware upload (no USB MSC, no SWD required)
- Custom streaming protocol with CRC32 verification
- Upload during flash erase (pipelined write) to minimize total time
- Robust handling of packet loss, resynchronization, and partial writes
- Supports GPIO-based reset via CP210x or classic RS-232 control lines
- Human-readable device logs mixed with binary protocol data
- Fully scriptable CLI (exit codes, no GUI, no interactive prompts)

---

## Performance

Typical flashing time for a **~1 MB firmware image**:

| Method                         | Time (approx.) |
|--------------------------------|----------------|
| **SFU UART uploader (this tool)** | **~15 seconds** |
| SWD + debug probe              | ~40 seconds    |
| USB Mass Storage boot mode     | Significantly slower |

In practice, this uploader is **several times faster** than standard RP2040 flashing methods, especially in production or iteration-heavy development workflows.

This was achieved by optimizing the update protocol for modern UART USB chips such as CP2102n

---

## How It Works (Short)

- The device runs a custom SFU bootloader
- Firmware is transferred over UART in fixed-size blocks
- The MCU buffers incoming data while flash erase is in progress
- Data is written sequentially and verified with CRC32
- The device rejects out-of-order or corrupted blocks
- Final CRC check confirms the full image before execution

---

## Usage

```text
Usage:
  sfu-cli-uploader [options] <firmware_file>

Options:
  -p, --port <PORT>        Serial port (COMx, /dev/ttyUSBx)
  -s, --speed <BAUD>       UART speed (default 921600)
  -si, --init-speed <BAUD> Initial speed before switching
  -sm, --main-speed <BAUD> Upload speed

  --info-only              Query device info only
  --erase-only             Erase flash only
  --no-prewrite            Disable upload during erase
  --version                Print tool / device version

  -r, --reset <T> <MASK> <VAL...>
      GPIO-based reset sequence

Example:

sfu-cli-uploader -p COM5 -si 1000000 -sm 2000000 firmware.bin
```

## Build

The project is a standard Rust CLI application.

```
cargo build --release
```

No external build steps, no custom toolchains, no vendor SDKs required.

## Requirements

 - RP2040 device with SFU bootloader installed
 - UART connection (USB-UART adapter or on-board USB-UART bridge)
 - Optional: CP210x for GPIO-based reset control

## Notes
 - This tool prioritizes speed, determinism, and automation
 - Panic (exit code 101) is treated as a fatal host-side or hardware error
 - Designed to be embedded into shell scripts, CI, and production flashing pipelines

## Related Projects

- **https://github.com/Mirn/rp2040-SFU**  
  RP2040 SFU UART bootloader implementing the device-side of this protocol.

- **https://github.com/Mirn/Boot_F4_fast_uart**  
  High-speed SFU UART bootloader for STM32F4 microcontrollers.

- **https://github.com/Mirn/Boot_F745_SFU**  
  SFU-style SFU UART bootloader for STM32F745 devices.

- **https://github.com/Mirn/Boot_STM32H743_SFU_fast_uart**  
  Fast UART SFU bootloader for STM32H743 MCUs.
