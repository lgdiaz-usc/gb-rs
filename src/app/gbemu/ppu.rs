pub struct PPU {
    //Memory
    video_ram: Vec<[u8; 0x4000]>,
    video_ram_index: usize,
    object_attribute_memory: [u8; 0xA0],
}

impl PPU {
    pub fn new() -> Self {
        let mut video_ram = Vec::new();
        video_ram.push([0; 0x4000]);

        Self {
            video_ram: video_ram,
            video_ram_index: 0,
            object_attribute_memory: [0; 0xA0],
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
            0
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
            
        }
        else {
            panic!("ERROR: Address out of bounds!")
        }
    }
}