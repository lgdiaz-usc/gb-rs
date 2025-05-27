use std::{fs::{File, OpenOptions}, io::{BufWriter, Bytes, Read}, sync::mpsc::{channel, Sender}};

pub struct MBC2 {
    rom_banks: Vec<[u8; 0x4000]>,
    aux_rom_bank_index: usize,
    ram: [u8; 512],
    save_sender: Option<Sender<(u8, u64)>>,
    ram_enabled: bool,
}

impl MBC2 {
    pub fn new(rom_bank_count: usize, has_battery: bool, rom_file_path: String) -> Self {
        let mut save_sender_temp = None;
        let mut ram = [0; 512];
        
        if has_battery {
            let ram_file_path = super::mapper::rom_to_save(rom_file_path.clone());

            match File::open(ram_file_path.clone()) {
                Ok(mut file) => {
                    file.read(&mut ram).unwrap();
                }
                Err(e) => {
                    match e.kind() {
                        _ => panic!("{}", e),
                    }
                }
            }

            let save_file = BufWriter::new(OpenOptions::new()
                                                            .write(true)
                                                            .create(true)
                                                            .open(ram_file_path)
                                                            .unwrap());
            let (save_sender, save_receiver) = channel();
            super::mapper::write_thread(save_file, save_receiver);

            save_sender_temp = Some(save_sender);
        }

        let rom_file = File::open(rom_file_path).unwrap().bytes();
        let rom_banks = Self::prepare_rom(rom_file, rom_bank_count);

        Self {
            rom_banks: rom_banks,
            aux_rom_bank_index: 1,
            ram: ram,
            save_sender: save_sender_temp,
            ram_enabled: false
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
                let address = address & 0x1FF;
                self.ram[address as usize]
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
        if address <= 0x3FFF {
            if address & 0x100 == 0 {
                self.ram_enabled = value & 0xF == 0xA;
            }
            else {
                let mut temp_index = (value & 0b1111) as usize;
                if temp_index == 0 {
                    self.aux_rom_bank_index = 1;
                    return;
                }

                if temp_index > self.rom_banks.len() {
                    temp_index %= self.rom_banks.len();
                }

                self.aux_rom_bank_index = temp_index; 
            }
        }
        else if address >= 0xA000 && address <= 0xBFFF {
            if self.ram_enabled {
                let address = address & 0x1FF;
                let value = value & 0xF;

                self.ram[address as usize] = value;
                
                if let Some(sender) = &self.save_sender {
                    sender.send((value, address as u64)).unwrap();
                }
            }
        }
        else {
            panic!("Error:: Index out of bounds")
        }
    }

    pub fn prepare_rom(mut file: Bytes<File>, rom_bank_count: usize) -> Vec<[u8; 0x4000]> {
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