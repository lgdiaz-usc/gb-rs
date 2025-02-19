use core::time;
use std::{fs::File, io::Read, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread};
use super::cartridge_info::{CartridgeInfo};

#[derive(Clone)]
pub struct GBEmu {
    pub rom_file_path: Arc<Mutex<Option<String>>>,
    pub rom_info: Arc<Mutex<Option<CartridgeInfo>>>,
    pub file_changed: Arc<AtomicBool>
}

impl Default for GBEmu {
    fn default() -> Self {
        Self {
            rom_file_path: Arc::new(Mutex::new(None)),
            rom_info: Arc::new(Mutex::new(None)),
            file_changed: Arc::new(AtomicBool::from(false))
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

    }
}

