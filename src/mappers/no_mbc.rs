use std::{fs::File, io::{Bytes, Read}};

pub struct NoMBC {
    rom_bank: [u8; 0x8000],
    ram_bank: Option<[u8; 0x2000]>
}

impl NoMBC {
    pub fn new(rom_file_path: String, has_ram: bool) -> Self {
        let rom_file = File::open(rom_file_path).unwrap().bytes();
        let rom_bank = Self::prepare_rom(rom_file);
        Self {
            rom_bank: rom_bank,
            ram_bank: if has_ram {Some([0; 0x2000])} else {None}
        }
    }

    pub fn prepare_rom(mut file: Bytes<File>) -> [u8; 0x8000] {
        let mut rom_data = [0; 0x8000];
        let mut iter = 0..0x8000;
        while let Some(i) = iter.next() {
            rom_data[i] = match file.next() {
                Some(val) => val.expect("Invalid byte?"),
                None => {
                    panic!("Invalid rom size!")
                },
            };
        }
    
        rom_data
    }
}

impl super::Mapper for NoMBC {
    fn read(&self, address: u16) -> u8 {
        if self.ram_bank != None && (address >= 0xA000 && address <= 0xBFFF) {
            self.ram_bank.unwrap()[(address - 0xA000) as usize];
        }

        if address > 0x8000 {
            panic!("Error: Address ${:x} out of bounds", address);
        }

        self.rom_bank[address as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        if self.ram_bank != None {
            self.ram_bank.unwrap()[address as usize] = value;
        }
    }
}