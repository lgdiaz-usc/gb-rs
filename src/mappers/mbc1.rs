use std::{fs::File, io::Bytes};

pub struct MBC1 {
    rom_banks: Vec<[u8; 0x4000]>,
    aux_rom_bank_index: usize,
    ram_banks: Option<Vec<[u8; 0x1000]>>,
    ram_bank_index: usize,
    has_battery: bool,
    ram_enabled: bool,
}

impl MBC1 {
    pub fn new(rom_banks: Vec<[u8; 0x4000]>, ram_bank_count: u8, has_battery: bool) -> Self {
        let mut ram_banks = Vec::with_capacity(ram_bank_count as usize);
        for _ in 0..ram_bank_count {
            ram_banks.push([0; 0x1000]);
        }

        Self {
            rom_banks: rom_banks,
            aux_rom_bank_index: 1,
            ram_banks: Some(ram_banks),
            ram_bank_index: 0,
            has_battery: has_battery,
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
                    //TODO: add battery functionality
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