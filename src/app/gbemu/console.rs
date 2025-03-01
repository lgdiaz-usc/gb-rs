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

    //Interrupt regissters
    interrupt_master_enable_flag: IMEState,
    interrupte_enable: u8,
    interrupt_flag: u8,
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
            interrupt_master_enable_flag: IMEState::Disabled,
            interrupte_enable: 0x00,
            interrupt_flag: 0xE1,
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
            match address {
                0xFF0F => self.interrupt_flag, //IF
                _ => panic!("ERROR: Unkown register at address ${:x}", address)
            }
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize]
        }
        //Interrupt Enable Register
        else {
            self.interrupte_enable
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
            let register = match address {
                0xFF0f => &mut self.interrupt_flag,
                _ => panic!("ERROR: Unknown register at address ${:x}", address)
            };

            *register = value;
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize] = value;
        }
        //Interrupt Enable Register
        else {
            self.interrupte_enable = value;
        }
    }

    fn write_16(&mut self, address: u16, value: u16) {
        if address == 0xFFFF {
            panic!("ERROR: Address out of bounds");
        }

        let (msb, lsb) = value.to_be_bytes().into();
        self.write(address, lsb);
        self.write(address + 1, msb);
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

        let opcode = self.read(self.program_counter);
        
        match opcode {
            //Block 0 one-offs
            0o000 => {}, //NOP
            0o010 => { //LD [n16], SP
                cycle_count = 20;
                instruction_size = 3;
                let address = self.read_16(self.program_counter + 1);
                self.write_16(address, self.stack_pointer);
            }
            0o020 => {
                //TODO: Implement STOP instruction
            }
            0o007 => { //RLCA
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                self.flag_toggle(self.a & 0x80 > 0, C_CARRY_FLAG);

                self.a <<= 1;
                if self.flags & C_CARRY_FLAG > 0 {
                    self.a += 0x01;
                }
            }
            0o017 => { //RRCA
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                self.flag_toggle(self.a & 0x01 > 0, C_CARRY_FLAG);

                self.a >>= 1;
                if self.flags & C_CARRY_FLAG > 0 {
                    self.a += 0x80;
                }
            }
            0o027 => { //RLA
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                let will_carry = self.a & 0x80 > 0;

                self.a <<= 1;
                if self.flags & C_CARRY_FLAG > 0 {
                    self.a += 0x01;
                }

                self.flag_toggle(will_carry, C_CARRY_FLAG);
            }
            0o037 => { //RRA
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                let will_carry = self.a & 0x01 > 0;

                self.a >>= 1;
                if self.flags & C_CARRY_FLAG > 0 {
                    self.a += 0x80;
                }

                self.flag_toggle(will_carry, C_CARRY_FLAG);
            }
            0o047 => { //DAA
                if self.flags & N_SUBTRACTION_FLAG > 0 {
                    if self.flags & H_HALF_CARRY_FLAG > 0 {
                        self.a += 0x6;
                    }
                    if self.flags & C_CARRY_FLAG > 0 {
                        self.a += 0x60;
                    }
                }
                else {
                    if (self.flags & C_CARRY_FLAG > 0) || (self.a > 0x99) {
                        self.a += 0x60;
                        self.flag_toggle(true, C_CARRY_FLAG);
                    }
                    if (self.flags & H_HALF_CARRY_FLAG > 1) || (self.a & 0xF > 0x9) {
                        self.a += 0x6;
                    }
                }

                self.flag_toggle(false, H_HALF_CARRY_FLAG);
                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
            }
            0o057 => { //CPL
                self.a = self.a ^ 0xFF;
                self.flag_toggle(true, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
            }
            0o067 => { //SCF
                self.flag_toggle(true, C_CARRY_FLAG);
                self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
            } 
            0o077 => { //CCF
                self.flag_toggle(self.flags & C_CARRY_FLAG == 0, C_CARRY_FLAG);
                self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
            }

            //Block 1 one-offs
            0o166 => {
                //TODO: Implement HALT instruction
            }

            //Block 3 one-offs
            0o311 => { //RET
                cycle_count = 16;
                self.program_counter = self.read_16(self.stack_pointer);
                self.stack_pointer += 2;
            }
            0o313 => { //PREFIX
                cycle_count = self.execute_prefixed_instruction();
            }
            0o331 => { //RETI
                cycle_count = 16;
                self.program_counter = self.read_16(self.stack_pointer);
                self.stack_pointer += 2;
                self.interrupt_master_enable_flag = IMEState::Enabled;
            }
            0o363 => { //DI
                self.interrupt_master_enable_flag = IMEState::Disabled;
            }
            0o373 => { //EI
                self.interrupt_master_enable_flag = IMEState::Pending;
            }

            //Invalid opcodes
            0o323 | 0o333 | 0o335 | 0o343 | 0o344 | 0o353 | 0o354 | 0o355 | 0o364 | 0o374 | 0o375 => panic!("ERROR: Invalid opcode!"),

            //The rest of the instructions are interpreted through pattern-matching. The above are instructions which break those patterns.
            _ => {
                let block = opcode & 0o300;
                match block {
                    0o000 => { //Block 0
                        match opcode & 0o007 {
                            0o000 => { //JR e8, JR cc, e8
                                let jump_condition = match opcode & 0o070 {
                                    0o030 => true,
                                    0o040 => self.flags & Z_ZERO_FLAG == 0,
                                    0o050 => self.flags & Z_ZERO_FLAG > 0,
                                    0o060 => self.flags & C_CARRY_FLAG == 0,
                                    0o070 => self.flags & C_CARRY_FLAG > 0,
                                    _ => panic!("ERROR: Condition opcode out of bounds!")
                                };

                                if jump_condition {
                                    cycle_count = 12;
                                    let jump_offset_u8 = self.read(self.program_counter + 1);
                                    if jump_offset_u8 >= 0x80 {
                                        instruction_size = u16::from_be_bytes([0xFF, jump_offset_u8]);
                                    }
                                    else {
                                        instruction_size = u16::from_be_bytes([0, jump_offset_u8]);
                                    }
                                }
                                else {
                                    instruction_size = 2;
                                    cycle_count = 8;
                                }
                            }
                            0o001 => { //LD r16, n16 | LD SP, n16 | ADD HL, r16 | ADD HL, SP
                                let is_add = opcode & 0o010 > 0;
                                let mut is_sp = false;
                            
                                let value;
                                if !is_add {
                                    value = self.read_16(self.program_counter + 1);
                                }
                                else {
                                    value = u16::from_be_bytes([self.h, self.l]);
                                }
                            
                                let (register_high, register_low) = match opcode & 0o060 {
                                    0o000 => (&mut self.b, &mut self.c),
                                    0o020 => (&mut self.d, &mut self.e),
                                    0o040 => (&mut self.h, &mut self.l),
                                    0o060 => {
                                        is_sp = true;
                                        (&mut self.b, &mut self.c) //<== Throwaway value
                                    }
                                    _ => panic!("ERROR: Register octet out of bounds!")
                                };
                            
                                if !is_add {
                                    cycle_count = 12;
                                    instruction_size = 3;
                                    if !is_sp {
                                        (*register_high, *register_low) = value.to_be_bytes().into();
                                    }
                                    else {
                                        self.stack_pointer = value;
                                    }
                                }
                                else {
                                    cycle_count = 8;
                                    if !is_sp {
                                        let register_value = u16::from_be_bytes([*register_high, *register_low]);
                                        (self.h, self.l) = (value + register_value).to_be_bytes().into();
                                    }
                                    else {
                                        (self.h, self.l) = (value + self.stack_pointer).to_be_bytes().into();
                                    }
                                
                                    self.flag_toggle(false, N_SUBTRACTION_FLAG);
                                    let hl_now = u16::from_be_bytes([self.h, self.l]);
                                    self.flag_toggle((value & 0x0FFF) > (hl_now & 0x0FFF), H_HALF_CARRY_FLAG);
                                    self.flag_toggle(value > hl_now, C_CARRY_FLAG);
                                }
                            }
                            0o002 => {
                                //LD [R16], a | LD a, [R16] | LD [HL+], a | ld a, [HL+] | ld [HL-], a | LD a, [HL-]
                                cycle_count = 8;
                                let address = match opcode & 0o060 {
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
                            
                                if opcode & 0o010 > 0 {
                                    self.a = self.read(address);
                                }
                                else {
                                    self.write(address, self.a);
                                }
                            }
                            0o003 => { //INC r16, INC SP, DEC r16, DEC SP
                                cycle_count = 8;
                                let incrementor = if opcode & 0o010 == 0 {1} else {u16::MAX};
                                let mut is_sp = false;
                                let (register_high, register_low) = match opcode & 060 {
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
                                let incrementor = if opcode & 007 == 0o004 {1} else {u8::MAX};
                                let register = match opcode & 0o070 {
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
                                let register = match opcode & 0o007 {
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
                        let source = match opcode & 0o007 {
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
                        let destination = match opcode & 0o070 {
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
                        let operand = match opcode & 0o007 {
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
                            _ => panic!("ERROR: Operand octet out of bounds!")
                        };

                        match opcode & 0o070 {
                            0o000 => { //ADD A, r8 | ADD A, [HL]
                                let temp_a = self.a;
                                self.a += operand;

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(false, N_SUBTRACTION_FLAG);
                                self.flag_toggle((temp_a & 0x0F) > (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                self.flag_toggle(temp_a > self.a, C_CARRY_FLAG);
                            }
                            0o010 => { //ADC A, r8 | ADC A, [HL]
                                let temp_a = self.a;
                                self.a += operand;
                                if self.flags & C_CARRY_FLAG > 0 {
                                    self.a += 1;
                                }

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(false, N_SUBTRACTION_FLAG);
                                self.flag_toggle((temp_a & 0x0F) > (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                self.flag_toggle(temp_a > self.a, C_CARRY_FLAG);
                            }
                            0o020 => { //SUB A, r8 | SUB A, [HL]
                                let temp_a = self.a;
                                self.a -= operand;

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                self.flag_toggle((temp_a & 0x0F) < (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                self.flag_toggle(temp_a < self.a, C_CARRY_FLAG);
                            }
                            0o030 => { //SBC A, r8 | SBC A, [HL]
                                let temp_a = self.a;
                                self.a -= operand;
                                if self.flags & C_CARRY_FLAG > 0 {
                                    self.a -= 1;
                                }

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                self.flag_toggle((temp_a & 0x0F) < (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                self.flag_toggle(temp_a < self.a, C_CARRY_FLAG);
                            }
                            0o040 => { //AND A, r8 | AND A [HL]
                                self.a &= operand;

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(true, H_HALF_CARRY_FLAG);
                                self.flag_toggle(false, N_SUBTRACTION_FLAG | C_CARRY_FLAG);
                            }
                            0o050 => { //XOR A, r8 | XOR A [HL]
                                self.a ^= operand;

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG | C_CARRY_FLAG);
                            }
                            0o060 => { //OR A, r8 | OR A [HL]
                                self.a |= operand;

                                self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG | C_CARRY_FLAG);
                            }
                            0o070 => { //CP A, r8 | CP A, [HL]
                                let comparison = self.a - operand;

                                self.flag_toggle(comparison == 0, Z_ZERO_FLAG);
                                self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                self.flag_toggle((self.a & 0x0F) < (comparison & 0x0F), H_HALF_CARRY_FLAG);
                                self.flag_toggle(self.a < comparison, C_CARRY_FLAG);
                            }
                            _ => panic!("ERROR: Operator octet out of bounds!")
                        }
                    }
                
                    0o300 => { //Block 3
                        match opcode & 0o007 {
                            0o000 => { //RET cc
                                let return_condition = match opcode & 0o030 {
                                    0o000 => self.flags & Z_ZERO_FLAG == 0,
                                    0o010 => self.flags & Z_ZERO_FLAG > 0,
                                    0o020 => self.flags & C_CARRY_FLAG == 0,
                                    0o030 => self.flags & C_CARRY_FLAG > 0,
                                    _ => panic!("ERROR: condition octet out of bounds!")
                                };

                                if return_condition {
                                    cycle_count = 20;
                                    self.program_counter = self.read_16(self.stack_pointer);
                                    self.stack_pointer += 2;
                                }
                                else {
                                    cycle_count = 8;
                                }
                            }
                            0o001 => { //POP r16 | POP AF
                                cycle_count = 12;

                                let popped_value = self.read_16(self.stack_pointer);
                                self.stack_pointer += 2;

                                let (register_high, register_low) = match opcode & 0o060 {
                                    0o000 => (&mut self.b, &mut self.c),
                                    0o020 => (&mut self.d, &mut self.e),
                                    0o040 => (&mut self.h, &mut self.l),
                                    0o060 => (&mut self.a, &mut self.flags),
                                    _ => panic!("ERROR: register octet out of bounds!")
                                };

                                (*register_high, *register_low) = u16::to_be_bytes(popped_value).into();
                            }
                            0o005 => { //PUSH r16 | PUSH AF
                                cycle_count = 16;
                                let pushed_value = u16::from_be_bytes(match opcode & 0o060 {
                                    0o000 => [self.b, self.c],
                                    0o020 => [self.d, self.e],
                                    0o040 => [self.h, self.l],
                                    0o060 => [self.a, self.flags],
                                    _ => panic!("ERROR: register octet out of bounds!")
                                });

                                self.stack_pointer -= 2;
                                self.write_16(self.stack_pointer, pushed_value);
                            }
                            0o006 => {
                                instruction_size = 2;
                                let operand = self.read(self.program_counter + 1);
                                
                                match opcode & 0o070 {
                                    0o000 => { //ADD A, n8
                                        let temp_a = self.a;
                                        self.a += operand;
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(false, N_SUBTRACTION_FLAG);
                                        self.flag_toggle((temp_a & 0x0F) > (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                        self.flag_toggle(temp_a > self.a, C_CARRY_FLAG);
                                    }
                                    0o010 => { //ADC A, n8
                                        let temp_a = self.a;
                                        self.a += operand;
                                        if self.flags & C_CARRY_FLAG > 0 {
                                            self.a += 1;
                                        }
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(false, N_SUBTRACTION_FLAG);
                                        self.flag_toggle((temp_a & 0x0F) > (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                        self.flag_toggle(temp_a > self.a, C_CARRY_FLAG);
                                    }
                                    0o020 => { //SUB A, n8
                                        let temp_a = self.a;
                                        self.a -= operand;
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                        self.flag_toggle((temp_a & 0x0F) < (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                        self.flag_toggle(temp_a < self.a, C_CARRY_FLAG);
                                    }
                                    0o030 => { //SBC A, n8
                                        let temp_a = self.a;
                                        self.a -= operand;
                                        if self.flags & C_CARRY_FLAG > 0 {
                                            self.a -= 1;
                                        }
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                        self.flag_toggle((temp_a & 0x0F) < (self.a & 0x0F), H_HALF_CARRY_FLAG);
                                        self.flag_toggle(temp_a < self.a, C_CARRY_FLAG);
                                    }
                                    0o040 => { //AND A, n8
                                        self.a &= operand;
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(true, H_HALF_CARRY_FLAG);
                                        self.flag_toggle(false, N_SUBTRACTION_FLAG | C_CARRY_FLAG);
                                    }
                                    0o050 => { //XOR A, n8
                                        self.a ^= operand;
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG | C_CARRY_FLAG);
                                    }
                                    0o060 => { //OR A, n8
                                        self.a |= operand;
                                    
                                        self.flag_toggle(self.a == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG | C_CARRY_FLAG);
                                    }
                                    0o070 => { //CP A, n8
                                        let comparison = self.a - operand;
                                    
                                        self.flag_toggle(comparison == 0, Z_ZERO_FLAG);
                                        self.flag_toggle(true, N_SUBTRACTION_FLAG);
                                        self.flag_toggle((self.a & 0x0F) < (comparison & 0x0F), H_HALF_CARRY_FLAG);
                                        self.flag_toggle(self.a < comparison, C_CARRY_FLAG);
                                    }
                                    _ => panic!("ERROR: Operator octet out of bounds!")
                                }
                            }
                            0o007 => { //RST vec
                                let jump_address_vector = match opcode & 070 {
                                    0o000 => 0x00,
                                    0o010 => 0x08,
                                    0o020 => 0x10,
                                    0o030 => 0x18,
                                    0o040 => 0x20,
                                    0o050 => 0x28,
                                    0o060 => 0x30,
                                    0o070 => 0x38,
                                    _ => panic!("ERROR: Vector octet out of bounds!")
                                };
                                
                                let return_address = self.read_16(self.program_counter + 1);
                                self.stack_pointer -= 2;
                                self.write_16(self.stack_pointer, return_address);

                                instruction_size = 0;
                                self.program_counter = self.read_16(jump_address_vector);
                            }
                            _ => panic!("ERROR: Column octet out of bounds!")
                        }
                    }
                    _ => panic!("ERROR: Block octet out of bounds!")
                }
            }
        }
        

        self.program_counter += instruction_size;
        cycle_count
    }

    fn execute_prefixed_instruction(&mut self) -> u8 {
        let opcode = self.read(self.program_counter + 1);
        let mut cycle_count = 8; 
        
        let mut is_hl = false;
        let operand = match opcode & 0o007 {
            0o000 => &mut self.b,
            0o001 => &mut self.c,
            0o002 => &mut self.d,
            0o003 => &mut self.e,
            0o004 => &mut self.h,
            0o005 => &mut self.l,
            0o006 => {
                is_hl = true;
                cycle_count = 16;
                &mut self.b //<===Throwaway value
            }
            0o007 => &mut self.a,
            _ => panic!("ERROR: Operand octet out of bounds!")
        };

        match opcode & 0o300 {
            0o000 => {
                match opcode & 0o070 {
                    0o000 => { //RLC r8 | RLC [HL]
                        let zero_condition ;
                        let carry_condition;

                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x80 > 0;

                            *operand <<= 1;
                            if carry_condition {
                                *operand += 1;
                            }
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);

                            zero_condition = value == 0;
                            carry_condition = value & 0x80 > 0;

                            value <<= 1;
                            if carry_condition {
                                value += 1;
                            }

                            self.write(address, value);
                        }

                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o010 => { //RRC r8 | RRC [HL]
                        let zero_condition ;
                        let carry_condition;

                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x01 > 0;

                            *operand >>= 1;
                            if carry_condition {
                                *operand += 0x80;
                            }
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);

                            zero_condition = value == 0;
                            carry_condition = value & 0x01 > 0;

                            value >>= 1;
                            if carry_condition {
                                value += 0x80;
                            }

                            self.write(address, value);
                        }

                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o020 => { //RL r8 | RL [HL]
                        let zero_condition ;
                        let carry_condition;

                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x80 > 0;

                            *operand <<= 1;
                            if self.flags & C_CARRY_FLAG > 0 {
                                *operand += 1;
                            }
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);

                            zero_condition = value == 0;
                            carry_condition = value & 0x80 > 0;

                            value <<= 1;
                            if self.flags & C_CARRY_FLAG > 0 {
                                value += 1;
                            }

                            self.write(address, value);
                        }

                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o030 => { //RR r8 | RR [HL]
                        let zero_condition ;
                        let carry_condition;
    
                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x01 > 0;
    
                            *operand >>= 1;
                            if self.flags & C_CARRY_FLAG > 0 {
                                *operand += 0x80;
                            }
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);
    
                            zero_condition = value == 0;
                            carry_condition = value & 0x01 > 0;
    
                            value >>= 1;
                            if self.flags & C_CARRY_FLAG > 0 {
                                value += 0x80;
                            }
    
                            self.write(address, value);
                        }
    
                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o040 => { //SLA r8 | SLA [HL]
                        let zero_condition ;
                        let carry_condition;

                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x80 > 0;

                            *operand <<= 1;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);

                            zero_condition = value == 0;
                            carry_condition = value & 0x80 > 0;

                            value <<= 1;

                            self.write(address, value);
                        }

                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o050 => { //SRA r8 | SRA [HL]
                        let zero_condition ;
                        let carry_condition;
    
                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x01 > 0;
    
                            let kept_bit = *operand & 0x80;
                            *operand >>= 1;
                            *operand |= kept_bit;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);
    
                            zero_condition = value == 0;
                            carry_condition = value & 0x01 > 0;
    
                            let kept_bit = value & 0x80;
                            value >>= 1;
                            value |= kept_bit;
    
                            self.write(address, value);
                        }
    
                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    0o060 => { //SWAP r8 | SWAP [HL]
                        let zero_condition;

                        if !is_hl {
                            zero_condition = *operand == 0;

                            let upper_nibble = (*operand) << 4;
                            let lower_nibble = (*operand) >> 4;
                            *operand = upper_nibble | lower_nibble;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);

                            zero_condition = value == 0;

                            let upper_nibble = value << 4;
                            let lower_nibble = value >> 4;
                            value = upper_nibble | lower_nibble;

                            self.write(address, value);
                        }

                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG | C_CARRY_FLAG);
                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                    }
                    0o070 => { //SRL r8 | SRL [HL]
                        let zero_condition ;
                        let carry_condition;
    
                        if !is_hl {
                            zero_condition = *operand == 0;
                            carry_condition = *operand & 0x01 > 0;
    
                            *operand >>= 1;
                        }
                        else {
                            let address = u16::from_be_bytes([self.h, self.l]);
                            let mut value = self.read(address);
    
                            zero_condition = value == 0;
                            carry_condition = value & 0x01 > 0;
    
                            value >>= 1;
    
                            self.write(address, value);
                        }
    
                        self.flag_toggle(zero_condition, Z_ZERO_FLAG);
                        self.flag_toggle(false, N_SUBTRACTION_FLAG | H_HALF_CARRY_FLAG);
                        self.flag_toggle(carry_condition, C_CARRY_FLAG);
                    }
                    _ => panic!("ERROR: instruction octet out of bounds!")
                };
            }
            0o100 => { //BIT u3, r8 | BIT u3, [HL]
                let u3 = (opcode & 0o070) >> 3;
                let tested_bit = 1 << u3;

                if !is_hl{
                    let condition = *operand & tested_bit == 0;
                    self.flag_toggle(condition, Z_ZERO_FLAG);
                }
                else {
                    cycle_count = 12;
                    let address = u16::from_be_bytes([self.h,self.l]);
                    let value = self.read(address);
                    self.flag_toggle(value & tested_bit == 0, Z_ZERO_FLAG);
                }

                self.flag_toggle(false, N_SUBTRACTION_FLAG);
                self.flag_toggle(true, H_HALF_CARRY_FLAG);
            }
            0o200 => { //RES u3, r8 | RES u3, [HL]
                let u3 = (opcode & 0o070) >> 3;
                let reset_bit = 1 << u3;

                if !is_hl{
                    *operand &= 0xFF ^ reset_bit;
                }
                else {
                    let address = u16::from_be_bytes([self.h,self.l]);
                    let value = self.read(address) & (0xFF ^ reset_bit);
                    self.write(address, value);
                }
            }
            0o300 => { //SET u3, r8 | SET u3, [HL]
                let u3 = (opcode & 0o070) >> 3;
                let set_bit = 1 << u3;

                if !is_hl{
                    *operand |= set_bit;
                }
                else {
                    let address = u16::from_be_bytes([self.h,self.l]);
                    let value = self.read(address) | set_bit;
                    self.write(address, value);
                }
            }
            _ => panic!("ERROR: Block octet out of bounds!")
        }

        cycle_count
    }
}

enum IMEState {
    Enabled,
    Disabled,
    Pending
}