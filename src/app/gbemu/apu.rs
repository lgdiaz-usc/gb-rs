pub struct APU {
    //Channel 1 registers
    ch_1_0_sweep: u8,   //NR10
    ch_1_1_length: u8,  //NR11
    ch_1_2_volume: u8,  //NR12
    ch_1_3_low: u8,     //NR13
    ch_1_4_high: u8,    //NR14

    //Channel 2 registers
    ch_2_1_length: u8,   //NR21
    ch_2_2_volume: u8,   //NR22
    ch_2_3_low: u8,      //NR23
    ch_2_4_high: u8,     //NR24

    //Channel 3 registers
    ch_3_0_enable: u8,   //NR30
    ch_3_1_length: u8,   //NR31
    ch_3_2_volume: u8,   //NR32
    ch_3_3_low: u8,      //NR33
    ch_3_4_high: u8,     //NR34

    //Channel 4 registers
    ch_4_1_length: u8,   //NR41
    ch_4_2_volume: u8,   //NR42
    ch_4_3_freq: u8,     //NR43
    ch_4_4_control: u8,  //NR44

    //Master control registers
    ch_5_0_volume: u8,   //NR50
    ch_5_1_panning: u8,  //NR51
    ch_5_2_enable: u8,   //NR52

    //Wave RAM
    wave_ram: [u8; 16],
}

impl APU {
    pub fn new() -> Self {
        Self {
            ch_1_0_sweep: 0x80,
            ch_1_1_length: 0xBF,
            ch_1_2_volume: 0xF3,
            ch_1_3_low: 0x00,
            ch_1_4_high: 0xFF,
            ch_2_1_length: 0x3F,
            ch_2_2_volume: 0x00,
            ch_2_3_low: 0xFF,
            ch_2_4_high: 0xBF,
            ch_3_0_enable: 0x7F,
            ch_3_1_length: 0xFF,
            ch_3_2_volume: 0x9F,
            ch_3_3_low: 0xFF,
            ch_3_4_high: 0xBF,
            ch_4_1_length: 0xFF,
            ch_4_2_volume: 0x00,
            ch_4_3_freq: 0x00,
            ch_4_4_control: 0xBF,
            ch_5_0_volume: 0x77,
            ch_5_1_panning: 0xF3,
            ch_5_2_enable: 0xF1,
            wave_ram: [0; 16],
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0xFF10 && address <= 0xFF26 {
            match address {
                0xFF10 => self.ch_1_0_sweep,
                0xFF11 => self.ch_1_1_length | 0b111111,
                0xFF12 => self.ch_1_2_volume,
                0xFF13 => 0x00,
                0xFF14 => self.ch_1_4_high | 0b10111111,
                0xFF16 => self.ch_2_1_length | 0b111111,
                0xFF17 => self.ch_2_2_volume,
                0xFF18 => 0x00,
                0xFF19 => self.ch_2_4_high | 0b10111111,
                0xFF1A => self.ch_3_0_enable,
                0xFF1B => 0xFF,
                0xFF1C => self.ch_3_2_volume,
                0xFF1D => 0xFF,
                0xFF1E => self.ch_3_4_high | 0b10111111,
                0xFF20 => 0xFF,
                0xFF21 => self.ch_4_2_volume,
                0xFF22 => self.ch_4_3_freq,
                0xFF23 => self.ch_4_4_control | 0b10111111,
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
}