pub struct APU {
    //Channel 1 registers
    ch_1_0_sweep: u8,   //NR10
    ch_1_1_length: u8,  //NR11
    ch_1_2_volume: u8,  //NR12
    ch_1_3_period: u16,     //NR13
    ch_1_4_length_enable: bool,    //NR14

    //Channel 2 registers
    ch_2_1_length: u8,   //NR21
    ch_2_2_volume: u8,   //NR22
    ch_2_3_period: u16,      //NR23
    ch_2_4_length_enable: bool,     //NR24

    //Channel 3 registers
    ch_3_0_enable: bool, //NR30
    ch_3_1_length: u8,   //NR31
    ch_3_2_volume: u8,   //NR32
    ch_3_3_period: u16,      //NR33
    ch_3_4_length_enable: bool,     //NR34

    //Channel 4 registers
    ch_4_1_length: u8,   //NR41
    ch_4_2_volume: u8,   //NR42
    ch_4_3_freq: u8,     //NR43
    ch_4_4_length_enable: bool,  //NR44

    //Master control registers
    ch_5_0_volume: u8,   //NR50
    ch_5_1_panning: u8,  //NR51
    ch_5_2_enable: u8,   //NR52

    //Wave RAM
    wave_ram: [u8; 16],

    //Timer for the APU
    apu_counter: u16, //DIV-APU
}

impl APU {
    pub fn new() -> Self {
        Self {
            ch_1_0_sweep: 0x80,
            ch_1_1_length: 0xBF,
            ch_1_2_volume: 0xF3,
            ch_1_3_period: 0x0000,
            ch_1_4_length_enable: true,
            ch_2_1_length: 0x3F,
            ch_2_2_volume: 0x00,
            ch_2_3_period: 0x0000,
            ch_2_4_length_enable: true,
            ch_3_0_enable: false,
            ch_3_1_length: 0xFF,
            ch_3_2_volume: 0x9F,
            ch_3_3_period: 0x00,
            ch_3_4_length_enable: true,
            ch_4_1_length: 0xFF,
            ch_4_2_volume: 0x00,
            ch_4_3_freq: 0x00,
            ch_4_4_length_enable: true,
            ch_5_0_volume: 0x77,
            ch_5_1_panning: 0xF3,
            ch_5_2_enable: 0xF1,
            wave_ram: [0; 16],
            apu_counter: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0xFF10 && address <= 0xFF26 {
            match address {
                0xFF10 => self.ch_1_0_sweep,
                0xFF11 => self.ch_1_1_length | 0b111111,
                0xFF12 => self.ch_1_2_volume,
                0xFF13 => 0xFF,
                0xFF14 => if self.ch_1_4_length_enable {0xFF} else {0xBF},
                0xFF16 => self.ch_2_1_length | 0b111111,
                0xFF17 => self.ch_2_2_volume,
                0xFF18 => 0xFF,
                0xFF19 => if self.ch_2_4_length_enable {0xFF} else {0xBF},
                0xFF1A => if self.ch_3_0_enable {0xFF} else {0x7F},
                0xFF1B => 0xFF,
                0xFF1C => self.ch_3_2_volume,
                0xFF1D => 0xFF,
                0xFF1E => if self.ch_2_4_length_enable {0xFF} else {0xBF},
                0xFF20 => 0xFF,
                0xFF21 => self.ch_4_2_volume,
                0xFF22 => self.ch_4_3_freq,
                0xFF23 => if self.ch_4_4_length_enable {0xFF} else {0xBF},
                0xFF24 => self.ch_5_0_volume,
                0xFF25 => self.ch_5_1_panning,
                0xFF26 => self.ch_5_2_enable,
                _ => {
                    println!("ERROR: Unknown register ${:x}", address);
                    0xFF
                }
            }
        }
        else if address >= 0xFF30 && address <= 0xFF3F {
            self.wave_ram[(address - 0xFF30) as usize]
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address >= 0xFF10 && address <= 0xFF26 {
            let mut value = value;
            let register = match address {
                0xFF10 => {
                    value |= 0b10000000;
                    &mut self.ch_1_0_sweep
                },
                0xFF11 => &mut self.ch_1_1_length,
                0xFF12 => &mut self.ch_1_2_volume,
                0xFF13 => {
                    self.ch_1_3_period &= 0xFF00;
                    self.ch_1_3_period |= value as u16;
                    return;
                },
                0xFF14 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 1
                    }

                    self.ch_1_4_length_enable = value & 0x40 != 0;
                    self.ch_1_3_period = (self.ch_1_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },
                0xFF16 => &mut self.ch_2_1_length,
                0xFF17 => &mut self.ch_2_2_volume,
                0xFF18 => {
                    self.ch_2_3_period &= 0xFF00;
                    self.ch_2_3_period |= value as u16;
                    return;
                },
                0xFF19 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 2
                    }

                    self.ch_2_4_length_enable = value & 0x40 != 0;
                    self.ch_2_3_period = (self.ch_2_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },
                0xFF1A => {
                    self.ch_3_0_enable = value & 0x80 != 0;
                    return;
                },
                0xFF1B => &mut self.ch_3_1_length,
                0xFF1C => {
                    value |= 0b1100000;
                    &mut self.ch_3_2_volume
                },
                0xFF1D => {
                    self.ch_3_3_period &= 0xFF00;
                    self.ch_3_3_period |= value as u16;
                    return;
                },
                0xFF1E => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 3
                    }

                    self.ch_3_4_length_enable = value & 0x40 != 0;
                    self.ch_3_3_period = (self.ch_3_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },
                0xFF20 => {
                    value |= 0b11000000;
                    &mut self.ch_4_1_length
                },
                0xFF21 => &mut self.ch_4_2_volume,
                0xFF22 => &mut self.ch_4_3_freq,
                0xFF23 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 4
                    }

                    self.ch_4_4_length_enable = value & 0x40 != 0;
                    return;
                },
                0xFF24 => &mut self.ch_5_0_volume,
                0xFF25 => &mut self.ch_5_1_panning,
                0xFF26 => {
                    value |= 0b1111111;
                    &mut self.ch_5_2_enable
                },
                _ => panic!("ERROR: Unknown register ${:x}", address)
            };

            *register = value;
        }
        else if address >= 0xFF30 && address <= 0xFF3F {
            self.wave_ram[(address - 0xFF30) as usize] = value;
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    pub fn update_apu_timer(&mut self) {
        self.apu_counter += 1;

        //TODO: Implement events that occur every N DIV-APU ticks
    }
}