use std::{fs::File, io::{BufWriter, Seek, Write}, sync::mpsc::Receiver};

use super::{MBC1, NoMBC};

pub enum Mapper {
    NoMBC(NoMBC),
    MBC1(MBC1),
}

impl Mapper {
    pub fn read(&self, address: u16) -> u8 {
        match self {
            Self::NoMBC(mapper) => mapper.read(address),
            Self::MBC1(mapper) => mapper.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match self {
            Self::NoMBC(mapper) => mapper.write(address, value),
            Self::MBC1(mapper) => mapper.write(address, value),
        }
    }
}

fn write_thread(mut file: BufWriter<File>, data_receiver: Receiver<(u8, u64)>) {
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
}