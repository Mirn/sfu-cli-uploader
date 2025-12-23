use std::env;
use std::time::{Duration, Instant};
use std::io::{self, Write};
use std::fs;
use std::fs::File;
use std::thread;
use std::process::ExitCode;

use serialport::SerialPort;

mod misc;
//use misc::spawn_stdin_channel;
//use misc::strip_trailing_newline;
use misc::tostr;
use misc::deserialize_bytes;
use misc::deserialize_u16_le;
use misc::deserialize_u32_le;

mod crc32;

mod packet;
use packet::packet_build;
use packet::PacketParser;
use packet::PacketParserExt;

mod cmdline;
use cmdline::parse_cmdline_from_env;

mod reset;
use reset::GpioResetStatus;
use reset::cp210x_gpio_reset;

use crate::crc32::crc32::crc32_sfu;


const SFU_CMD_ERASE_PART :u8 =   0xB3;
const SFU_CMD_INFO   :u8 = 0x97;
const SFU_CMD_ERASE  :u8 = 0xC5;
const SFU_CMD_WRITE  :u8 = 0x38;
const SFU_CMD_START  :u8 = 0x26;
const SFU_CMD_TIMEOUT:u8 = 0xAA;
const SFU_CMD_WRERROR:u8 = 0x55;
const SFU_CMD_HWRESET:u8 = 0x11;


const WR_BLOCK_SIZE:usize = 0x800;

#[derive(Debug, Clone)]
pub struct SfuInfo {
    pub device_id: [u8; 12],
    pub cpu_type: u32,
    pub flash_size_correct: u32,
    pub sfu_ver: u16,
    pub receive_size: u32,
    pub main_start_from: u32,
    pub main_run_from: u32,
    pub firmware_end_at: u32,
}

pub fn parse_sfu_info(body: &[u8], fw_len:u32) -> Option<SfuInfo> {
    if body.len() < 32 {
        return None;
    }

    let device_id      = deserialize_bytes::<12>(body, 0);
    let cpu_type       = deserialize_u32_le(body, 12);
    let flash_correct  = deserialize_u16_le(body, 16);
    let sfu_ver        = deserialize_u16_le(body, 18);
    let receive_size   = deserialize_u32_le(body, 20);
    let main_start     = deserialize_u32_le(body, 24);
    let main_run       = deserialize_u32_le(body, 28);

    Some(SfuInfo {
        device_id,
        cpu_type,
        flash_size_correct: (flash_correct as u32 * 1024),
        sfu_ver,
        receive_size,
        main_start_from: main_start,
        main_run_from: main_run,
        firmware_end_at: (main_start + fw_len),
    })
}

pub fn print_sfu_info_unwrap(info: &Option<SfuInfo>) {
    if let Some(info) = info  {
        print_sfu_info(info);
    } else {
        println!("SFU INFO: NONE");
    }
}

pub fn print_sfu_info(info: &SfuInfo) {
    println!("--- SFU INFO ---");
    print!("Device ID: [");
    for b in info.device_id {
        print!("{:02X} ", b);
    }
    println!("] {}", tostr(&info.device_id));

    println!("CPU Type:            0x{:08X}", info.cpu_type);
    println!("Flash Size Correct:  0x{:08x} {}", info.flash_size_correct, info.flash_size_correct);
    println!("SFU Version:         {:04X}", info.sfu_ver);
    println!("Receive Size:        {}", info.receive_size);
    println!("MAIN_START_FROM:     0x{:08X}", info.main_start_from);
    println!("MAIN_RUN_FROM:       0x{:08X}", info.main_run_from);
    println!("firmware end at:     0x{:08X}", info.firmware_end_at);
    println!("---------------------");
}

pub fn parse_erase_info(body: &[u8]) -> Option<i32> {
    if body.len() < 4 {
        return None;
    }
    let part_num       = deserialize_u32_le(body, 0) as i32;
    Some(part_num)
}

#[derive(Debug, Clone)]
pub struct WriteInfo {
    pub mcu_write_addr: u32,
    pub mcu_receive_count: u32,
}

pub fn parse_write_info(body: &[u8]) -> Option<WriteInfo> {
    if body.len() < 8 {
        return None;
    }

    let mcu_write_addr     = deserialize_u32_le(body, 0);
    let mcu_receive_count       = deserialize_u32_le(body, 4);

    Some(WriteInfo {
        mcu_write_addr: mcu_write_addr,
        mcu_receive_count: mcu_receive_count,
    })
}

#[derive(Debug, Clone)]
pub struct StartInfo {
    pub mcu_from: u32,
    pub mcu_count: u32,
    pub mcu_crc32: u32,
}

pub fn parse_start_info(body: &[u8]) -> Option<StartInfo> {
    if body.len() < 12 {
        return None;
    }

    let mcu_from  = deserialize_u32_le(body, 0);
    let mcu_count = deserialize_u32_le(body, 4);
    let mcu_crc32 = deserialize_u32_le(body, 8);

    Some(StartInfo {
        mcu_from: mcu_from,
        mcu_count: mcu_count,
        mcu_crc32: mcu_crc32,
    })
}
// fn send_blocking(port: &mut dyn serialport::SerialPort, packet: &mut PacketParser, cmd:u8, data: &[u8], timeout_ms: u32) -> Option<Vec<u8>> {
//     let _ = port.write(&packet_build(cmd, data));

//     let mut runtime = Instant::now();
//     let mut timeout:i32 = timeout_ms as i32;
//     while timeout > runtime.elapsed().as_millis() as i32 {
//         let mut serial_buf: Vec<u8> = vec![0; 0x10000];
//         match port.read(serial_buf.as_mut_slice()) {
//             Ok(t) => {
//                 let read = &serial_buf[..t];
//                 //println!("{:?}\t{:02X?}\t{}", runtime.elapsed(), read, tostr(read));
//                 packet.receive_data(read);
//                 while let Some(body) = packet.packets[cmd as usize].pop_front() {
//                     println!("{:?}\t{:2X}:\t{:02X?}\t{}", runtime.elapsed(), cmd, body.as_slice(), tostr(body.as_slice()));
//                     let info = parse_sfu_info(body.as_slice()).unwrap();
//                     print_sfu_info(&info);
//                     //return Some(body);
//                 };
//             },
//             Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
//             },
//             Err(e) => {
//                 panic!("{:?}", e);            
//             }
//         }

//         while let Some(str) = packet.logs.pop_front() {
//             println!("LOG: {str}");
//         }
//     }
//     return None;
// }

fn show_port_list() {
    println!("Available serial port list:");
    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports {
        println!("{}", p.port_name);
    }
}

fn write_all_serial(port: &mut dyn SerialPort, buf: &[u8]) -> io::Result<()> {
    let mut written = 0;
    while written < buf.len() {
        match port.write(&buf[written..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "serial write returned 0 bytes",
                ));
            }
            Ok(n) => {
                written += n;
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn send_write_command(timeline:&Instant, port: &mut dyn SerialPort, wr_addr_host:&mut u32, addr_shift:u32, fw_bin:&[u8]) {
    let start_index = (*wr_addr_host - addr_shift) as usize;
    let mut end_index = start_index + WR_BLOCK_SIZE;
    if end_index >= fw_bin.len() {
        end_index = fw_bin.len()
    }

    if start_index < end_index {
        println!("{}\tHOST: send SFU_CMD_WRITE with address {:08X} size: {}", timeline.elapsed().as_millis(), wr_addr_host, end_index-start_index);
        let cmd_write = packet_build(SFU_CMD_WRITE, &bytes![
            serialize_u32!(*wr_addr_host), 
            &fw_bin[start_index .. end_index]]);
        write_all_serial(&mut *port,&cmd_write);
        *wr_addr_host += WR_BLOCK_SIZE as u32;
    }
}

fn main() -> ExitCode {
    let mut timeline = Instant::now();
    let params = parse_cmdline_from_env();
    if params.is_none() {
        show_port_list();
        return ExitCode::from(255);
    }
    let params = params.unwrap();

    //let mut log = File::create("log.txt").expect("creation failed");
    
    if let Some(rst_seq) = params.reset {
        println!("{}\tHOST: reset begin", timeline.elapsed().as_millis());
        match cp210x_gpio_reset(&params.port, &rst_seq) {
            Ok(GpioResetStatus::UsedCp210x) => {println!("{}\tHOST: Reset done via CP210x GPIO latch", timeline.elapsed().as_millis());}
            Ok(GpioResetStatus::UsedDtrRts) => {println!("{}\tHOST: Reset done via DTR/RTS", timeline.elapsed().as_millis());}
            Err(e) => {
                eprintln!("{}\tHOST: GPIO reset error: {e}", timeline.elapsed().as_millis());
                return ExitCode::from(255);
            }
        }        
    }

    let mut fw_bin = vec![];
    let mut fw_crc32 = 0u32;
    if let Some(fname) = params.firmware_path {
        println!("{}\tHOST: load firmware file {}", timeline.elapsed().as_millis(), fname);
        fw_bin = if let Ok(mut bin) = fs::read(fname) {
            while bin.len() % 4 != 0 {
                bin.push(0xFF);
            }
            bin
        } else {
            eprintln!("{}\tHOST: load error", timeline.elapsed().as_millis());
            return ExitCode::from(255);
        };
        fw_crc32 = crc32_sfu(&fw_bin);
        println!("{}\tHOST: loaded {} (0x{:08X}) bytes, CRC32_SFU = 0x{:08X}", timeline.elapsed().as_millis(), fw_bin.len(), fw_bin.len(), fw_crc32);
    };
    

    println!("{}\tHOST: open port {}", timeline.elapsed().as_millis(), params.port);
    let mut port: Box<dyn SerialPort> = serialport::new(params.port, params.baud)
        .timeout(Duration::from_millis(1))
        .open().expect("Failed to open port");
    // thread::sleep(Duration::from_millis(32));
    // let _ = port.clear_break();
    // let _ = port.clear(serialport::ClearBuffer::All);
    // thread::sleep(Duration::from_millis(32));
    // let _ = port.clear(serialport::ClearBuffer::All);
    println!("{}\tHOST: open port done", timeline.elapsed().as_millis());

    let mut packet = PacketParser::new();

    let cmd_info = packet_build(SFU_CMD_INFO, &[]);
    let cmd_erase = packet_build(SFU_CMD_ERASE, &bytes![serialize_u32!(fw_bin.len() as u32)]);
    let cmd_start = packet_build(SFU_CMD_START, &bytes![serialize_u32!(fw_crc32)]);

    let mut stat_wr_addr_errors = 0;

    let mut self_close = Instant::now() + Duration::from_secs(10*60);;

    let mut timeout_info = Instant::now();
    let mut timeout_erase = Instant::now();
    let mut timeout_write = Instant::now();
    let mut timeout_start = Instant::now();

    let mut dev_info:Option<SfuInfo> = None;
    let mut erase_began = false;
    let mut erase_done = false;
    let mut write_done = false;

    let mut wr_addr_host = 0u32;    
    let mut last_mcu_addr = 0;
    let mut addr_shift = 0u32;

    let mut run = true;
    while run && (Instant::now() < self_close) {
        if dev_info.is_none() && Instant::now() > timeout_info {
            println!("{}\tHOST: send SFU_CMD_INFO", timeline.elapsed().as_millis());
            port.write(&cmd_info);
            timeout_info = Instant::now() + Duration::from_millis(1000);
        }

        if dev_info.is_some() && Instant::now() > timeout_erase && !erase_began && !erase_done {
            println!("{}\tHOST: send SFU_CMD_ERASE", timeline.elapsed().as_millis());
            port.write(&cmd_erase);
            timeout_erase = Instant::now() + Duration::from_millis(1000);
        }

        if erase_done && Instant::now() > timeout_write {
            // let mut start_index = (wr_addr_host - addr_shift) as usize;
            // let mut end_index = start_index + WR_BLOCK_SIZE;
            // if end_index >= fw_bin.len() {
            //     end_index = fw_bin.len()
            // }
            // if start_index < end_index {
            //     println!("{}\tHOST: send SFU_CMD_WRITE with address {:08X}", timeline.elapsed().as_millis(), wr_addr_host);
            //     let cmd_write = packet_build(SFU_CMD_WRITE, &bytes![
            //         serialize_u32!(wr_addr_host), 
            //         &fw_bin[start_index .. end_index]]);
            //     write_all_serial(&mut *port,&cmd_write);
            //     wr_addr_host += WR_BLOCK_SIZE as u32;
            // }
            send_write_command(&timeline, &mut *port, &mut wr_addr_host, addr_shift, &fw_bin);
            timeout_write = Instant::now() + Duration::from_millis(100);
        }
        if erase_done && write_done && Instant::now() > timeout_start {
            println!("{}\tHOST: send SFU_CMD_START", timeline.elapsed().as_millis());
            port.write(&cmd_start);
            timeout_start = Instant::now() + Duration::from_millis(1000);
        }

        let mut serial_buf: Vec<u8> = vec![0; 0x10000];
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => {
                let read = &serial_buf[..t];
                packet.receive_data(read);

                while let Some(body) = packet.packets[SFU_CMD_HWRESET as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_HWRESET was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_HWRESET, body.as_slice());
                    timeout_info = Instant::now() + Duration::from_millis(100);
                };

                while let Some(body) = packet.packets[SFU_CMD_INFO as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_INFO was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_INFO, body.as_slice());
                    dev_info = parse_sfu_info(body.as_slice(), fw_bin.len() as u32);
                    print_sfu_info_unwrap(&dev_info);
                    if let Some(info) = &dev_info {
                        wr_addr_host = info.main_start_from;
                        addr_shift = info.main_start_from;
                    };
                };

                while let Some(body) = packet.packets[SFU_CMD_ERASE_PART as usize].pop_front() {
                    let erase_part = parse_erase_info(body.as_slice());
                    println!("{}\tHOST: response to SFU_CMD_ERASE_PART was received: {:2X}:{:02X?}\t part = {}", timeline.elapsed().as_millis(), SFU_CMD_ERASE_PART, body.as_slice(), erase_part.unwrap_or(-1));
                    erase_began = true;
                };

                while let Some(body) = packet.packets[SFU_CMD_ERASE as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_ERASE was received: {:2X}:{:02X?} ERASE DONE", timeline.elapsed().as_millis(), SFU_CMD_ERASE, body.as_slice());
                    erase_done = true;
                };

                while let Some(body) = packet.packets[SFU_CMD_WRITE as usize].pop_front() {
                    let write_info = parse_write_info(body.as_slice());
                    let status = if let Some(info) = &write_info {
                        if last_mcu_addr == info.mcu_write_addr {
                            wr_addr_host = info.mcu_write_addr;
                            println!("{}\tHOST: Write address corrected at {:08X}", timeline.elapsed().as_millis(), wr_addr_host);
                            timeout_write = Instant::now() + Duration::from_millis(250);
                            stat_wr_addr_errors += 1;
                        } else {
                            timeout_write = Instant::now();
                        }
                        last_mcu_addr = info.mcu_write_addr;                        

                        let status = format!("mcu_addr: 0x{:08X}, mcu_used: {}", info.mcu_write_addr, info.mcu_receive_count);
                        println!("{}\tHOST: response to SFU_CMD_WRITE was received: {:2X}:{:02X?}\t{}", timeline.elapsed().as_millis(), SFU_CMD_WRITE, body.as_slice(), status);                        

                        if let Some(dev_info) = &dev_info {
                            if info.mcu_write_addr == dev_info.firmware_end_at {
                                write_done = true;                            
                                println!("{}\tHOST: ================ Write done =================", timeline.elapsed().as_millis());
                            }
                        }                        
                    } else {
                        run = false;
                        println!("{}\tHOST: response to SFU_CMD_WRITE was received but parse ERROR, unknow format!", timeline.elapsed().as_millis());
                    };                    
                };

                while let Some(body) = packet.packets[SFU_CMD_START as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_START was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_START, body.as_slice());
                    let start_info = parse_start_info(body.as_slice());
                    if let Some(info) = &start_info {
                        println!("{}\tHOST: firmware from   : 0x{:08X}", timeline.elapsed().as_millis(), info.mcu_from);
                        println!("{}\tHOST: firmware size   : 0x{:08X} ({})", timeline.elapsed().as_millis(), info.mcu_count, info.mcu_count);
                        println!("{}\tHOST: firmware end at : 0x{:08X}", timeline.elapsed().as_millis(), info.mcu_from + info.mcu_count);
                        println!("{}\tHOST: mcu actual crc32: 0x{:08X}", timeline.elapsed().as_millis(), info.mcu_crc32);
                        println!("{}\tHOST: crc32 from file : 0x{:08X}", timeline.elapsed().as_millis(), fw_crc32);
                        self_close = Instant::now() + Duration::from_millis(500);
                    };
                }
                while let Some(body) = packet.packets[SFU_CMD_WRERROR as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_WRERROR was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_WRERROR, body.as_slice());
                    run = false;
                }

                while let Some(body) = packet.packets[SFU_CMD_TIMEOUT as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_TIMEOUT was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_TIMEOUT, body.as_slice());
                    run = false;
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
            },
            Err(e) => {
                panic!("{:?}", e);            
            }
        }

        while let Some(str) = packet.logs.pop_front() {
            println!("{}\tDEVICE: {str}", timeline.elapsed().as_millis());
        }

        if timeline.elapsed().as_millis() > 100000 {
            break;
        }
    }

    // thread::sleep(Duration::from_millis(64));
    // let _ = send_blocking(&mut *port, &mut packet, SFU_CMD_INFO, &[], 3000);

    println!("");
    packet.print_stats();
    println!("");
    let mut stat_unhandled_cmds = 0;
    for cmd_code in &packet.packets {
        stat_unhandled_cmds += cmd_code.len();
    }
    println!("stat_wr_addr_errors: {stat_wr_addr_errors}");
    println!("stat_unhandled_cmds: {stat_unhandled_cmds}");
    return ExitCode::from(0);
}
