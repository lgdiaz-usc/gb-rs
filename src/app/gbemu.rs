use core::time;
use std::{fs::File, io::Read, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread, time::{Duration, Instant}};
use console::GBConsole;
use egui::Color32;

use super::cartridge_info::CartridgeInfo;

mod console;
mod ppu;
mod apu;

#[derive(Clone)]
pub struct GBEmu {
    pub rom_file_path: Arc<Mutex<Option<String>>>,
    pub rom_info: Arc<Mutex<Option<CartridgeInfo>>>,
    pub file_changed: Arc<AtomicBool>,
    pub screen_pixels: Arc<Mutex<Option<Vec<ScreenPixel>>>>,
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

        //TODO: Get and apply configs for keymaps
        let button_list = ButtonList::default(); 

        let mut console = GBConsole::new(info, rom_file, frame.clone(), button_list);

        let mut console_output = String::new();

        //Enforce framerate
        let clock_speed = 4.194304;
        let speed_factor = 1;
        //let fps = 4.0;
        let cycle_time = Duration::from_nanos((4000_f64 / clock_speed).round() as u64 * speed_factor);
        let mut next_cycle = Instant::now() + cycle_time;

        let mut frame_time = Instant::now();

        let mut cpu_delay = 255;
        '_Frame: loop {
            for _scanline in 0..154 {
                for _cycle in 0..114 {
                    //TODO: Implement some sort of periodic input checking so the Joypad Interrupt can work somewhat properly
                    if cpu_delay == 255 {
                        cpu_delay = console.handle_interrupt();
                        if !console.is_halted {
                            cpu_delay += console.get_instruction_delay();
                        }
                    }
                    cpu_delay -= 1;

                    if cpu_delay == 0 {
                        if console.interrupt_master_enable_flag == console::IMEState::Pending {
                            console.interrupt_master_enable_flag = console::IMEState::Enabled
                        }
                        console.execute_instruction();
                        cpu_delay -= 1;
                    }

                    console.update_timer();

                    for _dot in 0..4 {
                        if console.update_ppu() {
                            self.draw_new_frame(&frame, &console);
                            
                            if true {
                                println!("{:?}", Instant::now() - frame_time);
                            }
                            frame_time = Instant::now();
                        }

                        if let Some(serial_output) = console.check_serial() {
                            console_output.push((serial_output as char).to_ascii_uppercase());
                        }                        
                    }     

                    console.update_apu();  
                    
                    //Wait until next t_cycle
                    thread::sleep(next_cycle - Instant::now());
                    next_cycle += cycle_time;     
                }
            }

            print!("{}", console_output);
            console_output.clear();
        }
    }

    fn draw_new_frame(&self, frame: &egui::Context, console: &GBConsole) {
        let internal_screen = console.dump_screen();
        let mut pixel_colors = Vec::new();
        let bg_pallette = Self::dmg_pallette(console.dmg_bg_pallette);
        let obj0_pallette = Self::dmg_pallette(console.dmg_obj_pallette_0);
        let obj1_pallette = Self::dmg_pallette(console.dmg_obj_pallette_1);
    
        for i in 0..144 {
            let mut pixel_chunk = ScreenPixel { color: Color32::PLACEHOLDER, x: -1.0, y: -1.0, width: 0.0};
            for j in 0..160 {
                let pixel_color = match (*internal_screen)[i][j].palette {
                    None => bg_pallette[internal_screen[i][j].color as usize],
                    Some(pallette) => {
                        if pallette == 0 {
                            obj0_pallette[internal_screen[i][j].color as usize]
                        }
                        else {
                            obj1_pallette[internal_screen[i][j].color as usize]
                        }
                    }
                };
    
                if pixel_color != pixel_chunk.color {
                    pixel_colors.push(pixel_chunk.clone());
                    pixel_chunk.color = pixel_color;
                    pixel_chunk.width = 0.0;
                    pixel_chunk.x = j as f32;
                    pixel_chunk.y = i as f32;
                }
                pixel_chunk.width += 1.0;
            }
            if pixel_chunk.width > 0.0 {
                pixel_colors.push(pixel_chunk.clone());
            }
        }
    
        {
            let mut lock = self.screen_pixels.lock().unwrap();
            *lock = Some(pixel_colors);
            drop(lock);
        }
        frame.request_repaint();
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

#[derive(Clone)]
pub struct ScreenPixel {
    color: Color32,
    x: f32,
    y: f32,
    width: f32,
}

impl ScreenPixel {
    pub fn to_rect(&self, game_height: f32, game_width: f32, y_offset: f32, x_offset: f32) -> egui::epaint::RectShape {
        let pixel_width = game_width / 160.0;
        let pixel_height = game_height / 144.0;
    
        let min_x = x_offset + (pixel_width * self.x);
        let min_y = y_offset + (pixel_height * self.y);
    
        let max_x = min_x + (pixel_width * self.width);
        let max_y = min_y + pixel_height;
        
        egui::epaint::RectShape::new(
            egui::Rect {
                min: egui::Pos2::new(min_x, min_y),
                max: egui::Pos2::new(max_x, max_y)
            },
            egui::Rounding::ZERO,
            self.color,
            egui::Stroke::NONE
        )
    }
}

pub struct ButtonList {
    up: KeyType,
    down: KeyType,
    left: KeyType,
    right: KeyType,
    start: KeyType,
    select: KeyType,
    a: KeyType,
    b: KeyType,
}

impl Default for ButtonList {
    fn default() -> Self {
        Self { 
            up: KeyType::Key(egui::Key::ArrowUp), 
            down: KeyType::Key(egui::Key::ArrowDown), 
            left: KeyType::Key(egui::Key::ArrowLeft), 
            right: KeyType::Key(egui::Key::ArrowRight), 
            start: KeyType::Key(egui::Key::Enter), 
            select: KeyType::Modifier(egui::Modifiers::SHIFT), 
            a: KeyType::Key(egui::Key::Z), 
            b: KeyType::Key(egui::Key::X) 
        }
    }
}

enum KeyType {
    Key(egui::Key),
    Modifier(egui::Modifiers),
}

impl KeyType {
    pub fn get_state(&self, ctx: &egui::Context) -> bool {
        match self {
            Self::Key(key) => ctx.input(|x| x.key_down(*key)),
            Self::Modifier(modifier) => ctx.input(|x| x.modifiers.matches_logically(*modifier))
        }
    }
}

