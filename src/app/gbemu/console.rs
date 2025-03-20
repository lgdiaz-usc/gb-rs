use std::{fs::File, io::Bytes};

use crate::{app::cartridge_info::CartridgeInfo, mappers::{Mapper, NoMBC, MBC1}};

use super::ppu::{self, Pixel, PPU};

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
    working_ram: [u8; 0x2000],
    aux_working_ram: Vec<[u8; 0x4000]>,
    aux_working_ram_index: usize,
    high_ram: [u8; 0x80],

    //Interrupt registers
    pub interrupt_master_enable_flag: IMEState,
    interrupt_enable: u8,
    interrupt_flag: u8,

    //Serial data transfer registers
    serial_byte: u8, //SB
    serial_control: u8, //SC
    serial_counter: u8, //Counts down from 8 per cycle.

    //Timing registers
    timer_divider: u16, //DIV
    timer_counter: u8, //TIMA
    timer_modulo: u8, //TMA
    timer_control: u8, //TAC
    timer_overflowed: bool,

    //DMG Pallette registers
    pub dmg_bg_pallette: u8,    //BGP
    pub dmg_obj_pallette_0: u8, //OBP0
    pub dmg_obj_pallette_1: u8, //OBP1

    //DMA registers
    dma: u8,
    dma_counter: u16,

    //Misc variables
    pub is_halted: bool,

    //External objects
    ppu: PPU
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
            working_ram: [0; 0x2000],
            aux_working_ram: aux_working_ram,
            aux_working_ram_index: 0,
            high_ram: [0; 0x80],
            interrupt_master_enable_flag: IMEState::Disabled,
            interrupt_enable: 0x00,
            interrupt_flag: 0xE1,
            serial_byte: 0x00,
            serial_control: 0x7E,
            serial_counter: 0,
            timer_divider: 0xAB << 6,
            timer_counter: 0x00,
            timer_modulo: 0x00,
            timer_control: 0xF8,
            timer_overflowed: false,
            dmg_bg_pallette: 0xFC,
            dmg_obj_pallette_0: 0x00,
            dmg_obj_pallette_1: 0x00,
            dma: 0xFF,
            dma_counter: 0xA0 << 2,
            is_halted: false,
            ppu: ppu::PPU::new(),
        }
    }

    fn read(&self, address: u16) -> u8 {
        //Cartrige ROM
        if address < 0x8000 {
            self.cartridge.read(address)
        }
        //VRAM
        else if address < 0xA000 {
            self.ppu.read(address)
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
            //panic!("ERROR: Echo RAM access prohibited");
            self.read(address - 0x2000)
        }
        //Object Attribute Memory
        else if address < 0xFEA0 {
            self.ppu.read(address)
        }
        //Not Usable (Use is prohibited by Nintendo)
        else if address < 0xFF00 {
            //TODO: Properly implement this address space (VERY low priority)
            //panic!("ERROR: Prohibited Address Space")
            let ppu_state = self.ppu.get_mode();
            if ppu_state == 2 || ppu_state == 3 {
                0xFF
            }
            else {
                0x00
            }
        }
        //I/O Registers
        else if address < 0xFF80 {
            //TODO: Implement I/O Registers
            match address {
                0xFF00 => 0, //P1/JOYP
                0xFF01 => self.serial_byte, //SB
                0xFF02 => self.serial_control, //SC
                0xFF04 => (self.timer_divider >> 6).to_be_bytes()[1], //DIV
                0xFF05 => self.timer_counter, //TIMA
                0xFF06 => self.timer_modulo, //TMA
                0xFF07 => self.timer_control, //TAC
                0xFF0F => self.interrupt_flag, //IF
                0xFF10..0xFF27 => 0, //Audio registers
                0xFF30..0xFF40 => 0, //Waveform registers             
                0xFF46 => self.dma, //DMA transfer source address 0xXX00 + dma_counter
                0xFF47 => self.dmg_bg_pallette, //BGP
                0xFF48 => self.dmg_obj_pallette_0, //OBP0
                0xFF49 => self.dmg_obj_pallette_1, //OBP1
                0xFF40..0xFF46 | 0xFF4A | 0xFF4B => self.ppu.read(address), //PPU Registers
                0xFF4D => 0, //KEY1
                0xFF4F => 0, //VBK
                0xFF51..0xFF55 => 0, //HDMA1-4 (write only)
                0xFF55 => 0, //HDMA5
                0xFF56 => 0, //RP
                0xFF68..0xFF6D => 0, //Other CGB registers
                0xFF70 => 0, //SVBK
                0xFF76 | 0xFF77 => 0, //CGB Audio registers
                _ => {
                    println!("ERROR: Unkown register at address ${:x}", address);
                    0
                }
            }
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize]
        }
        //Interrupt Enable Register
        else {
            self.interrupt_enable
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
            self.ppu.write(address, value);
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
            //panic!("ERROR: Echo RAM access prohibited");
            self.write(address - 0x2000, value);
        }
        //Object Attribute Memory
        else if address < 0xFEA0 {
            self.ppu.write(address, value);
        }
        //Not Usable (Use is prohibited by Nintendo)
        else if address < 0xFF00 {
            //TODO: Properly implement this address space (VERY low priority)
            //panic!("ERROR: Prohibited Address Space")
        }
        //I/O Registers
        else if address < 0xFF80 {
            //TODO: Implement I/O Registers
            let register = match address {
                0xFF00 => return, //P1/JoyP
                0xFF01 => { //SB
                    if self.serial_control & 0x80 > 0 {
                        return;
                    }
                    &mut self.serial_byte
                }
                0xFF02 => { //SC
                    if self.serial_control & 0x80 > 0 {
                        self.serial_control = value | 0x80;
                        return;
                    }
                    else if value & 0x80 > 0 {
                        self.serial_counter = 8;
                    }
                    &mut self.serial_control
                }
                0xFF04 => { //DIV
                    self.timer_divider = 0;
                    return;                }
                0xFF05 => &mut self.timer_counter, //TIMA
                0xFF06 => &mut self.timer_modulo, //TMA
                0xFF07 => &mut self.timer_control, //TAC
                0xFF0f => &mut self.interrupt_flag, //IF
                0xFF46 => { //DMA transfer address. Also starts the DMA transfer process be resetting the dma_counter
                    self.dma_counter = 0;
                    self.dma = if value < 0xDf {value} else {0xDF};
                    return;
                }
                0xFF10..0xFF27 => return, //Sound registers
                0xFF30..0xFF40 => return, //Waveform registers
                0xFF47 => &mut self.dmg_bg_pallette, //BGP
                0xFF48 => &mut self.dmg_obj_pallette_0, //OBP0
                0xFF49 => &mut self.dmg_obj_pallette_1, //OBP1
                0xFF40..0xFF46 | 0xFF4A | 0xFF4B => { //PPU Registers
                    self.ppu.write(address, value);
                    return;
                }
                0xFF4D => return, //KEY1
                0xFF4F => return, //VBK
                0xFF51..0xFF56 => return, //HDMA1-5
                0xFF56 => return, //RP
                0xFF68..0xFF6D => return, //Other CGB registers
                0xFF70 => return, //SVBK
                0xFF76 | 0xFF77 => return, //CGB audio registers
                _ => {
                    println!("ERROR: Unknown register at address ${:x}", address);
                    return;
                }
            };

            *register = value;
        }
        //HRAM
        else if address < 0xFFFF {
            self.high_ram[(address - 0xFF80) as usize] = value;
        }
        //Interrupt Enable Register
        else {
            self.interrupt_enable = value;
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

    pub fn handle_interrupt(&mut self) -> u8 {
        if self.is_halted && self.interrupt_flag & 0b00011111 > 0 {
            self.is_halted = false;
        }

        if self.interrupt_master_enable_flag == IMEState::Enabled {
            let mut bit_to_check = 0b1;

            for _ in 0..5 {
                if (self.interrupt_enable & bit_to_check > 0) && (self.interrupt_flag & bit_to_check > 0) {
                    break;
                }
                else {
                    bit_to_check <<= 1;
                }
            }

            let interrupt_vector = match bit_to_check {
                0b1 => 0x40,
                0b10 => 0x48,
                0b100 => 0x50,
                0b1000 => 0x58,
                0b10000 => 0x60,
                _ => return 0
            };

            self.stack_pointer -= 2;
            self.write_16(self.stack_pointer, self.program_counter);
            //self.program_counter = self.read_16(interrupt_vector);
            self.program_counter = interrupt_vector;
            self.interrupt_master_enable_flag = IMEState::Disabled;
            self.interrupt_flag &= 0xFF ^ bit_to_check;
            return 20;
        }

        0
    }

    pub fn update_ppu(&mut self) {
        if self.dma_counter < 0xA0 << 2 {
            if self.dma_counter & 0b11 == 0 {
                let lsb = u16::to_be_bytes(self.dma_counter >> 2)[1];
                let source_address = u16::from_be_bytes([self.dma, lsb]);
                let value = self.read(source_address);
                self.ppu.dma_transfer(value, lsb);
            }

            self.dma_counter += 1;
        }

        self.ppu.update();

        //Update Interrupt flags
        let stat = self.ppu.read(0xFF41);
        let mut interrupt_flag_temp = self.interrupt_flag & 0b11111100;

        if self.ppu.get_mode() == 1 { //If in VBLANK mode, set VBLANK flag
            interrupt_flag_temp |= 0b1;
        }

        //Set STAT/LCD flag if:
        if stat & 0b1011 == 0b1000 || //STAT mode 0 is selcted and the mode is 0
         stat & 0b10011 == 0b10001 || //STAT mode 1 is selected and the mode is 1
         stat & 0b100011 == 0b100010 || //STAT mode 2 is selected and the mode is 2
         stat & 0b1000100 == 0b1000100 { //LYC check is selected and LY == LYC
            interrupt_flag_temp |= 0b10;
        }
        
        self.interrupt_flag = interrupt_flag_temp;
    }

    pub fn check_serial(&mut self) -> Option<u8> {
        let mut transferred_byte = None;

        if self.serial_counter == 8 {
            transferred_byte = Some(self.serial_byte);
        }

        if self.serial_counter > 0 {
            self.serial_counter -= 1;
            self.serial_byte <<= 1;

            if self.serial_counter == 0 {
                self.serial_control &= 0x7F;
                self.interrupt_flag |= 0b1000;
            }
        }

        transferred_byte
    }

    pub fn update_timer(&mut self) {
        //timer counter is reset and interrupt is requested on the m-cycle after overflow
        if self.timer_overflowed {
            self.timer_counter = self.timer_modulo;
            self.interrupt_flag |= 0b100;
            self.timer_overflowed = false;
        }

        self.timer_divider += 1;

        if self.timer_control & 0b100 > 0 {
            let increment_every = match self.timer_control & 0b11 {
                0b00 => 0xFF,
                0b01 => 0x04,
                0b10 => 0x0F,
                0b11 => 0x3F,
                _ => panic!("ERROR: Increment value out of bounds!")
            };

            if self.timer_divider & increment_every == 0 {
                self.timer_counter += 1;

                if self.timer_counter == 0 {
                    self.timer_overflowed = true;
                }
            }
        }
    }

    pub fn dump_screen(&self) -> &[[Pixel; 160]; 144] {
        self.ppu.dump_screen()
    }

    pub fn execute_instruction(&mut self) -> u8 {
        let mut instruction_size = 1;
        let mut cycle_count = 4;

        let opcode = self.read(self.program_counter);

        if false {
           self.debug_message(opcode);
        }
        
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
                        self.a -= 0x6;
                    }
                    if self.flags & C_CARRY_FLAG > 0 {
                        self.a -= 0x60;
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
                if self.interrupt_master_enable_flag == IMEState::Enabled {
                    self.is_halted = true;
                }
                else if self.interrupt_flag & 0b00011111 == 0{
                    self.is_halted = true;
                }
            }

            //Block 3 one-offs
            0o303 => { //JP
                instruction_size = 0;
                cycle_count = 16;
                self.program_counter = self.read_16(self.program_counter + 1);
            }
            0o311 => { //RET
                cycle_count = 16;
                instruction_size = 0;
                self.program_counter = self.read_16(self.stack_pointer);
                self.stack_pointer += 2;
            }
            0o313 => { //PREFIX
                instruction_size = 2;
                cycle_count = self.execute_prefixed_instruction();
            }
            0o315 => { //CALL
                cycle_count = 6;
                instruction_size = 0;
                self.stack_pointer -= 2;
                self.write_16(self.stack_pointer, self.program_counter + 3);
                self.program_counter = self.read_16(self.program_counter + 1);
            }
            0o331 => { //RETI
                cycle_count = 16;
                instruction_size = 0;
                self.program_counter = self.read_16(self.stack_pointer);
                self.stack_pointer += 2;
                self.interrupt_master_enable_flag = IMEState::Enabled;
            }
            0o340 => { //LDH [a8], A
                cycle_count = 12;
                instruction_size = 2;
                let address = u16::from_be_bytes([0xFF, self.read(self.program_counter + 1)]);
                
                self.write(address, self.a);
            }
            0o350 => { //ADD SP, e8
                instruction_size = 2;
                cycle_count = 16;

                let offset_lsb = self.read(self.program_counter + 1);
                let offset;
                if offset_lsb & 0x80 == 0 {
                    offset = u16::from_be_bytes([0x00, offset_lsb]);
                }
                else {
                    offset = u16::from_be_bytes([0xFF, offset_lsb]);
                }

                self.stack_pointer += offset;

                let carry_check = self.stack_pointer.to_be_bytes()[1];
                self.flag_toggle((carry_check & 0xF0) < (offset_lsb & 0xFF), H_HALF_CARRY_FLAG);
                self.flag_toggle(carry_check < offset_lsb, C_CARRY_FLAG);
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG);
            }
            0o351 => { //JP HL
                instruction_size = 0;
                self.program_counter = u16::from_be_bytes([self.h, self.l]);
            }
            0o360 => { //LDH A, [a8]
                instruction_size = 2;
                cycle_count = 12;
                let address = u16::from_be_bytes([0xFF, self.read(self.program_counter + 1)]);

                self.a = self.read(address);
            }
            0o363 => { //DI
                self.interrupt_master_enable_flag = IMEState::Disabled;
            }
            0o370 => { //LD HL, SP + e8
                instruction_size = 2;
                cycle_count = 12;

                let offset_lsb = self.read(self.program_counter + 1);
                let offset;
                if offset_lsb & 0x80 == 0 {
                    offset = u16::from_be_bytes([0x00, offset_lsb]);
                }
                else {
                    offset = u16::from_be_bytes([0xFF, offset_lsb]);
                }

                let new_pointer = self.stack_pointer + offset;
                (self.h, self.l) = new_pointer.to_be_bytes().into();

                let carry_check = new_pointer.to_be_bytes()[1];
                self.flag_toggle((carry_check & 0xF0) < (offset_lsb & 0xFF), H_HALF_CARRY_FLAG);
                self.flag_toggle(carry_check < offset_lsb, C_CARRY_FLAG);
                self.flag_toggle(false, Z_ZERO_FLAG | N_SUBTRACTION_FLAG);
            }
            0o371 => { //LD SP, HL
                cycle_count = 8;
                self.stack_pointer = u16::from_be_bytes([self.h,self.l]);
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
                                    instruction_size += 2;
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
                                let (register_high, register_low) = match opcode & 0o060 {
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
                                let incrementor = if opcode & 0o007 == 0o004 {1} else {u8::MAX};
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
                
                    0o200 => { //Block 2
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
                                    instruction_size = 0;
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
                                self.flags &= 0xF0;
                            }
                            0o002 if opcode & 0o070 >= 0o040 => { //LDH [C], A | LD [a16], A | LDH A, [C] | LD A, [a16]
                                let address;
                                if opcode & 0o010 == 0 {
                                    cycle_count = 8;
                                    address = u16::from_be_bytes([0xFF, self.c]);
                                }
                                else {
                                    cycle_count = 16;
                                    instruction_size = 3;
                                    address = self.read_16(self.program_counter + 1);
                                }

                                if opcode & 0o020 == 0 {
                                    self.write(address, self.a);
                                }
                                else {
                                    self.a = self.read(address);
                                }
                            }
                            0o002 => { //JP cc
                                let jump_condition = match opcode & 0o030 {
                                    0o000 => self.flags & Z_ZERO_FLAG == 0,
                                    0o010 => self.flags & Z_ZERO_FLAG > 0,
                                    0o020 => self.flags & C_CARRY_FLAG == 0,
                                    0o030 => self.flags & C_CARRY_FLAG > 0,
                                    _ => panic!("ERROR: condition octet out of bounds!")
                                };
                                if jump_condition {
                                    instruction_size = 0;
                                    cycle_count = 16;
                                    self.program_counter = self.read_16(self.program_counter + 1);
                                }
                                else {
                                    cycle_count = 12;
                                    instruction_size = 3;
                                }
                            }
                            0o004 => { //CALL cc
                                let jump_condition = match opcode & 0o030 {
                                    0o000 => self.flags & Z_ZERO_FLAG == 0,
                                    0o010 => self.flags & Z_ZERO_FLAG > 0,
                                    0o020 => self.flags & C_CARRY_FLAG == 0,
                                    0o030 => self.flags & C_CARRY_FLAG > 0,
                                    _ => panic!("ERROR: condition octet out of bounds!")
                                };
                                if jump_condition {
                                    instruction_size = 0;
                                    cycle_count = 24;
                                    self.stack_pointer -= 2;
                                    self.write_16(self.stack_pointer, self.program_counter + 3);
                                    self.program_counter = self.read_16(self.program_counter + 1);
                                }
                                else {
                                    cycle_count = 12;
                                    instruction_size = 3;
                                }
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
                                let jump_address_vector = match opcode & 0o070 {
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
                                
                                let return_address = self.program_counter + 1;
                                self.stack_pointer -= 2;
                                self.write_16(self.stack_pointer, return_address);

                                instruction_size = 0;
                                //self.program_counter = self.read_16(jump_address_vector);
                                self.program_counter = jump_address_vector;
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

    fn debug_message(&self, opcode: u8) {
        let instruction = match opcode {
            0o000 => format!("NOP"),
            0o010 => format!("LD [{:x}], SP", self.read_16(self.program_counter + 1)),
            0o020 => format!("STOP ${:x}", self.read(self.program_counter + 1)),

            0o007 => format!("RLCA"),
            0o017 => format!("RRCA"),
            0o027 => format!("RLA"),
            0o037 => format!("RRA"),

            0o047 => format!("DAA"),
            0o057 => format!("CPL"),
            0o067 => format!("SCF"),
            0o077 => format!("CCF"),

            0o166 => format!("HALT"),

            0o340 => format!("LDH [{:x}], A", self.read(self.program_counter + 1)),
            0o350 => format!("ADD SP, {}", self.read(self.program_counter + 1) as i8),
            0o360 => format!("LDH A, [{:x}]", self.read(self.program_counter + 1)),
            0o370 => format!("LD HL, SP + {}", self.read(self.program_counter + 1) as i8),

            0o311 => format!("RET"),
            0o331 => format!("RETI"),
            0o351 => format!("JP HL"),
            0o371 => format!("LD SP, HL"),

            0o303 => format!("JP ${:x}", self.read_16(self.program_counter + 1)),
            0o313 => debug_message_prefixed(self.read(self.program_counter + 1)),
            0o363 => format!("DI"),
            0o373 => format!("EI"),
            
            0o315 => format!("CALL ${:x}", self.read_16(self.program_counter + 1)),

            0o323 | 0o333 | 0o335 | 0o343 | 0o344 | 0o353 | 0o354 | 0o355 | 0o364 | 0o374 | 0o375 => format!("ILLEGAL OPCODE ${:x}", opcode),

            _ => match opcode & 0o300 {
                0o000 => {
                    match opcode & 0o007 {
                        0o000 => {
                            let condition = match opcode & 0o070 {
                                0o030 => "",
                                0o040 => "NZ",
                                0o050 => "Z",
                                0o060 => "NC",
                                0o070 => "C",
                                _ => panic!()
                            };
                            let jump_pointer = self.read(self.program_counter + 1);
                            format!("JR {}, {:+}", condition, jump_pointer as i8)
                        }
                        0o001 => {
                            let register = match opcode & 0o060 {
                                0o000 => "BC",
                                0o020 => "DE",
                                0o040 => "HL",
                                0o060 => "SP",
                                _ => panic!()
                            };

                            if opcode & 0o010 == 0 {
                                format!("LD {}, ${:x}", register, self.read_16(self.program_counter + 1))
                            }
                            else {
                                format!("ADD HL, {}", register)
                            }
                        }
                        0o002 => {
                            let register = match opcode & 0o060 {
                                0o000 => "BC",
                                0o020 => "DE",
                                0o040 => "HL+",
                                0o060 => "HL-",
                                _ => panic!()
                            };

                            if opcode & 0o010 == 0 {
                                format!("LD [{}], A", register)
                            }
                            else {
                                format!("LD A, [{}]", register)
                            }
                        }
                        0o003 => {
                            let register = match opcode & 0o060 {
                                0o000 => "BC",
                                0o020 => "DE",
                                0o040 => "HL",
                                0o060 => "SP",
                                _ => panic!()
                            };

                            if opcode & 0o010 == 0 {
                                format!("INC {}", register)
                            }
                            else {
                                format!("DEC {}", register)
                            }
                        }
                        0o004 | 0o005 => {
                            let register = match opcode & 0o070 {
                                0o000 => "B",
                                0o010 => "C",
                                0o020 => "D",
                                0o030 => "E",
                                0o040 => "H",
                                0o050 => "L",
                                0o060 => "[HL]",
                                0o070 => "A",
                                _ => panic!()
                            };

                            if opcode & 0o007 == 0o004 {
                                format!("INC {}", register)
                            }
                            else {
                                format!("DEC {}", register)
                            }
                        }
                        0o006 => {
                            let register = match opcode & 0o070 {
                                0o000 => "B",
                                0o010 => "C",
                                0o020 => "D",
                                0o030 => "E",
                                0o040 => "H",
                                0o050 => "L",
                                0o060 => "[HL]",
                                0o070 => "A",
                                _ => panic!()
                            };
                            format!("LD {}, ${:x}", register, self.read(self.program_counter + 1))
                        }
                        _ => panic!()
                    }
                }
                0o100 => {
                    let src = match opcode & 0o007 {
                        0o000 => "B",
                        0o001 => "C",
                        0o002 => "D",
                        0o003 => "E",
                        0o004 => "H",
                        0o005 => "L",
                        0o006 => "[HL]",
                        0o007 => "A",
                        _ => panic!()
                    };
                    let dest = match opcode & 0o070 {
                        0o000 => "B",
                        0o010 => "C",
                        0o020 => "D",
                        0o030 => "E",
                        0o040 => "H",
                        0o050 => "L",
                        0o060 => "[HL]",
                        0o070 => "A",
                        _ => panic!()
                    };
                    format!("LD {}, {}", dest, src)
                }
                0o200 => {
                    let src = match opcode & 0o007 {
                        0o000 => "B",
                        0o001 => "C",
                        0o002 => "D",
                        0o003 => "E",
                        0o004 => "H",
                        0o005 => "L",
                        0o006 => "[HL]",
                        0o007 => "A",
                        _ => panic!()
                    };
                    let op = match opcode & 0o070 {
                        0o000 => "ADD",
                        0o010 => "ADC",
                        0o020 => "SUB",
                        0o030 => "SBC",
                        0o040 => "AND",
                        0o050 => "XOR",
                        0o060 => "OR",
                        0o070 => "CP",
                        _ => panic!()
                    };
                    format!("{} A, {}", op, src)
                }
                0o300 => {
                    match opcode & 0o007 {
                        0o000 => {
                            let condition = match opcode & 0o070 {
                                0o000 => "NZ",
                                0o010 => "Z",
                                0o020 => "NC",
                                0o030 => "C",
                                _ => panic!()
                            };
                            format!("RET {}", condition)
                        }
                        0o001 => {
                            let register = match opcode & 0o070 {
                                0o000 => "BC",
                                0o020 => "DE",
                                0o040 => "HL",
                                0o060 => "AF",
                                _ => panic!()
                            };
                            format!("POP {}", register)
                        }
                        0o002 if opcode & 0o040 == 0 => {
                            let condition = match opcode & 0o070 {
                                0o000 => "NZ",
                                0o010 => "Z",
                                0o020 => "NC",
                                0o030 => "C",
                                _ => panic!()
                            };
                            format!("JP {}, ${:x}", condition, self.read_16(self.program_counter + 1))
                        }
                        0o002 => {
                            let (op, register) = if opcode & 0o010 == 0 {("LDH", format!("C"))} else {("LD", format!("{:x}", self.read_16(self.program_counter + 1)))};
                            if opcode & 0o020 == 0 {
                                format!("{} [{}], A", op, register)
                            }
                            else {
                                format!("{} A, [{}]", op, register)
                            }
                        }
                        0o004 => {
                            let condition = match opcode & 0o070 {
                                0o000 => "NZ",
                                0o010 => "Z",
                                0o020 => "NC",
                                0o030 => "C",
                                _ => panic!()
                            };
                            format!("CALL {}, ${:x}", condition, self.read_16(self.program_counter + 1))
                        }
                        0o005 => {
                            let register = match opcode & 0o070 {
                                0o000 => "BC",
                                0o020 => "DE",
                                0o040 => "HL",
                                0o060 => "AF",
                                _ => panic!()
                            };
                            format!("PUSH {}", register)
                        }
                        0o006 => {
                            let op = match opcode & 0o070 {
                                0o000 => "ADD",
                                0o010 => "ADC",
                                0o020 => "SUB",
                                0o030 => "SBC",
                                0o040 => "AND",
                                0o050 => "XOR",
                                0o060 => "OR",
                                0o070 => "CP",
                                _ => panic!()
                            };
                            format!("{} A, ${:x}", op, self.read(self.program_counter + 1))
                        }
                        0o007 => {
                            let vector = match opcode & 0o070 {
                                0o000 => "00",
                                0o010 => "08",
                                0o020 => "10",
                                0o030 => "18",
                                0o040 => "20",
                                0o050 => "28",
                                0o060 => "30",
                                0o070 => "38",
                                _ => panic!()
                            };
                            format!("RST ${}", vector)
                        }
                        _ => panic!("Unknown opcode {:o}", opcode)
                    }
                }
                _ => format!("ERROR: Invalid opcode!")
            }
        };

        println!("{:x}: {}", self.program_counter, instruction);
    }
}

fn debug_message_prefixed(opcode: u8) -> String {
    let register = match opcode & 0o007 {
        0o000 => "B",
        0o001 => "C",
        0o002 => "D",
        0o003 => "E",
        0o004 => "H",
        0o005 => "L",
        0o006 => "[HL]",
        0o007 => "A",
        _ => panic!()
    };
    
    if opcode & 0o300 == 0 {
        let op = match opcode & 0o070 {
            0o000 => "RLC",
            0o010 => "RRC",
            0o020 => "RL",
            0o030 => "RR",
            0o040 => "SLA",
            0o050 => "SRA",
            0o060 => "SWAP",
            0o070 => "SRL",
            _ => panic!()
        };
        format!("{} {}", op, register)
    }
    else {
        let op = match opcode & 0o300 {
            0o100 => "BIT",
            0o200 => "RES",
            0o300 => "SET",
            _ => panic!()
        };
        let bit = (opcode & 0o070) >> 3;
        format!("{} {}, {}", op, bit, register)
    }
}

#[derive(PartialEq)]
pub enum IMEState {
    Enabled,
    Disabled,
    Pending
}