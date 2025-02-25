use std::{fs::File, io::Bytes};

use serde::de::value;

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

const Z_ZERO_FLAG: u8 = 128;
const N_SUBTRACTION_FLAG: u8 = 64;
const H_HALF_CARRY_FLAG: u8 = 32;
const C_CARRY_FLAG: u8 = 16;
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

    fn flag_toggle(&mut self, condition: bool, flag: u8) {
        if condition {
            self.flags |= flag;
        }
        else {
            self.flags &= 0xFF ^ flag;
        }
    }

    fn execute_instruction(&mut self) -> u8 {
        let mut instruction_size = 1;
        let mut cycle_count = 4;
        let block = self.program_counter & 0o300;
        match block {
            0o000 => { //Block 0
                match self.program_counter & 0o007 {
                    0o002 => {
                        //LD [R16], a | LD a, [R16] | LD [HL+], a | ld a, [HL+] | ld [HL-], a | LD a, [HL-]
                        cycle_count = 8;
                        let address = match self.program_counter & 0o060 {
                            0o000 => u16::from_be_bytes([self.b, self.c]),
                            0o020 => u16::from_be_bytes([self.d, self.e]),
                            0o040 => {
                                let address_temp = u16::from_be_bytes([self.h, self.l]);
                                (self.h, self.l) = (address_temp + 1).to_be_bytes().into();
                                address_temp
                            }
                            0o060 => {
                                let address_temp = u16::from_be_bytes([self.h, self.l]);
                                (self.h, self.l) = (address_temp - 1).to_be_bytes().into();
                                address_temp
                            }
                            _ => panic!("ERROR: address octet out of bounds!")
                        };

                        if self.program_counter & 0o010 > 0 {
                            self.a = self.read(address);
                        }
                        else {
                            self.write(address, self.a);
                        }
                    }
                    0o003 => { //INC r16, INC SP, DEC r16, DEC SP
                        cycle_count = 8;
                        let incrementor = if self.program_counter & 0o010 == 0 {1} else {u16::MAX};
                        let mut is_sp = false;
                        let (register_high, register_low) = match self.program_counter & 060 {
                            0o000 => (&mut self.b, &mut self.c),
                            0o020 => (&mut self.d, &mut self.e),
                            0o040 => (&mut self.h, &mut self.l),
                            0o060 => {
                                is_sp = true;
                                (&mut self.b, &mut self.c) //<== throwaway value
                            },
                            _ => panic!("ERROR: register octet out of bounds!")
                        };

                        if !is_sp {
                            let value = u16::from_be_bytes([*register_high, *register_low]) + incrementor;
                            (*register_high, *register_low) = value.to_be_bytes().into();
                        }
                        else {
                            self.stack_pointer += incrementor;
                        }
                    }
                    0o004 | 0o005 => { //INC r8, INC [HL], DEC r8, DEC [HL]
                        let mut is_hl = false;
                        let incrementor = if self.program_counter & 007 == 0o004 {1} else {u8::MAX};
                        let register = match self.program_counter & 0o070 {
                            0o000 => &mut self.b,
                            0o010 => &mut self.c,
                            0o020 => &mut self.d,
                            0o030 => &mut self.e,
                            0o040 => &mut self.h,
                            0o050 => &mut self.l,
                            0o060 => {
                                is_hl = true;
                                cycle_count = 12;
                                &mut self.b //<== Throwaway value
                            }
                            0o070 => &mut self.a,
                            _ => panic!("ERROR: Register octet out of bounds!")
                        };

                        let register_before;
                        let register_after;
                        if !is_hl {
                            register_before = *register;
                            *register += incrementor;
                            register_after = *register;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            register_before = self.read(address);
                            let value = register_before + incrementor;
                            self.write(address, value);
                            register_after = value;
                        }
                        self.flag_toggle(register_after == 0, Z_ZERO_FLAG);
                        self.flag_toggle(incrementor == 1, N_SUBTRACTION_FLAG);

                        let half_carry_condition = if incrementor == 1 {
                            (register_before & 0b1111 > 0) && (register_after & 0b1111 == 0)
                        }
                        else {
                            (register_before & 0b1111 == 0) && (register_after & 0b1111 > 0)
                        };
                        self.flag_toggle(half_carry_condition, H_HALF_CARRY_FLAG);
                    }
                    0o006 => {
                        //LD r8, n8 | LD [HL], r8
                        instruction_size = 2;
                        cycle_count = 8;

                        let mut is_hl = false;
                        let value = self.read(self.program_counter + 1);
                        let register = match self.program_counter & 0o007 {
                            0o000 => &mut self.b,
                            0o001 => &mut self.c,
                            0o002 => &mut self.d,
                            0o003 => &mut self.e,
                            0o004 => &mut self.h,
                            0o005 => &mut self.l,
                            0o006 => {
                                is_hl = true;
                                cycle_count = 12;
                                &mut self.b //<== Throwaway value
                            }
                            0o007 => &mut self.a,
                            _ => panic!("ERROR: Register octet out of bounds!")
                        };

                        if !is_hl {
                            *register = value;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            self.write(address, value);
                        }
                    }
                    _ => panic!("ERROR: Column octet out of bounds!")
                }
            }

            0o100 => { //Block 1
                //LD r8, r8, | LD r8, [HL] | LD [HL], r8
                let source = match self.program_counter & 0o007 {
                    0o000 => self.b,
                    0o001 => self.c,
                    0o002 => self.d,
                    0o003 => self.e,
                    0o004 => self.h,
                    0o005 => self.l,
                    0o006 => {
                        cycle_count = 8;
                        self.read(u16::from_be_bytes([self.h, self.l]))
                    }
                    0o007 => self.a,
                    _ => panic!("ERROR: Source octet out of bounds!")
                };

                let mut is_hl = false;
                let destination = match self.program_counter & 0o070 {
                    0o000 => &mut self.b,
                    0o010 => &mut self.c,
                    0o020 => &mut self.d,
                    0o030 => &mut self.e,
                    0o040 => &mut self.h,
                    0o050 => &mut self.l,
                    0o060 => {
                        is_hl = true;
                        cycle_count = 8;
                        &mut self.b //<== throway value to get out of the match (Will not be used!)
                    }
                    0o070 => &mut self.a,
                    _ =>panic!("Error: Destination octet out of bounds!")
                };

                if !is_hl {
                    *destination = source;
                }
                else {
                    let address = u16::from_be_bytes([self.h, self.l]);
                    self.write(address, source);
                }
            }

            0o200 => { //Blok 2

            }

            0o300 => { //Block 3

            }
            _ => panic!("ERROR: Block octet out of bounds!")
        }

        self.program_counter += instruction_size;
        cycle_count
    }
}