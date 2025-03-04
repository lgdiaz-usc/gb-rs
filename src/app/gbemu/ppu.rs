pub struct PPU {
    //Memory
    video_ram: Vec<[u8; 0x4000]>,
    video_ram_index: usize,
    object_attribute_memory: [u8; 0xA0],

    //LCDC register values
        //All u16 lcdc values are subtracted by 0x8000 so they can be plugged into video_ram directly
    lcdc_7_lcd_enabled: bool, 
    lcdc_6_window_tile_map_area: u16, //0x9800, 0x9C00
    lcdc_5_window_enabled: bool,
    lcdc_4_tile_data_area: u16, //0x8800, 0x8000
    lcdc_3_bg_tile_map_area: u16, //0x9800, 0x9C00
    lcdc_2_obj_is_tall: bool, //0: 8x8, 1: 8x16
    lcdc_1_obj_enable: bool,
    lcdc_0_bg_window_enable: bool,
}

impl PPU {
    pub fn new() -> Self {
        let mut video_ram = Vec::new();
        video_ram.push([0; 0x4000]);

        Self {
            video_ram,
            video_ram_index: 0,
            object_attribute_memory: [0; 0xA0],
            lcdc_7_lcd_enabled: true,
            lcdc_6_window_tile_map_area: 0x9800 - 0x8000,
            lcdc_5_window_enabled: false,
            lcdc_4_tile_data_area: 0x8000 - 0x8000,
            lcdc_3_bg_tile_map_area: 0x9800 - 0x8000,
            lcdc_2_obj_is_tall: false,
            lcdc_1_obj_enable: false,
            lcdc_0_bg_window_enable: true,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0x8000 && address <= 0x9fff {
            self.video_ram[self.video_ram_index][(address - 0x8000) as usize]
        }
        else if address >= 0xFE00 && address <= 0xFE9F {
            self.object_attribute_memory[(address - 0xFE00) as usize]
        }
        else if address >= 0xFF00 && address <= 0xFF7F {
            //TODO: Implement PPU registers
            match address {
                0xFF40 => { //LCDC
                    let mut lcdc = 0;
                    if self.lcdc_7_lcd_enabled {
                        lcdc |= 128;
                    }
                    if self.lcdc_6_window_tile_map_area == 0x9C00 - 0x8000 {
                        lcdc |= 64;
                    }
                    if self.lcdc_5_window_enabled {
                        lcdc |= 32;
                    }
                    if self.lcdc_4_tile_data_area == 0x8000 - 0x8000 {
                        lcdc |= 16;
                    }
                    if self.lcdc_3_bg_tile_map_area == 0x9c00 - 0x8000 {
                        lcdc |= 8;
                    }
                    if self.lcdc_2_obj_is_tall {
                        lcdc |= 4;
                    }
                    if self.lcdc_1_obj_enable {
                        lcdc |= 2;
                    }
                    if self.lcdc_0_bg_window_enable {
                        lcdc |= 1;
                    }

                    lcdc
                }
                _ => panic!("ERROR: Unknown register at address ${:x}", address)
            }
        }
        else {
            panic!("ERROR: Address out of bounds!")
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address >= 0x8000 && address <= 0x9fff {
            self.video_ram[self.video_ram_index][(address - 0x8000) as usize] = value;
        }
        else if address >= 0xFE00 && address <= 0xFE9F {
            self.object_attribute_memory[(address - 0xFE00) as usize] = value;
        }
        else if address >= 0xFF00 && address <= 0xFF7F {
            //TODO: Implement PPU registers
            match address {
                0xFF40 => { //LCDC
                    self.lcdc_7_lcd_enabled = value & 128 > 0;
                    self.lcdc_6_window_tile_map_area = if value & 64 > 0 {0x9C00 - 0x8000} else {0x9800 - 0x8000};
                    self.lcdc_5_window_enabled = value & 32 > 0;
                    self.lcdc_4_tile_data_area = if value & 16 > 0 {0x8000 - 0x8000} else {0x8800 - 0x8000};
                    self.lcdc_3_bg_tile_map_area = if value & 8 > 0 {0x9C00 - 0x8000} else {0x9800 - 0x8000};
                    self.lcdc_2_obj_is_tall = value & 4 > 0;
                    self.lcdc_1_obj_enable = value & 2 > 0;
                    self.lcdc_0_bg_window_enable = value & 1 > 0;
                }
                _ => panic!("ERROR: Unkown register at address ${:x}", address)
            }
        }
        else {
            panic!("ERROR: Address out of bounds!")
        }
    }
}