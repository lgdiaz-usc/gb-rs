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
    wy: u8,
    wx: u8,

    //Buffers for the rendering process
    obj_buffer: Vec<u16>,
    bg_fifo: VecDeque<Pixel>,
    obj_fifo: VecDeque<Pixel>,
    screen: [[Pixel; 160]; 144],

    //Misc. variables
    dot_counter: u16, //The current dot on the current scanline;
    mode_3_penalty: u8,
    bg_fetch_state: u8,
    obj_fetch_state: u8,
    fetched_obj_address: u16,
    lx: u8, //The current pixel being pushed in mode 3
    w_ly: u8, //The current line of the window
    w_lx: u8, //The currrent x coordinate of the window
    ly_eq_wy: bool, //Whether or not ly = wy is true at any point in the frame
    is_window_fetching_mode: bool,
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
            wy: 0x00,
            wx: 0x00,
            obj_buffer: Vec::with_capacity(10),
            bg_fifo: VecDeque::with_capacity(8),
            obj_fifo: VecDeque::with_capacity(8),
            screen: [[Pixel {color: 0, palette: None, bg_priority: None, tile: None}; 160]; 144],
            dot_counter: 0,
            mode_3_penalty: 0,
            bg_fetch_state: 0,
            obj_fetch_state: 7,
            fetched_obj_address: 0,
            lx: 0,
            w_ly: 0,
            w_lx: 0,
            ly_eq_wy: false,
            is_window_fetching_mode: false,
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
                0xFF41 => self.stat,
                0xFF42 => self.scy,
                0xFF43 => self.scx,
                0xFF44 => self.ly,
                0xFF45 => self.ly_compare, //LYC
                0xFF4A => self.wy,
                0xFF4B => self.wx,
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
            let mut value = value;

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

                    if !self.lcdc_7_lcd_enabled {
                        self.ly = 0;
                        self.dot_counter = 0;
                        self.ppu_mode = PPU_MODE_0_HBLANK;
                        self.stat &= 0xFC;
                    }
                    return;
                }
                0xFF41 => { //STAT
                    value |= 0x80;
                    &mut self.stat
                }
                0xFF42 => &mut self.scy,
                0xFF43 => &mut self.scx,
                0xFF44 => return, //LY is read only!
                0xFF45 => &mut self.ly_compare, //LYC
                0xFF4A => &mut self.wy,
                0xFF4B => &mut self.wx,
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
                let bg_tile_map_index = self.lcdc_3_bg_tile_map_area as usize;
                let w_tile_map_index = self.lcdc_6_window_tile_map_area as usize;

                if self.bg_fetch_state == 6 {
                    //every 8 pixels (after the initial pixels are pushed), fetch a new tile
                    if self.bg_fifo.is_empty() {
                        if !self.lcdc_0_bg_window_enable {
                            for _ in 0..8 {
                                self.bg_fifo.push_back(Pixel { color: 0, palette: None, bg_priority: None, tile: None });
                            }
                        }
                        else if self.is_window_fetching_mode {
                            let tile_map_offset_x = (self.w_lx >> 3) as usize;
                            let tile_map_offset_y = (((self.w_ly as u16) & 0xF8) << 2) as usize;
                            let tile_index = self.video_ram[0][w_tile_map_index + tile_map_offset_x + tile_map_offset_y];
                            self.bg_fifo = self.tile_fetch_w(tile_index);
                        }
                        else {
                            let tile_map_offset_x = ((self.lx + self.scx) >> 3) as usize;
                            let tile_map_offset_y = (((self.ly as u16 + self.scy as u16) & 0xF8) << 2) as usize;
                            let tile_index = self.video_ram[0][bg_tile_map_index + tile_map_offset_x + tile_map_offset_y];
                            self.bg_fifo = self.tile_fetch_bg(tile_index);

                            if self.lx == 0 {
                                let offset = self.scx & 0b111;

                                for _ in 0..offset {
                                    self.bg_fifo.pop_front();
                                }

                                self.mode_3_penalty += offset;
                            }
                        }

                        self.bg_fetch_state = 0;
                    }
                }
                else {
                    self.bg_fetch_state += 1;
                }

                if self.obj_fetch_state == 7 {
                    //Fetch objects with the same x coordinate as the current pixel
                    for index in 0..self.obj_buffer.len() {
                        let object = self.obj_buffer[index];
                        if self.object_attribute_memory[object as usize + 1] - 8 == self.lx {
                            self.fetched_obj_address = object;
                            self.obj_fetch_state = 0;
                            self.bg_fetch_state = 0;
                            self.mode_3_penalty += 6;
                            self.obj_buffer.remove(index);
                            break;
                        }
                    }
                }
                else if self.obj_fetch_state == 6 {
                    let mut pixel_row = self.tile_fetch_obj(self.fetched_obj_address);
                            
                    if !self.obj_fifo.is_empty() && pixel_row[0].tile == self.obj_fifo[0].tile {
                        self.mode_3_penalty += if self.obj_fifo.len() > 2 {self.obj_fifo.len() as u8 - 2} else {0};
                    }
                    
                    for pixel_index in 0..self.obj_fifo.len() {
                        if self.obj_fifo[pixel_index].color == 0 {
                            self.obj_fifo[pixel_index] = pixel_row.pop_front().unwrap();
                        }
                        else {
                            pixel_row.pop_front();
                        }
                    }
                    self.obj_fifo.extend(pixel_row);
                    self.obj_fetch_state = 7;
                }
                else {
                    self.obj_fetch_state += 1;
                    self.bg_fetch_state = 0;
                }

                if self.mode_3_penalty == 0 {
                    //Pixel Mixing
                    if !self.bg_fifo.is_empty() {
                        let bg_pixel = self.bg_fifo.pop_front().unwrap();
                        let obj_pixel = self.obj_fifo.pop_front();
                        self.screen[self.ly as usize][self.lx as usize] = match obj_pixel {
                            Some(obj_pixel) => {
                                if !self.lcdc_1_obj_enable {
                                    bg_pixel
                                }
                                else if obj_pixel.color == 0 {
                                    bg_pixel
                                }
                                else if obj_pixel.bg_priority.unwrap() && bg_pixel.color != 0 {
                                    bg_pixel
                                }
                                else {
                                    obj_pixel
                                }
                            }
                            None => bg_pixel,
                        };
                        self.lx += 1;
                        if self.is_window_fetching_mode {
                            self.w_lx += 1;
                        } 
                    }
                }
                else {
                    self.mode_3_penalty -= 1;
                }
            }
            _ => panic!("ERROR: Invalid PPU Mode \"{}\"", self.ppu_mode)
        }

        if self.lcdc_7_lcd_enabled {
            self.dot_counter += 1;
            if self.ppu_mode == PPU_MODE_2_OAM_SCAN && self.dot_counter == 80 {
                self.ppu_mode = PPU_MODE_3_DRAW_PIXELS;
            }
            else if self.ppu_mode == PPU_MODE_3_DRAW_PIXELS && self.lx == 160 {
                self.ppu_mode = PPU_MODE_0_HBLANK;
                self.lx = 0;
                self.w_lx = 0;
                if self.is_window_fetching_mode {
                    self.w_ly += 1;
                }
                self.is_window_fetching_mode = false;
                self.obj_buffer.clear();
                self.bg_fifo.clear();
                self.obj_fifo.clear();
            }
            else if self.dot_counter == 456 {
                self.dot_counter = 0;
                if self.ly >= 153 {
                    self.ly = 0;
                    self.w_ly = 0;
                    self.ly_eq_wy = false;
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

            let mut stat = self.stat & 0b11111000;
            if self.ly == self.ly_compare {
                stat |= 0b100;
            }
            stat |= self.ppu_mode;

            self.stat = stat;
        }
    }

    fn tile_row_fetch(&self, tile_index: u8, tile_height: u16, y_flip: bool, x_flip: bool, bank: usize, is_obj: bool) -> VecDeque<u8> {
        let mut tile_row = VecDeque::with_capacity(8);
        let tile_index = match is_obj && self.lcdc_2_obj_is_tall {
            true => tile_index & 0xFE,
            false => tile_index
        };

        let tile_address = match self.lcdc_4_tile_data_area || is_obj {
            true => (tile_index as u16) << 4,
            false => {
                let area_start = if tile_index & 0x80 > 0 {0x8000 - 0x8000} else {0x9000 - 0x8000};
                area_start + ((tile_index as u16) << 4)
            }
        };

        let flip_edge = match is_obj && self.lcdc_2_obj_is_tall {
            true => 30,
            false => 14
        };
        let row_offset = match y_flip {
            true => flip_edge - (tile_height << 1),
            false => tile_height << 1
        };
        let lsb = self.video_ram[bank][(tile_address + row_offset) as usize];
        let msb = self.video_ram[bank][(tile_address + row_offset) as usize + 1];
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
        let tile_height = (self.ly as u16 + self.scy as u16) & 0b111;
        //TODO:: Add support for CGB (BG attribute map support)
        let color_row = self.tile_row_fetch(tile_index, tile_height, false, false, 0, false);
        let mut pixel_row = VecDeque::with_capacity(8);

        for pixel in color_row {
            pixel_row.push_back(Pixel{color: pixel, palette: None, bg_priority: None, tile: None});
        }

        pixel_row
    }

    fn tile_fetch_w(&self, tile_index: u8) -> VecDeque<Pixel> {
        let tile_height = (self.w_ly as u16) & 0b111;
        //TODO:: Add support for CGB (BG attribute map support)
        let color_row = self.tile_row_fetch(tile_index, tile_height, false, false, 0, false);
        let mut pixel_row = VecDeque::with_capacity(8);

        for pixel in color_row {
            pixel_row.push_back(Pixel{color: pixel, palette: None, bg_priority: None, tile: None});
        }

        pixel_row
    }

    fn tile_fetch_obj(&self, oam_index: u16) -> VecDeque<Pixel> {
        let tile_height = self.ly as u16 - (self.object_attribute_memory[oam_index as usize] - 16) as u16;
        let tile_index = self.object_attribute_memory[oam_index as usize + 2];
        let obj_attributes = self.object_attribute_memory[oam_index as usize + 3];
        let y_flip = obj_attributes & 0b1000000 > 0;
        let x_flip = obj_attributes & 0b100000 > 0;
        //TODO: Add support for CGB (VRMA bank and palette support)
        let color_row = self.tile_row_fetch(tile_index, tile_height, y_flip, x_flip, 0, true);

        let mut pixel_row = VecDeque::with_capacity(8);
        let bg_priority = obj_attributes & 0b10000000 > 0;
        let palette = (obj_attributes & 0b10000) >> 4;
        let tile = (self.object_attribute_memory[oam_index as usize + 1] - 8 + self.scx) & 0b11111000;


        for pixel in color_row {
            pixel_row.push_back(Pixel{color: pixel, palette: Some(palette), bg_priority: Some(bg_priority), tile: Some(tile)});
        }

        pixel_row
    }

    pub fn get_mode(&self) -> u8 {
        self.ppu_mode
    }

    pub fn has_entered_vblank(&self) -> bool {
        self.ly == 144 && self.dot_counter == 0
    }

    pub fn dma_transfer(&mut self, value: u8, address: u8) {
        self.object_attribute_memory[address as usize] = value;
    }

    pub fn dump_screen(&self) -> &[[Pixel; 160]; 144] {
        &self.screen
    }
}

#[derive(Clone,Copy)]
pub struct Pixel {
    pub color: u8,
    pub palette: Option<u8>,
    bg_priority: Option<bool>,
    tile: Option<u8>
}