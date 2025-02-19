mod mapper;

mod no_mbc;
mod mbc1; //TODO: Make separate struct for MBC1M Cartridges

pub use self::{
    mapper::Mapper,
    no_mbc::NoMBC,
    mbc1::MBC1,
};