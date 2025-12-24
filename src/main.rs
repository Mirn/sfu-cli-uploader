//use std::env;
//use std::fs::File;
use std::time::{Duration, Instant};
use std::io::{self};
use std::fs;
use std::thread::{self, sleep};
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
const SFU_CMD_SPEED  :u8 = 0x4B;
const SFU_CMD_TIMEOUT:u8 = 0xAA;
const SFU_CMD_WRERROR:u8 = 0x55;
const SFU_CMD_HWRESET:u8 = 0x11;

const WR_BLOCK_SIZE:usize = 0x800; //must be a multiple of 256

#[derive(Debug, Clone)]
pub struct SfuInfo {
    pub device_id: [u8; 12],
    pub cpu_type: u32,
    pub flash_size_correct: u32,
    pub sfu_ver: u16,
    pub receive_size: usize,
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
    let receive_size   = deserialize_u32_le(body, 20) as usize;
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

pub fn parse_erase_info(body: &[u8]) -> Option<i32> {
    if body.len() < 4 {
        return None;
    }
    let part_num = deserialize_u32_le(body, 0) as i32;
    Some(part_num)
}

#[derive(Debug, Clone)]
pub struct SpeedChangeInfo {
    pub old_bod: u32,
    pub new_bod: u32,
}

pub enum SpeedInfo {
    GET(u32),
    CHANGE(SpeedChangeInfo),
}

pub fn parse_speed_info(body: &[u8]) -> Option<SpeedInfo> {
    if body.len() == 4 {
        let bod     = deserialize_u32_le(body, 0);
        return Some(SpeedInfo::GET(bod));
    } else if body.len() == 8 {
        let old_bod     = deserialize_u32_le(body, 0);
        let new_bod     = deserialize_u32_le(body, 4);
        return Some(SpeedInfo::CHANGE(SpeedChangeInfo{old_bod:old_bod, new_bod:new_bod}));
    } else {
        return None;
    }
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
    let mcu_receive_count  = deserialize_u32_le(body, 4);

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

fn show_port_list() {
    println!("Available serial port list:");
    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports {
        println!("{}", p.port_name);
    }
}

fn write_all_serial(port: &mut dyn SerialPort, buf: &[u8]) -> io::Result<()> {
    let mut written = 0;
    let mut deadline = Instant::now() + Duration::from_millis(500);

    while written < buf.len() {
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "serial write stalled"));
        }

        match port.write(&buf[written..]) {
            Ok(0) => {
                thread::yield_now();
            }
            Ok(n) => {
                written += n;
                deadline = Instant::now() + Duration::from_millis(500);
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut
                       || e.kind() == io::ErrorKind::WouldBlock
                       || e.kind() == io::ErrorKind::Interrupted => {
                thread::yield_now();
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn send_write_command(timeline:&Instant, port: &mut dyn SerialPort, wr_addr_host:&mut u32, addr_shift:u32, fw_bin:&[u8], inflight_bytes_estimate:&mut usize) -> io::Result<()> {
    let start_index = (*wr_addr_host - addr_shift) as usize;
    let mut end_index = start_index + WR_BLOCK_SIZE;
    if end_index >= fw_bin.len() {
        end_index = fw_bin.len()
    }

    if start_index < end_index {
        println!("{}\tHOST: send SFU_CMD_WRITE with address {:08X} size: {} used: {}", timeline.elapsed().as_millis(), wr_addr_host, end_index-start_index, inflight_bytes_estimate);
        let cmd_write = packet_build(SFU_CMD_WRITE, &bytes![
            serialize_u32!(*wr_addr_host), 
            &fw_bin[start_index .. end_index]]);
        *wr_addr_host += (end_index - start_index) as u32;
        *inflight_bytes_estimate += cmd_write.len();
        return write_all_serial(&mut *port,&cmd_write);
    } else {
        return Ok(());
    }
}

const RESULT_SUCCESS:u8 = 0;
const RESULT_PARAM_ERROR:u8 = 2;
const RESULT_FW_LOAD_ERROR:u8 = 3;
const RESULT_RESET_ERROR:u8 = 4;
const RESULT_HOST_TIMEOUT_ERROR:u8 = 5;
const RESULT_DEVICE_TIMEOUT_ERROR:u8 = 10;
const RESULT_ERASE_ERROR:u8 = 11;
const RESULT_INFO_ERROR:u8 = 12;
const RESULT_PARSE_WRITE_ERROR:u8 = 13;
const RESULT_DEVICE_WRITE_ERROR:u8 = 14;
const RESULT_SPEED_ERROR:u8 = 15;

fn main() -> ExitCode {
    let timeline = Instant::now();
    let params = parse_cmdline_from_env();
    if params.is_none() {
        show_port_list();
        return ExitCode::from(RESULT_PARAM_ERROR);
    }
    let params = params.unwrap();

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
            return ExitCode::from(RESULT_FW_LOAD_ERROR);
        };
        fw_crc32 = crc32_sfu(&fw_bin);
        println!("{}\tHOST: loaded {} (0x{:08X}) bytes, CRC32_SFU = 0x{:08X}", timeline.elapsed().as_millis(), fw_bin.len(), fw_bin.len(), fw_crc32);
    };
    
    if let Some(rst_seq) = params.reset {
        println!("{}\tHOST: reset begin", timeline.elapsed().as_millis());
        match cp210x_gpio_reset(&params.port, &rst_seq) {
            Ok(GpioResetStatus::UsedCp210x) => {println!("{}\tHOST: Reset done via CP210x GPIO latch", timeline.elapsed().as_millis());}
            Ok(GpioResetStatus::UsedDtrRts) => {println!("{}\tHOST: Reset done via DTR/RTS", timeline.elapsed().as_millis());}
            Err(e) => {
                eprintln!("{}\tHOST: GPIO reset error: {e}", timeline.elapsed().as_millis());
                return ExitCode::from(RESULT_RESET_ERROR);
            }
        }        
    }

    println!("{}\tHOST: open port {}", timeline.elapsed().as_millis(), params.port);
    let mut port: Box<dyn SerialPort> = serialport::new(params.port, params.baud_init)
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
    let cmd_speed_get =  packet_build(SFU_CMD_SPEED, &[]);
    let cmd_speed_set =  packet_build(SFU_CMD_SPEED, &bytes![serialize_u32!(params.baud_main)]);

    let mut stat_write_resend_errors = 0;

    let mut self_close = Instant::now() + Duration::from_secs(2*60);

    let mut timeout_info = Instant::now();
    let mut timeout_erase = Instant::now();
    let mut timeout_write = Instant::now();
    let mut timeout_start = Instant::now();
    let mut timeout_speed_get = Instant::now();
    let mut timeout_speed_set = Instant::now();

    let mut resend_timeout = Duration::from_millis(250);

    let mut dev_info:Option<SfuInfo> = None;
    let mut erase_began = false;
    let mut erase_done = false;
    let mut write_done = false;
    let mut start_done = false;
    let mut speed_get_done = params.baud_main == params.baud_init;
    let mut speed_set_done = speed_get_done;

    let mut speed_get_attempts = 4;

    let mut wr_addr_host = 0u32;    
    let mut last_mcu_addr = 0;
    let mut addr_shift = 0u32;

    let mut inflight_bytes_estimate = 0;
    let mut inflight_bytes_limit = 0x10000;
    let mut write_actual_size = WR_BLOCK_SIZE;

    let mut write_bulk_size = 0;
    let     write_bulk_limit = 0x8000; //TODO: fix it, read device extra info for example

    let mut serial_buf: Vec<u8> = vec![0; 0x10000];

    let mut result = RESULT_HOST_TIMEOUT_ERROR;
    let mut run = true;
    while run && (Instant::now() < self_close) {
        if dev_info.is_none() && Instant::now() > timeout_info {
            println!("{}\tHOST: send SFU_CMD_INFO", timeline.elapsed().as_millis());
            write_all_serial(&mut *port, &cmd_info).expect("Write ERROR");
            timeout_info = Instant::now() + Duration::from_millis(1000);
        }

        if dev_info.is_some() && !erase_done && !erase_began && !write_done && Instant::now() > timeout_speed_get && !speed_get_done {
            if speed_get_attempts > 0 {
                println!("{}\tHOST: send SFU_CMD_SPEED(get)", timeline.elapsed().as_millis());
                write_all_serial(&mut *port, &cmd_speed_get).expect("Write ERROR");
                timeout_speed_get = Instant::now() + Duration::from_millis(300);
                speed_get_attempts -= 1;
            } else {
                speed_get_done = true;
                speed_set_done = true;
            }
        }

        if dev_info.is_some() && !erase_done && !erase_began && !write_done && Instant::now() > timeout_speed_set && speed_get_done && !speed_set_done {
            println!("{}\tHOST: send SFU_CMD_SPEED(SET)", timeline.elapsed().as_millis());
            write_all_serial(&mut *port, &cmd_speed_set).expect("Write ERROR");
            timeout_speed_set = Instant::now() + Duration::from_millis(1000);
        }

        if dev_info.is_some() && Instant::now() > timeout_erase && !erase_began && !erase_done && !write_done && speed_set_done && speed_get_done {
            println!("{}\tHOST: send SFU_CMD_ERASE", timeline.elapsed().as_millis());
            if params.erase_only {
                if let Some(info) = &dev_info {
                    let cmd_erase = packet_build(SFU_CMD_ERASE, &bytes![serialize_u32!(info.flash_size_correct)]);
                    write_all_serial(&mut *port, &cmd_erase).expect("Write ERROR");
                } else {
                    println!("{}\tERROR: There is no device info about flash erase size", timeline.elapsed().as_millis());
                    result = RESULT_ERASE_ERROR;
                    run = false;
                }
            } else {
                write_all_serial(&mut *port, &cmd_erase).expect("Write ERROR");
            }            
            timeout_erase = Instant::now() + Duration::from_millis(1000);
        }

        if Instant::now() > timeout_write && ((erase_began && !params.no_prewrite) || erase_done) &&  !write_done && speed_set_done && speed_get_done {
            if ((inflight_bytes_estimate + write_actual_size*2) < inflight_bytes_limit) && 
                ((write_bulk_size + write_actual_size*2) < write_bulk_limit) 
            {
                let size_before = inflight_bytes_estimate;
                send_write_command(&timeline, &mut *port, &mut wr_addr_host, addr_shift, &fw_bin, &mut inflight_bytes_estimate).expect("Write error!");
                if write_actual_size == WR_BLOCK_SIZE {
                    write_actual_size = inflight_bytes_estimate - size_before;
                }
                write_bulk_size += write_actual_size;
            } else {
                timeout_write = Instant::now() + Duration::from_millis(10);
            }
        }

        if erase_done && write_done && speed_set_done && speed_get_done && Instant::now() > timeout_start {
            println!("{}\tHOST: send SFU_CMD_START", timeline.elapsed().as_millis());
            write_all_serial(&mut *port, &cmd_start).expect("Write ERROR");
            timeout_start = Instant::now() + Duration::from_millis(1000);
        }

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
                    if let Some(info) = &dev_info {
                        println!("{}\tHOST: --- SFU INFO ---", timeline.elapsed().as_millis());
                        print!("{}\tHOST: Device ID: [", timeline.elapsed().as_millis());
                        for b in info.device_id {
                            print!("{:02X} ", b);
                        }
                        println!("] {}", tostr(&info.device_id));
                        println!("{}\tHOST: CPU Type:            0x{:08X}", timeline.elapsed().as_millis() , info.cpu_type);
                        println!("{}\tHOST: Flash Size Correct:  0x{:08x} {}", timeline.elapsed().as_millis(), info.flash_size_correct, info.flash_size_correct);
                        println!("{}\tHOST: SFU Version:         {:04X}", timeline.elapsed().as_millis(), info.sfu_ver);
                        println!("{}\tHOST: Receive Size:        {}", timeline.elapsed().as_millis(), info.receive_size);
                        println!("{}\tHOST: MAIN_START_FROM:     0x{:08X}", timeline.elapsed().as_millis(), info.main_start_from);
                        println!("{}\tHOST: MAIN_RUN_FROM:       0x{:08X}", timeline.elapsed().as_millis(), info.main_run_from);
                        println!("{}\tHOST: firmware end at:     0x{:08X}", timeline.elapsed().as_millis(), info.firmware_end_at);
                        println!("{}\tHOST: ---------------------", timeline.elapsed().as_millis());

                        wr_addr_host = info.main_start_from;
                        addr_shift = info.main_start_from;
                        inflight_bytes_limit = info.receive_size as usize;
                        run = !params.info_only;

                        if info.sfu_ver < 0x200 { //check not supported SFU_CMD_SPEED
                            speed_get_done = true;
                            speed_set_done = true;
                        }
                    } else {
                        println!("{}\tHOST: SFU INFO PARSING ERROR", timeline.elapsed().as_millis());                        
                        result = RESULT_INFO_ERROR;
                        run = false;
                    }
                };

                while let Some(body) = packet.packets[SFU_CMD_ERASE_PART as usize].pop_front() {
                    let erase_part = parse_erase_info(body.as_slice());
                    println!("{}\tHOST: response to SFU_CMD_ERASE_PART was received: {:2X}:{:02X?}\t part = {}", timeline.elapsed().as_millis(), SFU_CMD_ERASE_PART, body.as_slice(), erase_part.unwrap_or(-1));
                    erase_began = true;
                    write_bulk_size = 0;
                };

                while let Some(body) = packet.packets[SFU_CMD_ERASE as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_ERASE was received: {:2X}:{:02X?} ERASE DONE", timeline.elapsed().as_millis(), SFU_CMD_ERASE, body.as_slice());
                    erase_done = true;
                    run = !params.erase_only;
                };

                while let Some(body) = packet.packets[SFU_CMD_SPEED as usize].pop_front() {
                    let speed_info = parse_speed_info(body.as_slice());
                    if let Some(info) = &speed_info {
                        match info {
                            SpeedInfo::GET(v) => {
                                println!("{}\tHOST: response to SFU_CMD_SPEED was received: {:2X}:{:02X?}\t current BOD = {v}", timeline.elapsed().as_millis(), SFU_CMD_SPEED, body.as_slice());
                                timeout_speed_set = Instant::now();
                                speed_get_done = true;
                            }
                            SpeedInfo::CHANGE (v) => {
                                println!("{}\tHOST: response to SFU_CMD_SPEED was received: {:2X}:{:02X?}\t old_BOD = {}; New_BOD = {}", timeline.elapsed().as_millis(), SFU_CMD_SPEED, body.as_slice(), v.old_bod, v.new_bod);
                                let _ = port.clear(serialport::ClearBuffer::Input);
                                let _ = port.clear(serialport::ClearBuffer::Output);
                                port.set_baud_rate(v.new_bod).expect("ERROR: port.set_baud_rate");
                                sleep(Duration::from_millis(1));
                                let _ = port.clear(serialport::ClearBuffer::Input);
                                let _ = port.clear(serialport::ClearBuffer::Output);
                                println!("{}\tHOST: Baud rate changed to {} !", timeline.elapsed().as_millis(), v.new_bod);
                                speed_set_done = true;
                                speed_get_done = false;
                                timeout_speed_get = Instant::now() + Duration::from_millis(300);
                            }
                        };
                    } else {
                        println!("{}\tHOST: response to SFU_CMD_SPEED was received but parse ERROR, unknow format!", timeline.elapsed().as_millis());
                        result = RESULT_SPEED_ERROR;
                        run = false;
                    };                    
                }


                while let Some(body) = packet.packets[SFU_CMD_WRITE as usize].pop_front() {
                    let write_info = parse_write_info(body.as_slice());
                    if let Some(info) = &write_info {
                        write_bulk_size = 0;
                        if inflight_bytes_estimate < write_actual_size {
                            inflight_bytes_estimate = 0
                        } else {
                            inflight_bytes_estimate -= write_actual_size;
                        }
                        if last_mcu_addr == info.mcu_write_addr {
                            wr_addr_host = info.mcu_write_addr;
                            println!("{}\tHOST: Write address corrected at 0x{:08X}", timeline.elapsed().as_millis(), wr_addr_host);
                            timeout_write = Instant::now() + resend_timeout;
                            resend_timeout += Duration::from_millis(250);
                            stat_write_resend_errors += 1;
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
                        println!("{}\tHOST: response to SFU_CMD_WRITE was received but parse ERROR, unknow format!", timeline.elapsed().as_millis());
                        result = RESULT_PARSE_WRITE_ERROR;
                        run = false;
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
                        result = RESULT_SUCCESS;
                        start_done = true;
                    };
                }
                while let Some(body) = packet.packets[SFU_CMD_WRERROR as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_WRERROR was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_WRERROR, body.as_slice());
                    result = RESULT_DEVICE_WRITE_ERROR;
                    run = false;
                }

                while let Some(body) = packet.packets[SFU_CMD_TIMEOUT as usize].pop_front() {
                    println!("{}\tHOST: response to SFU_CMD_TIMEOUT was received: {:2X}:{:02X?}", timeline.elapsed().as_millis(), SFU_CMD_TIMEOUT, body.as_slice());
                    result = RESULT_DEVICE_TIMEOUT_ERROR;
                    run = false;
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                packet.tick(); //for log timeouts checking
            },
            Err(e) => {
                panic!("{:?}", e);            
            }
        }

        while let Some(str) = packet.logs.pop_front() {
            println!("{}\tDEVICE: {str}", timeline.elapsed().as_millis());
        }
    }

    // thread::sleep(Duration::from_millis(64));
    // let _ = send_blocking(&mut *port, &mut packet, SFU_CMD_INFO, &[], 3000);

    if (packet.stat_crc_error_packets != 0) ||
       (packet.stat_incomplete_bytes != 0) ||
       (packet.stat_other_error_packets != 0) ||
       (packet.stat_size_or_code_error_packets != 0) ||
       (packet.stat_log_bytes == 0) ||
       (packet.stat_log_lines == 0) ||
       (packet.stat_valid_packets == 0)
    {
        println!("");
        packet.print_stats();
        println!("");

        if packet.current_log_line.len() != 0 {
            println!("WARNING: non finished device log line: {}", packet.current_log_line);
        }
    }

    if result == RESULT_HOST_TIMEOUT_ERROR {
        println!("ERROR: HOST TIMEOUT!!!");
    } else if !(write_done && erase_done && start_done && (result == RESULT_SUCCESS)) {
        println!("WARNING: UPDATING NOT FINISHED!!!!");
    }

    let mut stat_unhandled_commands = 0;
    for cmd_code in &packet.packets {
        stat_unhandled_commands += cmd_code.len();
    }
    if (stat_unhandled_commands + stat_write_resend_errors) !=0 {
        println!("WARNING: stat_write_resend_errors: {stat_write_resend_errors}");
        println!("WARNING: stat_unhandled_commands:  {stat_unhandled_commands}");
    }
    return ExitCode::from(result);
}
