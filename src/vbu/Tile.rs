pub struct Tile {
    pub pixels: [[u8; 8]; 8]
}

impl Tile {
    pub fn new(raw_data: [u16; 8]) -> Self {
        let mut temp_pixels = [[0; 8]; 8];
        
        let mut row_num = 0;
        while row_num < 8 {
            let mut col_num = 0;
            while col_num < 8 {
                if raw_data[row_num] & (0x1 << col_num) > 0 {
                    temp_pixels[row_num][(7 as usize) - col_num] += 0b10;
                }

                if raw_data[row_num] & (0x100 << col_num) > 0 {
                    temp_pixels[row_num][(7 as usize) - col_num] += 0b1;
                }

                col_num += 1;
            }

            row_num += 1;
        }

        Self { pixels: temp_pixels }
    }
}