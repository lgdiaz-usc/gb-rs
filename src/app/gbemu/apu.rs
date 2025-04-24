use std::{sync::mpsc::{channel, Receiver, Sender}, thread};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, FromSample, Sample, SizedSample};

pub struct APU {
    //Channel 1 registers
    ch_1_0_sweep: u8,   //NR10
    ch_1_1_length: u8,  //NR11
    ch_1_2_volume: u8,  //NR12
    ch_1_3_period: u16,     //NR13
    ch_1_4_length_enable: bool,    //NR14

    //Channel 2 registers
    ch_2_1_length: u8,   //NR21
    ch_2_2_volume: u8,   //NR22
    ch_2_3_period: u16,      //NR23
    ch_2_4_length_enable: bool,     //NR24

    //Channel 3 registers
    ch_3_0_enable: bool, //NR30
    ch_3_1_length: u8,   //NR31
    ch_3_2_volume: u8,   //NR32
    ch_3_3_period: u16,      //NR33
    ch_3_4_length_enable: bool,     //NR34

    //Channel 4 registers
    ch_4_1_length: u8,   //NR41
    ch_4_2_volume: u8,   //NR42
    ch_4_3_freq: u8,     //NR43
    ch_4_4_length_enable: bool,  //NR44

    //Master control registers
    ch_5_0_volume: u8,   //NR50
    ch_5_1_panning: u8,  //NR51
    ch_5_2_enable: bool,   //NR52

    //Channel Enable flags
    ch_1_enable: bool,
    ch_2_enable: bool,
    ch_3_enable: bool,
    ch_4_enable: bool,

    //DAC Enable falgs
    dac_1_enable: bool,
    dac_2_enable: bool,
    dac_3_enable: bool,
    dac_4_enable: bool,

    //Wave RAM
    wave_ram: [u8; 16],

    //Timer for the APU
    apu_counter: u16, //DIV-APU

    //Internal APU registers
    //Channel 2
    ch_2_duty_counter: u8,
    ch_2_period_counter: u16,
    ch_2_length_counter: u8,
    ch_2_envelope_counter: u8,
    ch_2_volume: u8,

    //Variables for sending data to audio library
    sample_data: SampleData,
    sender: Sender<SampleData>,
}

impl APU {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        thread::spawn(move || {
            Self::init_device(receiver);
        });

        Self {
            ch_1_0_sweep: 0x80,
            ch_1_1_length: 0xBF,
            ch_1_2_volume: 0xF3,
            ch_1_3_period: 0x0000,
            ch_1_4_length_enable: true,
            ch_2_1_length: 0x3F,
            ch_2_2_volume: 0x00,
            ch_2_3_period: 0x0000,
            ch_2_4_length_enable: true,
            ch_3_0_enable: false,
            ch_3_1_length: 0xFF,
            ch_3_2_volume: 0x9F,
            ch_3_3_period: 0x00,
            ch_3_4_length_enable: true,
            ch_4_1_length: 0xFF,
            ch_4_2_volume: 0x00,
            ch_4_3_freq: 0x00,
            ch_4_4_length_enable: true,
            ch_5_0_volume: 0x77,
            ch_5_1_panning: 0xF3,
            ch_5_2_enable: true,
            wave_ram: [0; 16],
            ch_1_enable: false,
            ch_2_enable: false,
            ch_3_enable: false,
            ch_4_enable: false,
            dac_1_enable: false,
            dac_2_enable: false,
            dac_3_enable: false,
            dac_4_enable: false,
            ch_2_duty_counter: 0,
            ch_2_envelope_counter: 0,
            ch_2_length_counter: 0,
            ch_2_period_counter: 0,
            ch_2_volume: 0,
            apu_counter: 0,
            sample_data: SampleData::default(),
            sender
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0xFF10 && address <= 0xFF26 {
            match address {
                0xFF10 => self.ch_1_0_sweep,
                0xFF11 => self.ch_1_1_length | 0b111111,
                0xFF12 => self.ch_1_2_volume,
                0xFF13 => 0xFF,
                0xFF14 => if self.ch_1_4_length_enable {0xFF} else {0xBF},
                0xFF16 => self.ch_2_1_length | 0b111111,
                0xFF17 => self.ch_2_2_volume,
                0xFF18 => 0xFF,
                0xFF19 => if self.ch_2_4_length_enable {0xFF} else {0xBF},
                0xFF1A => if self.ch_3_0_enable {0xFF} else {0x7F},
                0xFF1B => 0xFF,
                0xFF1C => self.ch_3_2_volume,
                0xFF1D => 0xFF,
                0xFF1E => if self.ch_2_4_length_enable {0xFF} else {0xBF},
                0xFF20 => 0xFF,
                0xFF21 => self.ch_4_2_volume,
                0xFF22 => self.ch_4_3_freq,
                0xFF23 => if self.ch_4_4_length_enable {0xFF} else {0xBF},
                0xFF24 => self.ch_5_0_volume,
                0xFF25 => self.ch_5_1_panning,
                0xFF26 => {
                    let mut value = 0b1110000;
                    if self.ch_5_2_enable {
                        value |= 0b10000000;
                    }
                    if self.ch_4_enable {
                        value |= 0b1000;
                    }
                    if self.ch_3_enable {
                        value |= 0b100;
                    }
                    if self.ch_2_enable {
                        value |= 0b10;
                    }
                    if self.ch_1_enable {
                        value |= 0b1
                    }
                    value
                },
                _ => {
                    println!("ERROR: Unknown register ${:x}", address);
                    0xFF
                }
            }
        }
        else if address >= 0xFF30 && address <= 0xFF3F {
            self.wave_ram[(address - 0xFF30) as usize]
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address >= 0xFF10 && address <= 0xFF26 {
            let mut value = value;
            let register = match address {
                0xFF10 => {
                    value |= 0b10000000;
                    &mut self.ch_1_0_sweep
                },
                0xFF11 => &mut self.ch_1_1_length,
                0xFF12 => &mut self.ch_1_2_volume,
                0xFF13 => {
                    self.ch_1_3_period &= 0xFF00;
                    self.ch_1_3_period |= value as u16;
                    return;
                },
                0xFF14 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 1
                    }

                    self.ch_1_4_length_enable = value & 0x40 != 0;
                    self.ch_1_3_period = (self.ch_1_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF16 => &mut self.ch_2_1_length,
                0xFF17 => {
                    self.dac_2_enable = value & 0xF8 != 0;
                    self.ch_2_enable = self.dac_2_enable;

                    &mut self.ch_2_2_volume
                },
                0xFF18 => {
                    self.ch_2_3_period &= 0xFF00;
                    self.ch_2_3_period |= value as u16;
                    return;
                },
                0xFF19 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 2
                        self.ch_2_enable = true;
                        self.ch_2_length_counter = self.ch_2_1_length & 0x3F;
                        self.ch_2_period_counter = self.ch_2_3_period;
                        self.ch_2_envelope_counter = 0;
                        self.ch_2_volume = self.ch_2_2_volume >> 4;
                    }

                    self.ch_2_4_length_enable = value & 0x40 != 0;
                    self.ch_2_3_period = (self.ch_2_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF1A => {
                    self.ch_3_0_enable = value & 0x80 != 0;
                    return;
                },
                0xFF1B => &mut self.ch_3_1_length,
                0xFF1C => {
                    value |= 0b1100000;
                    &mut self.ch_3_2_volume
                },
                0xFF1D => {
                    self.ch_3_3_period &= 0xFF00;
                    self.ch_3_3_period |= value as u16;
                    return;
                },
                0xFF1E => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 3
                    }

                    self.ch_3_4_length_enable = value & 0x40 != 0;
                    self.ch_3_3_period = (self.ch_3_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF20 => {
                    value |= 0b11000000;
                    &mut self.ch_4_1_length
                },
                0xFF21 => &mut self.ch_4_2_volume,
                0xFF22 => &mut self.ch_4_3_freq,
                0xFF23 => {
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 4
                    }

                    self.ch_4_4_length_enable = value & 0x40 != 0;
                    return;
                },
                
                0xFF24 => &mut self.ch_5_0_volume,
                0xFF25 => &mut self.ch_5_1_panning,
                0xFF26 => {
                    self.ch_5_2_enable = value & 0x80 != 0;
                    return;
                },
                _ => panic!("ERROR: Unknown register ${:x}", address)
            };

            *register = value;
        }
        else if address >= 0xFF30 && address <= 0xFF3F {
            self.wave_ram[(address - 0xFF30) as usize] = value;
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    pub fn init_device(receiver: Receiver<SampleData>) {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("ERROR: failed to find output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::I8 => Self::run::<i8>(receiver, &device, &config.into()),
            cpal::SampleFormat::I16 => Self::run::<i16>(receiver, &device, &config.into()),
            //cpal::SampleFormat::I24 => Self::run::<I24>(receiver, &device, &config.into()),
            cpal::SampleFormat::I32 => Self::run::<i32>(receiver, &device, &config.into()),
            //cpal::SampleFormat::I48 => Self::run::<I48>(receiver, &device, &config.into()),
            cpal::SampleFormat::I64 => Self::run::<i64>(receiver, &device, &config.into()),
            cpal::SampleFormat::U8 => Self::run::<u8>(receiver, &device, &config.into()),
            cpal::SampleFormat::U16 => Self::run::<u16>(receiver, &device, &config.into()),
            //cpal::SampleFormat::U24 => Self::run::<U24>(receiver, &device, &config.into()),
            cpal::SampleFormat::U32 => Self::run::<u32>(receiver, &device, &config.into()),
            //cpal::SampleFormat::U48 => Self::run::<U48>(receiver, &device, &config.into()),
            cpal::SampleFormat::U64 => Self::run::<u64>(receiver, &device, &config.into()),
            cpal::SampleFormat::F32 => Self::run::<f32>(receiver, &device, &config.into()),
            cpal::SampleFormat::F64 => Self::run::<f64>(receiver, &device, &config.into()),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        }
    }

    fn run<T>(receiver: Receiver<SampleData>,device: &cpal::Device, config: &cpal::StreamConfig)
    where 
        T: SizedSample + FromSample<f32>,
    {
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        let mut current_sample_data = SampleData::default();

        let mut sample_clock = 0.0;
        let mut next_value = move || {
            if let Ok(data) = receiver.try_recv() {
                current_sample_data = data;
            }

            sample_clock = (sample_clock + 1.0) % sample_rate;

            (sample_clock * 440.0 * 2.0 * current_sample_data.test * std::f32::consts::PI / sample_rate).sin()
        };

        let err_fn = |err| eprintln!("An error occurred on stream: {}", err);

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::write_data(data, channels, &mut next_value)
            },
            err_fn,
            None,
        ).unwrap();
        stream.play().unwrap();

        loop {}
    }

    fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
    where
        T: Sample + FromSample<f32>,
    {
        for frame in output.chunks_mut(channels) {
            let value: T = T::from_sample(next_sample());
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }

    pub fn update_apu_timer(&mut self) {
        self.apu_counter += 1;

        //TODO: Implement events that occur every N DIV-APU ticks
    }
}

#[derive(Clone,Copy)]
pub struct SampleData {
    test: f32
}

impl Default for SampleData {
    fn default() -> Self {
        Self { 
            test: 0.5
        }
    }
}