use core::time;
use std::{fs::File, io::Read, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread};
use console::GBConsole;
use egui::Color32;

use super::cartridge_info::CartridgeInfo;

mod console;
mod ppu;

#[derive(Clone)]
pub struct GBEmu {
    pub rom_file_path: Arc<Mutex<Option<String>>>,
    pub rom_info: Arc<Mutex<Option<CartridgeInfo>>>,
    pub file_changed: Arc<AtomicBool>,
    pub screen_pixels: Arc<Mutex<Option<[[Color32; 160]; 144]>>>,
}

impl Default for GBEmu {
    fn default() -> Self {
        Self {
            rom_file_path: Arc::new(Mutex::new(None)),
            rom_info: Arc::new(Mutex::new(None)),
            file_changed: Arc::new(AtomicBool::from(false)),
            screen_pixels: Arc::new(Mutex::new(None)),
        }
    }
}

impl GBEmu {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let r: GBEmu = Default::default();

        let ctx = cc.egui_ctx.clone();
        let lock = r.clone();
        thread::spawn(move || {
            let mut temp_file_changed = lock.file_changed.load(Ordering::Relaxed);
            while !temp_file_changed {
                thread::sleep(time::Duration::from_millis(10));
                temp_file_changed = lock.file_changed.load(Ordering::Relaxed);
            }
            lock.file_changed.store(false, Ordering::Relaxed);

            lock.processor(ctx);
        });

        r
    }

    fn processor(&self, frame: egui::Context) {
        //Gets a local copyof the rom file path so we don't need to request access to it every time we read
        let current_file_path: String;
        {
            let lock = self.rom_file_path.lock().unwrap();
            current_file_path = lock.clone().unwrap();
            drop(lock);
        }

        //The first rom bank which also holds the cartridge header
        let mut cartridge_header: [u8; 0x14f] = [0; 0x14f];

        //Read the file for the first rom bank
        let mut rom_file = File::open(current_file_path.clone()).expect("ERROR: File not found!").bytes();
        let mut iter = 0..0x14f;
        while let Some(i) = iter.next() {
            cartridge_header[i] = match rom_file.next() {
                Some(val) => val.expect("Invalid byte?"),
                None => {
                    panic!("Invalid rom size!")
                },
            };
        }
        

        //Grabs metadata from the rom's cartrige header
        let info = CartridgeInfo::new(&cartridge_header[0x134..0x14f]);
        {
            let mut lock = self.rom_info.lock().unwrap();
            *lock = Some(info.clone());
            drop(lock);
        }

        drop(rom_file);
        let rom_file = File::open(current_file_path.clone()).expect("ERROR: File not found!").bytes();
        let mut console = GBConsole::new(info, rom_file);

        let mut console_output = String::new();

        let mut cpu_delay = 0;
        '_Frame: loop {
            for _scanline in 0..154 {
                for _dot in 0..456 {
                    cpu_delay += console.handle_interrupt();

                    if cpu_delay == 0 {
                        let interrupt_to_be_enabled = console.interrupt_master_enable_flag == console::IMEState::Pending;
                        cpu_delay = console.execute_instruction();
                        if interrupt_to_be_enabled && console.interrupt_master_enable_flag == console::IMEState::Pending {
                            console.interrupt_master_enable_flag = console::IMEState::Enabled
                        }
                    }

                    cpu_delay -= 1;
                    console.update_ppu();
                    if let Some(serial_output) = console.check_serial() {
                        console_output.push((serial_output as char).to_ascii_uppercase());
                    }
                }
            }

            print!("{}", console_output);
            console_output.clear();

            let internal_screen = console.dump_screen();
            let mut pixel_colors = [[Color32::WHITE; 160]; 144];
            let bg_pallette = Self::dmg_pallette(console.dmg_bg_pallette);
            let obj0_pallette = Self::dmg_pallette(console.dmg_obj_pallette_0);
            let obj1_pallette = Self::dmg_pallette(console.dmg_obj_pallette_1);

            for i in 0..144 {
                for j in 0..160 {
                    pixel_colors[i][j] = match (*internal_screen)[i][j].palette {
                        None => bg_pallette[internal_screen[i][j].color as usize],
                        Some(pallette) => {
                            if pallette == 0 {
                                obj0_pallette[internal_screen[i][j].color as usize]
                            }
                            else {
                                obj1_pallette[internal_screen[i][j].color as usize]
                            }
                        }
                    }
                }
            }

            {
                let mut lock = self.screen_pixels.lock().unwrap();
                *lock = Some(pixel_colors);
                drop(lock);
            }
            frame.request_repaint();
        }
    }

    fn dmg_pallette(console_pallette: u8) -> [Color32; 4] {
        let mut pallette = [Color32::WHITE; 4];

        for i in 0..4 {
            let color_code = (console_pallette >> (i * 2)) & 0b11;
            pallette[i] = match color_code {
                0b00 => Color32::WHITE,
                0b01 => Color32::LIGHT_GRAY,
                0b10 => Color32::DARK_GRAY,
                0b11 => Color32::BLACK,
                _ => panic!("Error: Unkown index!")
            }
        }

        pallette
    }
}

