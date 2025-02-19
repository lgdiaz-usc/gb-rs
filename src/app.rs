use std::sync::atomic::Ordering;

pub mod gbemu;
pub mod cartridge_info;
pub use cartridge_info::CGBState;


impl eframe::App for gbemu::GBEmu {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Rom").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("GameBoy Roms", &["gb", "gbc"]).pick_file() {
                            let mut lock = self.rom_file_path.lock().unwrap();
                            *lock = Some(path.display().to_string());
                            drop(lock);
                            self.file_changed.store(true, Ordering::Relaxed);
                        }
                    }
                    // NOTE: no File->Quit on web pages!
                    let is_web = cfg!(target_arch = "wasm32");
                    if !is_web {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            /*let top = ctx.available_rect().top();
            let left = ctx.available_rect().left();
            let painter = ui.painter();

            painter.rect(egui::Rect { min: egui::pos2(0.0 + left, 10.0 + top), max: egui::pos2(100.0 + left, 100.0 + top) }, egui::Rounding::ZERO, egui::Color32::RED, egui::Stroke::NONE);
*/
            let lock = self.rom_file_path.lock().unwrap();
            if let Some(picked_path) = lock.clone() {
                ui.horizontal(|ui| {
                    ui.label("Loaded Rom: ");
                    ui.monospace(picked_path);
                });
            }
            else {
                ui.horizontal(|ui| {
                    ui.label("No rom detected!");
                });
            }
            drop(lock);
            let lock = self.rom_info.lock().unwrap();
            if let Some(info) = lock.clone() {
                ui.horizontal(|ui| {
                    ui.label("Title: ");
                    ui.monospace(info.title);
                });
                ui.horizontal(|ui| {
                    ui.label("Manufacturer Code: ");
                    ui.monospace(info.manufacturer_code);
                });
                ui.horizontal(|ui| {
                    ui.label("Gameboy Color Compatibility: ");
                    ui.monospace(match info.cgb_flag {
                        CGBState::Monochrome => "GameBoy only",
                        CGBState::Color => "GameBoy Color only",
                        CGBState::Both => "Gameboy Color enhancement supported"
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Super GameBoy support: ");
                    ui.monospace(format!("{}", info.is_sgb));
                });
                ui.horizontal(|ui| {
                    ui.label("Licensee: ");
                    ui.monospace(info.licensee);
                });
                ui.horizontal(|ui|{
                    ui.label("Mapper Code:");
                    ui.label(format!("{}", info.cartridge_type));
                });
                ui.horizontal(|ui| {
                    ui.label("Rom Size: ");
                    ui.label(format!("{} bytes ({} banks)", info.rom_size, info.rom_banks));
                });
                ui.horizontal(|ui| {
                    ui.label("Ram Size: ");
                    ui.label(format!("{} bytes ({} banks)", info.ram_size, info.ram_banks));
                });
                ui.horizontal(|ui| {
                    ui.label("Can be sold in Japan: ");
                    ui.monospace(format!("{}", info.overseas_only));
                });
                ui.horizontal(|ui| {
                    ui.label("Version: ");
                    ui.monospace(format!("{}", info.version_number));
                });
                ui.horizontal(|ui| {
                    ui.label("Header Checksum: ");
                    ui.monospace(format!("{}", info.header_checksum));
                });
                ui.horizontal(|ui| {
                    ui.label("Global Chacksum: ");
                    ui.monospace(format!("{}", info.global_checksum));
                });
            }
        });
    }
}

/*fn parse_tile(tile: Tile) {
    for row in tile.pixels {
        for pixel in row {
            match pixel {
                0 => print!("."),
                1 => print!("░"),
                2 => print!("▒"),
                3 => print!("▓"),
                _ => print!("?")
            }
        }
        println!();
    }
    println!("\n");
}*/
