use std::{fs::File, io::{BufWriter, Seek, Write}, sync::mpsc::Receiver, thread};

pub trait Mapper {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub fn write_thread(mut file: BufWriter<File>, data_receiver: Receiver<(u8, u64)>) {
    thread::spawn(move || {
        loop {
            if let Ok((value, address)) = data_receiver.recv() {
                if address != file.stream_position().unwrap() {
                    file.seek(std::io::SeekFrom::Start(address)).unwrap();
                }

                file.write(&[value]).unwrap();
            }
            else {
                file.flush().unwrap();
                return;
            }
        }
    });
}

pub fn rom_to_save(rom_file_path: String) -> String {
    if let Some(ram_file_path) = rom_file_path.rsplitn(2, ".").last() {
        ram_file_path.to_owned() + ".sav"
    }
    else {
        panic!("Error! Invalid file path");
    }
}