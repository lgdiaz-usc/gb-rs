use std::{fs::File, io::{BufWriter, Bytes, Read}, sync::mpsc::{channel, Sender}};

pub struct MBC1 {
    rom_banks: Vec<[u8; 0x4000]>,
    aux_rom_bank_index: usize,
    ram_banks: Option<Vec<[u8; 0x2000]>>,
    ram_bank_index: usize,
    _has_battery: bool,
    save_sender: Option<Sender<(u8, u64)>>,
    ram_enabled: bool,
}

impl MBC1 {
    pub fn new(rom_banks: Vec<[u8; 0x4000]>, ram_bank_count: u8, has_battery: bool, rom_file_path: String) -> Self {
        let mut save_sender_temp = None;
        let ram_banks;
        if ram_bank_count == 0 {
            ram_banks = None;
        }
        else {
            let mut ram_bank_vec = Vec::with_capacity(ram_bank_count as usize);
            
            let mut fill_with_0s = || { 
                for _ in 0..ram_bank_count {
                    ram_bank_vec.push([0; 0x2000]);
                }
            };

            if has_battery {
                let ram_file_path = super::mapper::rom_to_save(rom_file_path);

                match File::open(ram_file_path.clone()) {
                    Ok(mut file) => {
                        for _ in 0..ram_bank_count {
                            let mut ram_bank = [0; 0x2000];
                            file.read(&mut ram_bank).unwrap();
                            ram_bank_vec.push(ram_bank);
                        }
                    }
                    Err(e) => {
                        match e.kind() {
                            std::io::ErrorKind::NotFound => fill_with_0s(),
                            _ => panic!("{}", e),
                        }
                    }
                }

                let save_file = BufWriter::new(File::create(ram_file_path).unwrap());
                let (save_sender, save_receiver) = channel();
                super::mapper::write_thread(save_file, save_receiver);

                save_sender_temp = Some(save_sender);
            }
            else {
                fill_with_0s();
            }

            ram_banks = Some(ram_bank_vec);
        }

        Self {
            rom_banks: rom_banks,
            aux_rom_bank_index: 1,
            ram_banks: ram_banks,
            ram_bank_index: 0,
            _has_battery: has_battery,
            save_sender: save_sender_temp,
            ram_enabled: true
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address <= 0x3FFF {
            self.rom_banks[0][address as usize]
        }
        else if address <= 0x7FFF {
            self.rom_banks[self.aux_rom_bank_index][(address - 0x4000) as usize]
        }
        else if address >= 0xA000 && address <= 0xBFFF {
            if self.ram_enabled {
                match &self.ram_banks {
                    Some(ram_banks) => ram_banks[self.ram_bank_index][(address - 0xA000) as usize],
                    None => 0xFF //I'm not sure what happens when you try to read ram without having it, so I'm having it act like disabled ram
                }
            }
            else {
                0xFF
            }
        }
        else {
            panic!("Error: index out of bounds!");
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address <= 0x1FFF {
            self.ram_enabled = value & 0xF == 0xA;
        }
        else if address <= 0x3FFF {
            let mut temp_index = (value & 0b11111) as usize;
            if temp_index == 0 {
                self.aux_rom_bank_index = 1;
            }

            if temp_index > self.rom_banks.len() {
                temp_index %= self.rom_banks.len();
            }

            self.aux_rom_bank_index = temp_index;
        }
        else if address <= 0x5FFF {
            self.ram_bank_index = (value & 0b11) as usize;
        }
        else if address <= 0x7FFF {
            return;
        }
        else if address >= 0xA000 && address <= 0xBFFF {
            if self.ram_enabled {
                if self.ram_banks != None {
                    self.ram_banks.as_mut().unwrap()[self.ram_bank_index][(address - 0xA000) as usize] = value;
                    
                    if let Some(sender) = &self.save_sender {
                        sender.send((value, translate_address(address, self.ram_bank_index))).unwrap();
                    }
                }
            }
        }
        else {
            panic!("Error:: Index out of bounds")
        }
    }

    pub fn prepare_rom(mut file: Bytes<File>, rom_bank_count: u8) -> Vec<[u8; 0x4000]> {
        let mut rom_data: Vec<[u8; 0x4000]> = Vec::new();
        
        for _ in 0..rom_bank_count {
            let mut rom_bank = [0; 0x4000];
            let mut iter = 0..0x4000;
            while let Some(i) = iter.next() {
                rom_bank[i] = match file.next() {
                    Some(val) => val.expect("Invalid byte?"),
                    None => {
                        panic!("Invalid rom size!")
                    },
                };
            }
    
            rom_data.push(rom_bank);
        }
    
        rom_data
    }
}

fn translate_address(gb_address: u16, ram_bank_index: usize) -> u64 {
    ((gb_address - 0xA000) as u64) + (ram_bank_index as u64 * 0x2000)
}