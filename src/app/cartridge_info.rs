#[derive(Clone)]
pub enum CGBState {
    Monochrome,
    Color,
    Both
}

#[derive(Clone)]
pub struct CartridgeInfo {
    pub title: String,
    pub manufacturer_code: String,
    pub cgb_flag: CGBState,
    pub is_sgb: bool,
    pub licensee: String,
    pub cartridge_type: u8,
    pub rom_size: usize,
    pub rom_banks: usize,
    pub ram_size: usize,
    pub ram_banks: usize,
    pub overseas_only: bool,
    pub version_number: u8,
    pub header_checksum: u8,
    pub global_checksum: u16
}

impl CartridgeInfo {
    pub fn new(header: &[u8]) -> Self {
        let title = std::str::from_utf8(&header[..16]).expect("Invalid String").to_ascii_uppercase();
        let manufacturer_code = std::str::from_utf8(&header[11..15]).expect("Invalid String").to_ascii_uppercase();
        let cgb_flag = match header[15] {
            0x80 => CGBState::Both,
            0xC0 => CGBState::Color,
            _ => CGBState::Monochrome
        };
    
        let mut licensee: String;
        if header[23] == 33 {
            let code_digit_1 = (header[16] as char).to_ascii_uppercase();
            let code_digit_2 = (header[17] as char).to_ascii_uppercase();
            licensee = match code_digit_1 {
                '0' => match code_digit_2 {
                    '0' => "None",
                    '1' => "Nintendo Research & Development 1",
                    '8' => "Capcom",
                    _ => ""
                }
                '1' => match code_digit_2 {
                    '3' => "EA (Electronic Arts)",
                    '8' => "Hudson Soft",
                    '9' => "B-AI",
                    _ => ""
                }
                '2' => match code_digit_2 {
                    '0' => "KSS",
                    '2' => "Planning Office WADA",
                    '4' => "PCM Complete",
                    '5' => "San-X",
                    '8' => "Kemco",
                    '9' => "SETA Corporation",
                    _ => ""
                }
                '3' => match code_digit_2 {
                    '0' => "Viacom",
                    '1' => "Nintendo",
                    '2' => "Bandai",
                    '3' => "Ocean Software/Acclaim Entertainment",
                    '4' => "Konami",
                    '5' => "HectorSoft",
                    '7' => "Taito",
                    '8' => "Hudson Soft",
                    '9' => "Banpresto",
                    _ => ""
                }
                '4' => match code_digit_2 {
                    '1' => "Uni Soft",
                    '2' => "Atlus",
                    '4' => "Malibu Interactive",
                    '6' => "Angel",
                    '7' => "Bullet-Proof Software",
                    '9' => "Irem",
                    _ => ""
                }
                '5' => match code_digit_2 {
                    '0' => "Absolute",
                    '1' => "Acclaim Entertainment",
                    '2' => "Activision",
                    '3' => "Sammy USA Corporation",
                    '4' => "Konami",
                    '5' => "Hi Tech Expressions",
                    '6' => "LJN",
                    '7' => "Matchbox",
                    '8' => "Mattel",
                    '9' => "Milton Bradley Company",
                    _ => ""
                }
                '6' => match code_digit_2  {
                    '0' => "Titus Interactive",
                    '1' => "Virgin Games Ltd.",
                    '4' => "Lucasfilm Games",
                    '7' => "Ocean Software",
                    '9' => "EA (Electronic Arts)",
                    _ => ""
                }
                '7' => match code_digit_2 {
                    '0' => "Infogrames",
                    '1' => "Interplay Entertainment",
                    '2' => "Broderbund",
                    '3' => "Sculptured Software",
                    '5' => "The Sales Curve Limited",
                    '8' => "THQ",
                    '9' => "Accolade",
                    _ => ""
                }
                '8' => match code_digit_2 {
                    '0' => "Misawa Entertainment",
                    '3' => "lozc",
                    '6' => "Tokuma Shoten",
                    '7' => "Tsukuda Original",
                    _ => ""
                }
                '9' => match code_digit_2 {
                    '1' => "Chunsoft Co.",
                    '2' => "Video System",
                    '3' => "Ocean Software/Acclaim Entertainment",
                    '5' => "Varie",
                    '6' => "Yonezawa/s’pal",
                    '7' => "Kaneko",
                    '9' => "Pack-In-Video",
                    'H' => "Bottom Up",
                    _ => ""
                }
                'A' => match code_digit_2 {
                    '4' => "Konami (Yu-Gi-Oh!)",
                    _ => ""
                }
                'B' => match code_digit_2 {
                    'L' => "MTO",
                    _ => ""
                }
                'D' => match code_digit_2 {
                    'K' => "Kodansha",
                    _ => ""
                }
                _ => ""
            }.to_string();
        }
        else {
            let old_licensees = ["None", "Nintendo", "", "", "", "", "", "", "Capcom", "HOT-B", "Jaleco", "Coconuts Japan", "Elite Systems", "", "", "", "", "", "", "EA (Electronic Arts)", "", "", "", "", "", "Hudson Soft", "ITC Entertainment", "Yanoman", "", "", "Japan Clary", "", "Virgin Games Ltd.", "", "", "", "", "PCM Complete", "San-X", "", "", "Kemco", "SETA Corporation", "", "", "", "", "", "", "Infogrames", "Nintendo", "Bandai", "", "Konami", "HectorSoft", "", "", "Capcom", "Banpresto", "", "", "Entertainment Interactive", "", "Gremlin", "", "", "Ubi Soft", "Atlus", "", "Malibu Interactive", "", "Angel", "Spectrum HoloByte", "", "Irem", "Virgin Games Ltd.", "", "", "Malibu Interactive", "", "U.S. Gold", "Absolute", "Acclaim Entertainment", "Activision", "Sammy USA Corporation", "GameTek", "Park Place", "LJN", "Matchbox", "", "Milton Bradley Company", "Mindscape", "Romstar", "Naxat Soft", "Tradewest", "", "", "Titus Interactive", "Virgin Games Ltd.", "", "", "", "", "", "Ocean Software", "", "EA (Electronic Arts)", "", "", "", "", "Elite Systems", "Electro Brain", "Infogrames", "Interplay Entertainment", "Broderbund", "Sculptured Software", "", "The Sales Curve Limited", "", "", "THQ", "Accolade", "Triffix Entertainment", "", "MicroProse", "", "", "Kemco", "Misawa Entertainment", "", "", "LOZC G.", "", "", "Tokuma Shoten", "", "", "", "", "Bullet-Proof Software", "Vic Tokai Corp.", "", "Ape Inc.", "I’Max", "Chunsoft Co.", "Video System", "Tsubaraya Productions", "", "Varie", "Yonezawa/S’Pal", "Kemco", "", "Arc", "Nihon Bussan", "Tecmo", "Imagineer", "Banpresto", "", "Nova", "", "Hori Electric", "Bandai", "", "Konami", "", "Kawada", "Takara", "", "Technos Japan", "Broderbund", "", "Toei Animation", "Toho", "", "Namco", "Acclaim Entertainment", "ASCII Corporation or Nexsoft", "Bandai", "", "Square Enix", "", "HAL Laboratory", "SNK", "", "Pony Canyon", "Culture Brain", "Sunsoft", "", "Sony Imagesoft", "", "Sammy Corporation", "Taito", "", "Kemco", "Square", "Tokuma Shoten", "Data East", "Tonkin House", "", "Koei", "UFL", "Ultra Games", "VAP, Inc.", "Use Corporation", "Meldac", "Pony Canyon", "Angel", "Taito", "SOFEL (Software Engineering Lab)", "Quest", "Sigma Enterprises", "ASK Kodansha Co.", "", "Naxat Soft", "Copya System", "", "Banpresto", "Tomy", "LJN", "", "Nippon Computer Systems", "Human Ent.", "Altron", "Jaleco", "Towa Chiki", "Yutaka", "Varie", "", "Epoch", "", "Athena", "Asmik Ace Entertainment", "Natsume", "King Records", "Atlus", "Epic/Sony Records", "", "IGS", "", "A Wave", "", "", "Extreme Entertainment", "", "", "", "", "", "", "", "", "", "", "", "LJN"];
            licensee = old_licensees[header[23] as usize].to_string();
        }
        if licensee == "" {
            licensee = "Unkown Licensee".to_owned();
        }

        let is_sgb = header[18] == 0x03;
        let cartridge_type = header[19];
        let rom_size: usize = 0x8000 * (1 << header[20]);
        let rom_banks: usize = 0b10 << header[20];
        let (ram_size, ram_banks) = match header[21] {
            0 => (0,0),
            2 => (0x2000, 1),
            3 => (0x8000, 4),
            4 => (0x20000, 16),
            5 => (0x10000, 8),
            _ => panic!("Invalid RAM Size!")
        };
        let overseas_only = header[22] & 0b1 > 0;
        let version_number = header[23];
        let header_checksum = header[24];
        let global_checksum = ((header[25] as u16) << 8) + header[26] as u16;

        Self {title: title, manufacturer_code: manufacturer_code, cgb_flag: cgb_flag, licensee: licensee, is_sgb: is_sgb, cartridge_type: cartridge_type, rom_size: rom_size, rom_banks: rom_banks, ram_size: ram_size, ram_banks: ram_banks, overseas_only: overseas_only, version_number: version_number, header_checksum: header_checksum, global_checksum: global_checksum}
    }
}
