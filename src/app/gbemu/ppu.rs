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

    //STAT register values
    ppu_mode:u8,
    stat: u8,

    //Standalone registers
    ly: u8,
    ly_compare: u8,
    scy: u8,
    scx: u8,

    //Buffers for the rendering process
    obj_buffer: Vec<u16>,

    //Misc. variables
    dot_counter: u16, //The current dot on the current scanline;
    mode_3_end_index: u16,
}

//PPU mode values 
const PPU_MODE_0_HBLANK: u8      = 0;
const PPU_MODE_1_VBLANK: u8      = 1;
const PPU_MODE_2_OAM_SCAN:  u8   = 2;
const PPU_MODE_3_DRAW_PIXELS: u8 = 3;

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
            ppu_mode: PPU_MODE_1_VBLANK,
            stat: 0x85,
            ly: 0,
            ly_compare: 0,
            scy: 0x00,
            scx: 0x00,
            obj_buffer: Vec::with_capacity(10),
            dot_counter: 0,
            mode_3_end_index: 172 + 80,
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
                0xFF41 => { //STAT
                    let mut stat = self.stat & 0b01111000;
                    if self.ly == self.ly_compare {
                        stat |= 0b100;
                    }
                    stat |= self.ppu_mode;

                    stat
                }
                0xFF42 => self.scy,
                0xFF43 => self.scx,
                0xFF44 => self.ly,
                0xFF45 => self.ly_compare, //LYC
                _ => panic!("ERROR: Unknown register at address ${:x}", address)
            }
        }
        else {
            panic!("ERROR: Address out of bounds!")
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address >= 0x8000 && address <= 0x9fff {
            if self.ppu_mode != PPU_MODE_3_DRAW_PIXELS {
                self.video_ram[self.video_ram_index][(address - 0x8000) as usize] = value;
            }
        }
        else if address >= 0xFE00 && address <= 0xFE9F {
            if self.ppu_mode != PPU_MODE_2_OAM_SCAN && self.ppu_mode != PPU_MODE_3_DRAW_PIXELS {
                self.object_attribute_memory[(address - 0xFE00) as usize] = value;
            }
        }
        else if address >= 0xFF00 && address <= 0xFF7F {
            //TODO: Implement PPU registers
            let register = match address {
                0xFF40 => { //LCDC
                    self.lcdc_7_lcd_enabled = value & 128 > 0;
                    self.lcdc_6_window_tile_map_area = if value & 64 > 0 {0x9C00 - 0x8000} else {0x9800 - 0x8000};
                    self.lcdc_5_window_enabled = value & 32 > 0;
                    self.lcdc_4_tile_data_area = if value & 16 > 0 {0x8000 - 0x8000} else {0x8800 - 0x8000};
                    self.lcdc_3_bg_tile_map_area = if value & 8 > 0 {0x9C00 - 0x8000} else {0x9800 - 0x8000};
                    self.lcdc_2_obj_is_tall = value & 4 > 0;
                    self.lcdc_1_obj_enable = value & 2 > 0;
                    self.lcdc_0_bg_window_enable = value & 1 > 0;
                    return;
                }
                0xFF41 => &mut self.stat, //STAT
                0xFF42 => &mut self.scy,
                0xFF43 => &mut self.scx,
                0xFF44 => return, //LY is read only!
                0xFF45 => &mut self.ly_compare, //LYC
                _ => panic!("ERROR: Unkown register at address ${:x}", address)
            };

            *register = value;
        }
        else {
            panic!("ERROR: Address out of bounds!")
        }
    }

    pub fn update(&mut self) {
        match self.ppu_mode {
            PPU_MODE_0_HBLANK => {
                
            }
            PPU_MODE_1_VBLANK => {

            }
            PPU_MODE_2_OAM_SCAN => {
                if self.dot_counter & 1 == 0 && self.obj_buffer.len() < 10 {
                    let obj_address = self.dot_counter << 1;
                    let obj_y = self.object_attribute_memory[obj_address as usize] - 17;

                    let obj_height = match self.lcdc_2_obj_is_tall {
                        true => 16,
                        false => 8
                    };
                    if self.ly >= obj_y && self.ly <= obj_height + obj_y {
                        self.obj_buffer.push(obj_address);
                    }
                }
            }
            PPU_MODE_3_DRAW_PIXELS => {

            }
            _ => panic!("ERROR: Invalid PPU Mode \"{}\"", self.ppu_mode)
        }

        self.dot_counter += 1;
        if self.ppu_mode == PPU_MODE_2_OAM_SCAN && self.dot_counter == 80 {
            self.ppu_mode = PPU_MODE_3_DRAW_PIXELS;
        }
        else if self.ppu_mode == PPU_MODE_3_DRAW_PIXELS && self.dot_counter == self.mode_3_end_index {
            self.ppu_mode = PPU_MODE_0_HBLANK;
            self.mode_3_end_index = 172 + 80;
        }
        else if self.dot_counter == 456 {
            self.dot_counter = 0;
            if self.ly >= 153 {
                self.ly = 0;
                self.ppu_mode = PPU_MODE_2_OAM_SCAN;
            }
            else {
                self.ly += 1; 

                if self.ppu_mode == PPU_MODE_0_HBLANK && self.ly < 144 {
                    self.ppu_mode = PPU_MODE_2_OAM_SCAN;
                }
                else {
                    self.ppu_mode = PPU_MODE_1_VBLANK;
                }
            }
        }
    }
}