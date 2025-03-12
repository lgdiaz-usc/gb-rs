use std::collections::VecDeque;

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
    lcdc_4_tile_data_area: bool, //0: 0x8800, 1: 0x8000
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
    bg_fifo: VecDeque<Pixel>,
    obj_fifo: VecDeque<Pixel>,

    //Misc. variables
    dot_counter: u16, //The current dot on the current scanline;
    lx: u8, //The current pixel being pushed in mode 3
    mode_3_penalty: u8,
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
            lcdc_4_tile_data_area: false,
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
            bg_fifo: VecDeque::with_capacity(8),
            obj_fifo: VecDeque::with_capacity(8),
            dot_counter: 0,
            lx: 0,
            mode_3_penalty: 0,
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
                    if self.lcdc_4_tile_data_area {
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
                    self.lcdc_4_tile_data_area = value & 16 > 0;
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
                    let obj_y = self.object_attribute_memory[obj_address as usize] - 16;

                    let obj_height = match self.lcdc_2_obj_is_tall {
                        true => 16,
                        false => 8
                    };
                    if self.ly >= obj_y && self.ly < obj_height + obj_y {
                        self.obj_buffer.push(obj_address);
                    }
                }
            }
            PPU_MODE_3_DRAW_PIXELS => {
                if self.mode_3_penalty == 0 {
                    let tile_map_x = (self.scx >> 3) as u16;
                    let tile_map_y = ((self.scy & 0b11111000) as u16) << 2;
                    
                    //TODO: Implement Window fetching
                    //If at the beginning of a scanline, fetch pixels from tile cut off by scx
                    if self.lx == 0 {
                        self.mode_3_penalty = self.scx & 0b111;
                        let tile_index = self.video_ram[self.video_ram_index][(self.lcdc_3_bg_tile_map_area + tile_map_x + tile_map_y) as usize];
                        let mut pixel_row = self.tile_fetch_bg(tile_index);

                        for _ in 0..self.mode_3_penalty {
                            pixel_row.pop_front();
                        }
                        self.bg_fifo.extend(pixel_row)
                    }
                    //every 8 pixels (after the initial pixels are pushed), fetch a new tile
                    else if (self.lx + self.scx) & 0b111 == 0 {
                        let tile_index = self.video_ram[self.video_ram_index][(self.lcdc_3_bg_tile_map_area + tile_map_x + tile_map_y) as usize];
                        let mut pixel_row = self.tile_fetch_bg(tile_index);
                        self.bg_fifo.extend(pixel_row)
                    }


                }
                else {
                    self.mode_3_penalty -= 1;
                }
            }
            _ => panic!("ERROR: Invalid PPU Mode \"{}\"", self.ppu_mode)
        }

        self.dot_counter += 1;
        if self.ppu_mode == PPU_MODE_2_OAM_SCAN && self.dot_counter == 80 {
            self.ppu_mode = PPU_MODE_3_DRAW_PIXELS;
        }
        else if self.ppu_mode == PPU_MODE_3_DRAW_PIXELS && self.lx == 160 {
            self.ppu_mode = PPU_MODE_0_HBLANK;
            self.lx = 0;
            self.obj_buffer.clear();
            self.bg_fifo.clear();
            self.obj_fifo.clear();
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

    fn tile_row_fetch(&self, tile_index: u8, y_flip: bool, x_flip: bool) -> VecDeque<u8> {
        let mut tile_row = VecDeque::with_capacity(8);

        let tile_address = match self.lcdc_4_tile_data_area {
            true => (tile_index as u16) << 4,
            false => {
                let area_start = if tile_index & 0x80 > 0 {0x8000 - 0x8000} else {0x9000 - 0x8000};
                area_start + ((tile_index as u16) << 4)
            }
        };

        let row_offset = match y_flip {
            true => 14 - (((self.scy & 0b111) as u16 + self.ly as u16) << 1),
            false => ((self.scy & 0b111) as u16 + self.ly as u16) << 1
        };
        let lsb = self.video_ram[self.video_ram_index][(tile_address + row_offset) as usize];
        let msb = self.video_ram[self.video_ram_index][(tile_address + row_offset) as usize + 1];
        for bit in 0..8 {
            let mut pixel = 0;
            if lsb & (1 << bit) > 0 {
                pixel |= 0b1;
            }
            if msb & (1 << bit) > 0 {
                pixel |= 0b10;
            }

            match x_flip {
                true => tile_row.push_back(pixel),
                false => tile_row.push_front(pixel),
            }
        }

        tile_row
    }

    fn tile_fetch_bg(&self, tile_index: u8) -> VecDeque<Pixel> {
        let color_row = self.tile_row_fetch(tile_index, false, false);
        let mut pixel_row = VecDeque::with_capacity(8);

        for pixel in color_row {
            pixel_row.push_back(Pixel{color: pixel, palette: Option::None, bg_priority: Option::None});
        }

        pixel_row
    }
}

struct Pixel {
    color: u8,
    palette: Option<u8>,
    bg_priority: Option<bool>
}