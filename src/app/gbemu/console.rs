use std::{fs::File, io::Bytes};

use crate::{app::cartridge_info::CartridgeInfo, mappers::{Mapper, NoMBC, MBC1}};

pub struct GBConsole {
    //CPU Registers
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    flags: u8,
    stack_pointer: u16,
    program_counter: u16,

    //Cartridge ROM and RAM
    cartridge: Mapper,

    //Console RAM
    video_ram: Vec<[u8; 0x4000]>,
    video_ram_index: usize,
    working_ram: [u8; 0x2000],
    aux_working_ram: Vec<[u8; 0x4000]>,
    aux_working_ram_index: usize,
    high_ram: [u8; 0x39],
}

impl GBConsole {
    pub fn new(info: CartridgeInfo, file: Bytes<File>) -> Self {
        let cartridge: Mapper = match info.cartridge_type {
            0x00 => {
                //TODO: Figure out if any rom only games actually utilize external RAM and implement here
                let rom_bank = NoMBC::prepare_rom(file);
                Mapper::NoMBC(NoMBC::new(rom_bank, false))
            }
            0x01 | 0x02 | 0x03 => {
                let ram_bank_count = if info.cartridge_type == 0x01 {0} else {info.ram_banks as u8};
                let has_battery = info.cartridge_type == 0x03;
                let rom_banks = MBC1::prepare_rom(file, info.rom_banks as u8);
                Mapper::MBC1(MBC1::new(rom_banks, ram_bank_count, has_battery))
            }
            _ => panic!("Error: Unknown cartridge code: {}", info.cartridge_type)
        };

        let mut aux_working_ram = Vec::new();
        aux_working_ram.push([0; 0x4000]);

        let mut video_ram = Vec::new();
        video_ram.push([0; 0x4000]);

        Self {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            flags: 0b10000000,
            stack_pointer: 0xFFFE,
            program_counter: 0x0100,
            cartridge: cartridge,
            video_ram: video_ram,
            video_ram_index: 0,
            working_ram: [0; 0x2000],
            aux_working_ram: aux_working_ram,
            aux_working_ram_index: 0,
            high_ram: [0; 0x39],
        }
    }

    fn read(&self, address: u16) -> u8 {
        //Cartrige ROM
        if address < 0x8000 {
            self.cartridge.read(address)
        }
        //VRAM
        else if address < 0xA000 {
            self.video_ram[self.video_ram_index][(address - 0x8000) as usize]
        }
        //Cartrige RAM
        else if address < 0xC000 {
            self.cartridge.read(address)
        }
        //WRAM bank 0
        else if address < 0xD000 {
            self.working_ram[(address - 0xC000) as usize]
        }
        //WRAM bank 1-7
        else if address < 0xE000 {
            self.aux_working_ram[self.aux_working_ram_index][(address - 0xD000) as usize]
        }
        //Echo RAM (Use is prohibited by nintendo)
        else if address < 0xFE00 {
            //TODO: Properly impmlent Echo RAM
            panic!("ERROR: Echo RAM access prohibited");
        }
        //Object Attribute Memory
        else if address < 0xFEA0 {
            //TODO: Implement Object Attribute Memory
            0
        }
        //Not Usable (Use is prohibited by Nintendo)
        else if address < 0xFF00 {
            //TODO: Properly implement this address space (VERY low priority)
            panic!("ERROR: Prohibited Address Space")
        }
        //I/O Registers
        else if address < 0xFF80 {
            //TODO: Implement I/O Registers
            0
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize]
        }
        //Interrupt Enable Register
        else {
            //TODO: Implement Interrupt Enable register
            0
        }
    }

    fn read_16(&self, address: u16) -> u16 {
        if address == 0xFFFF {
            panic!("ERROR: Address out of bounds");
        }

        let lsb = self.read(address);
        let msb = self.read(address + 1);

        u16::from_le_bytes([lsb, msb])
    }

    fn write(&mut self, address: u16, value: u8) {
        //Cartrige ROM
        if address < 0x8000 {
            self.cartridge.write(address, value);
        }
        //VRAM
        else if address < 0xA000 {
            self.video_ram[self.video_ram_index][(address - 0x8000) as usize] = value;
        }
        //Cartrige RAM
        else if address < 0xC000 {
            self.cartridge.write(address, value);
        }
        //WRAM bank 0
        else if address < 0xD000 {
            self.working_ram[(address - 0xC000) as usize] = value;
        }
        //WRAM bank 1-7
        else if address < 0xE000 {
            self.aux_working_ram[self.aux_working_ram_index][(address - 0xD000) as usize] = value;
        }
        //Echo RAM (Use is prohibited by nintendo)
        else if address < 0xFE00 {
            //TODO: Properly impmlent Echo RAM
            panic!("ERROR: Echo RAM access prohibited");
        }
        //Object Attribute Memory
        else if address < 0xFEA0 {
            //TODO: Implement Object Access Memory
        }
        //Not Usable (Use is prohibited by Nintendo)
        else if address < 0xFF00 {
            //TODO: Properly implement this address space (VERY low priority)
            panic!("ERROR: Prohibited Address Space")
        }
        //I/O Registers
        else if address < 0xFF80 {
            //TODO: Implement I/O Registers
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize] = value;
        }
        //Interrupt Enable Register
        else {
            //TODO: Implement Interrupt Enable register
        }
    }
}