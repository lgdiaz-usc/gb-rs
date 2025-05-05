use std::{sync::mpsc::{channel, Receiver, Sender}, thread};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, FromSample, Sample, SizedSample};

pub struct APU {
    //Channel 2 registers
    ch_2_1_length: u8,   //NR21
    ch_2_2_volume: u8,   //NR22
    ch_2_3_period: u16,      //NR23
    ch_2_4_length_enable: bool,     //NR24

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
    dac_2_enable: bool,

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
    ch_2_envelope_increases: bool,
    ch_2_sweep_pace: u8,
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
            ch_2_1_length: 0x3F,
            ch_2_2_volume: 0x00,
            ch_2_3_period: 0x0000,
            ch_2_4_length_enable: true,
            ch_5_0_volume: 0x77,
            ch_5_1_panning: 0xF3,
            ch_5_2_enable: true,
            wave_ram: [0; 16],
            ch_1_enable: false,
            ch_2_enable: false,
            ch_3_enable: false,
            ch_4_enable: false,
            dac_2_enable: false,
            ch_2_duty_counter: 0,
            ch_2_envelope_counter: 0,
            ch_2_envelope_increases: false,
            ch_2_sweep_pace: 0,
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
                0xFF16 => self.ch_2_1_length | 0b111111,
                0xFF17 => self.ch_2_2_volume,
                0xFF18 => 0xFF,
                0xFF19 => if self.ch_2_4_length_enable {0xFF} else {0xBF},
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
                    //println!("ERROR: Unknown register ${:x}", address);
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
            //let mut value = value;
            let register = match address {
                0xFF16 => &mut self.ch_2_1_length,
                0xFF17 => {
                    self.dac_2_enable = value & 0xF8 != 0;
                    if !self.dac_2_enable {
                        self.disable_ch_2();
                    }

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
                        if self.ch_2_length_counter == 64 {
                            self.ch_2_length_counter = self.ch_2_1_length & 0x3F;
                        }
                        self.ch_2_period_counter = 0x7FF;
                        self.ch_2_envelope_counter = 0;
                        self.ch_2_volume = self.ch_2_2_volume >> 4;
                        self.ch_2_envelope_increases = self.ch_2_2_volume & 0b1000 != 0;
                        self.ch_2_sweep_pace = self.ch_2_2_volume & 0b111;
                        self.sample_data.ch_2_amp = digital_to_analog(self.ch_2_volume);
                    }

                    self.ch_2_4_length_enable = value & 0x40 != 0;
                    self.ch_2_3_period = (self.ch_2_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF24 => &mut self.ch_5_0_volume,
                0xFF25 => &mut self.ch_5_1_panning,
                0xFF26 => {
                    self.ch_5_2_enable = value & 0x80 != 0;
                    if !self.ch_5_2_enable {
                        //TODO: Disable other channels
                        self.disable_ch_2();
                    }
                    return;
                },
                _ => {
                    //println!("ERROR: Unknown register ${:x}", address);
                    return;
                }
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

    fn disable_ch_2(&mut self) {
        self.ch_2_enable = false;
        self.ch_2_envelope_counter = 0;
        self.ch_2_length_counter = 0;
        self.ch_2_period_counter = 0;
        self.ch_2_volume = 0;
        self.sample_data.ch_2_freq = 0.0;
        self.sender.send(self.sample_data.clone()).unwrap();
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

        let mut ch_2_sample_ratio = 0.0;

        let mut sample_clock = 0.0;
        let mut next_value = move || {
            if let Ok(data) = receiver.try_recv() {
                current_sample_data = data;
                ch_2_sample_ratio = if current_sample_data.ch_2_freq == 0.0 {
                    0.0
                }
                else {
                    sample_rate / current_sample_data.ch_2_freq
                }
            }

            sample_clock = (sample_clock + 1.0) % sample_rate;

            let mut mixed_sample = 0.0;

            //TODO: Implement other channels

            // Add channel 2
            mixed_sample += if (sample_clock % ch_2_sample_ratio) / ch_2_sample_ratio <= current_sample_data.ch_2_duty {current_sample_data.ch_2_amp} else {0.0};

            //TODO: implement Stereo, master volume, and HPF
            mixed_sample
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
        let apu_counter_before = self.apu_counter;
        self.apu_counter += 1;

        //TODO: Implement events that occur every N DIV-APU ticks
        let will_update_envelope;
        {
            let state_before = apu_counter_before & 0b1000 != 0;
            let state_after = self.apu_counter & 0b1000 != 0;
            will_update_envelope = state_before && !state_after;
        }
        if will_update_envelope && self.ch_2_sweep_pace != 0 {
            self.ch_2_envelope_counter += 1;
            if self.ch_2_envelope_counter == self.ch_2_sweep_pace {
                if self.ch_2_envelope_increases {
                    self.ch_2_volume += 1;
                }
                else {
                    self.ch_2_volume -= 1;
                }
                self.sample_data.ch_2_amp = digital_to_analog(self.ch_2_2_volume);
                self.ch_2_envelope_counter = 0;
                self.sender.send(self.sample_data.clone()).unwrap();
            }
        }

        let will_update_length_timer;
        {
            let state_before = apu_counter_before & 0b10 != 0;
            let state_after = self.apu_counter & 0b10 != 0;
            will_update_length_timer = state_before && !state_after;
        }
        if will_update_length_timer {
            if self.ch_2_4_length_enable && self.ch_2_length_counter < 64 {
                self.ch_2_length_counter += 1;
                if self.ch_2_length_counter == 64 {
                    self.disable_ch_2();
                }
            }
        }

    }

    pub fn update_apu(&mut self) {
        if self.ch_5_2_enable {
            let mut will_update_sample = false;

            if self.dac_2_enable {
                if self.ch_2_enable {
                    if self.ch_2_period_counter == 0x7FF {
                        self.ch_2_period_counter = self.ch_2_3_period;

                        let frequency = 131072.0 / (2048.0 - self.ch_2_3_period as f32);
                        let duty_cycle = match self.ch_2_1_length >> 6 {
                            0b00 => 0.125,
                            0b01 => 0.25,
                            0b10 => 0.5,
                            0b11 => 0.75,
                            _ => panic!("Error!: Invalid duty cycle bits")
                        };
                        will_update_sample = will_update_sample || 
                                             frequency != self.sample_data.ch_2_freq ||
                                             duty_cycle != self.sample_data.ch_2_duty;
                        self.sample_data.ch_2_duty = duty_cycle;
                        self.sample_data.ch_2_freq = frequency;

                        //Clock the duty step counter
                        self.ch_2_duty_counter += 1;
                        //println!("f: {frequency}, d: {duty_cycle}, v: {}", self.sample_data.ch_2_amp);
                    }
                    else {
                        self.ch_2_period_counter += 1;
                    }

                    //TODO: Implement the envelope
                }
                else {
                    //if the channel is disabled, channel emits a digital 0 (analog -1)
                    //0.0
                };
            }

            if will_update_sample {
                self.sender.send(self.sample_data.clone()).unwrap();
            }
        }
    }
}

fn digital_to_analog(digital: u8) -> f32 {
    let digital = (digital & 0x0F) as f32;
    ((2.0 / 15.0) * digital - 1.0) / 4.0
}

#[derive(Clone,Copy)]
pub struct SampleData {
    ch_2_freq: f32,
    ch_2_duty: f32,
    ch_2_amp: f32,
}

impl Default for SampleData {
    fn default() -> Self {
        Self { 
            ch_2_freq: 0.0,
            ch_2_duty: 0.0,
            ch_2_amp: 0.0,
        }
    }
}