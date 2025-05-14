use std::{sync::mpsc::{channel, Receiver, Sender}, thread};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, FromSample, Sample, SizedSample};

const T_CYCLE_RATE: f32 = 4194304.0;
const M_CYCLE_RATE: f32 = 1048576.0;

pub struct APU {
    //Channel 1 registers
    ch_1_0_sweep: u8,           //NR10
    ch_1_1_length: u8,          //NR11
    ch_1_2_volume: u8,          //NR12
    ch_1_3_period: u16,         //NR13
    ch_1_4_length_enable: bool, //NR14

    //Channel 2 registers
    ch_2_1_length: u8,          //NR21
    ch_2_2_volume: u8,          //NR22
    ch_2_3_period: u16,         //NR23
    ch_2_4_length_enable: bool, //NR24

    //Channel 3 registers
    ch_3_1_length: u8,          //NR31
    ch_3_2_level: u8,           //NR32
    ch_3_3_period: u16,         //NR33
    ch_3_4_length_enable: bool, //NR34

    //Channel 4 registers
    ch_4_1_length: u8,          //NR41
    ch_4_2_volume: u8,          //NR42
    ch_4_3_randomness: u8,      //NR43
    ch_4_4_length_enable: bool, //NR44

    //Master control registers
    ch_5_0_volume: u8,      //NR50
    ch_5_1_panning: u8,     //NR51
    ch_5_2_enable: bool,    //NR52

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
    //Channel 1
    ch_1_duty_counter: u8,
    ch_1_period_counter: u16,
    ch_1_length_counter: u8,
    ch_1_envelope_counter: u8,
    ch_1_envelope_increases: bool,
    ch_1_envelope_pace: u8,
    ch_1_volume: u8,
    ch_1_sweep_period: u16,
    ch_1_sweep_pace: u8,
    ch_1_sweep_enabled: bool,

    //Channel 2
    ch_2_duty_counter: u8,
    ch_2_period_counter: u16,
    ch_2_length_counter: u8,
    ch_2_envelope_counter: u8,
    ch_2_envelope_increases: bool,
    ch_2_envelope_pace: u8,
    ch_2_volume: u8,

    //Channel 3
    ch_3_sample_index: u8,
    ch_3_length_counter: u8,
    ch_3_period_counter: u16,
    ch_3_volume: f32,

    //Channel 4
    ch_4_lfsr: u16,
    ch_4_period_counter: u16,
    ch_4_length_counter: u8,
    ch_4_envelope_counter: u8,
    ch_4_envelope_pace: u8,
    ch_4_envelope_increases: bool,
    ch_4_volume: u8,

    //DAC Signals
    dac_1_signal: f32,
    dac_2_signal: f32,
    dac_3_signal: f32,
    dac_4_signal: f32,

    //Sample cycle counter
    gb_sample_rate: f32,
    gb_sample_counter: f32,

    //Variables for sending data to audio library
    sender: Sender<f32>,
}

impl APU {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let (sample_send, sample_receive) = channel();

        thread::spawn(move || {
            Self::init_device(receiver, sample_send);
        });

        let sample_rate = sample_receive.recv().unwrap();

        Self {
            ch_1_0_sweep: 0x80,
            ch_1_1_length: 0x3F,
            ch_1_2_volume: 0x00,
            ch_1_3_period: 0x0000,
            ch_1_4_length_enable: true,
            ch_2_1_length: 0x3F,
            ch_2_2_volume: 0x00,
            ch_2_3_period: 0x0000,
            ch_2_4_length_enable: true,
            ch_3_1_length: 0xFF,
            ch_3_2_level: 0x9F,
            ch_3_3_period: 0x0000,
            ch_3_4_length_enable: true,
            ch_4_1_length: 0xFF,
            ch_4_2_volume: 0x00,
            ch_4_3_randomness: 0x00,
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
            ch_1_duty_counter: 0,
            ch_1_envelope_counter: 0,
            ch_1_envelope_increases: false,
            ch_1_envelope_pace: 0,
            ch_1_length_counter: 0,
            ch_1_period_counter: 0,
            ch_1_volume: 0,
            ch_1_sweep_pace: 0,
            ch_1_sweep_period: 0,
            ch_1_sweep_enabled: false,
            ch_2_duty_counter: 0,
            ch_2_envelope_counter: 0,
            ch_2_envelope_increases: false,
            ch_2_envelope_pace: 0,
            ch_2_length_counter: 0,
            ch_2_period_counter: 0,
            ch_2_volume: 0,
            ch_3_length_counter: 0,
            ch_3_period_counter: 0,
            ch_3_sample_index: 0,
            ch_3_volume: 0.0,
            ch_4_envelope_counter: 0,
            ch_4_envelope_pace: 0,
            ch_4_envelope_increases: false,
            ch_4_length_counter: 0,
            ch_4_lfsr: 0,
            ch_4_period_counter: 0,
            ch_4_volume: 0,
            apu_counter: 0,
            dac_1_signal: 0.0,
            dac_2_signal: 0.0,
            dac_3_signal: 0.0,
            dac_4_signal: 0.0,
            gb_sample_rate: (M_CYCLE_RATE / sample_rate).ceil(),
            gb_sample_counter: 0.0,
            sender
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0xFF10 && address <= 0xFF26 {
            match address {
                0xFF10 => self.ch_1_0_sweep | 0x80, //NR10
                0xFF11 => self.ch_1_1_length | 0b111111, //NR11
                0xFF12 => self.ch_1_2_volume, //NR12
                0xFF13 => 0xFF, //NR13
                0xFF14 => if self.ch_1_4_length_enable {0xFF} else {0xBF}, //NR14

                0xFF16 => self.ch_2_1_length | 0b111111, //NR21
                0xFF17 => self.ch_2_2_volume, //NR22
                0xFF18 => 0xFF, //NR23
                0xFF19 => if self.ch_2_4_length_enable {0xFF} else {0xBF}, //NR24

                0xFF1A => if self.dac_3_enable {0xFF} else {0x7F}, //NR30
                0xFF1B => 0xFF, //NR31
                0xFF1C => self.ch_3_2_level, //NR32
                0xFF1D => 0xFF, //NR33
                0xFF1E => if self.ch_2_4_length_enable {0xFF} else {0xBF}, //NR34

                0xFF20 => self.ch_4_1_length | 0b11000000, //NR41
                0xFF21 => self.ch_4_2_volume, //NR42
                0xFF22 => self.ch_4_3_randomness, //NR43
                0xFF23 => if self.ch_4_4_length_enable {0xFF} else {0xBF}, //NR44

                0xFF24 => self.ch_5_0_volume, //NR50
                0xFF25 => self.ch_5_1_panning, //NR 51
                0xFF26 => { //NR52
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
            if self.ch_3_enable {
                0xFF
            }
            else {
                self.wave_ram[(address - 0xFF30) as usize]
            }
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if address >= 0xFF10 && address <= 0xFF26 {
            //let mut value = value;
            let register = match address {
                0xFF10 => &mut self.ch_1_0_sweep, //NR10
                0xFF11 => &mut self.ch_1_1_length, //NR11
                0xFF12 => { //NR12
                    self.dac_1_enable = value & 0xF8 != 0;
                    if !self.dac_1_enable {
                        self.disable_ch_1();
                    }

                    &mut self.ch_1_2_volume
                },
                0xFF13 => { //NR13
                    self.ch_1_3_period &= 0xFF00;
                    self.ch_1_3_period |= value as u16;
                    return;
                },
                0xFF14 => { //NR14
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 2
                        self.ch_1_enable = true;
                        if self.ch_1_length_counter == 64 {
                            self.ch_1_length_counter = self.ch_1_1_length & 0x3F;
                        }
                        self.ch_1_period_counter = 0x7FF;
                        self.ch_1_envelope_counter = 0;
                        self.ch_1_volume = self.ch_1_2_volume >> 4;
                        self.ch_1_envelope_increases = self.ch_1_2_volume & 0b1000 != 0;
                        self.ch_1_envelope_pace = self.ch_1_2_volume & 0b111;
                        self.ch_1_sweep_period = self.ch_1_3_period;
                        self.ch_1_sweep_pace = (self.ch_1_0_sweep >> 4) & 0b111;
                        self.ch_1_sweep_enabled = (self.ch_1_sweep_pace != 0) || (self.ch_1_0_sweep & 0b111 != 0);
                        if self.ch_1_0_sweep & 0b111 != 0 {
                            self.calculate_sweep();
                        }
                    }

                    self.ch_1_4_length_enable = value & 0x40 != 0;
                    self.ch_1_3_period = (self.ch_1_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF16 => &mut self.ch_2_1_length, //NR21
                0xFF17 => { //NR22
                    self.dac_2_enable = value & 0xF8 != 0;
                    if !self.dac_2_enable {
                        self.disable_ch_2();
                    }

                    &mut self.ch_2_2_volume
                },
                0xFF18 => { //NR23
                    self.ch_2_3_period &= 0xFF00;
                    self.ch_2_3_period |= value as u16;
                    return;
                },
                0xFF19 => { //NR24
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
                        self.ch_2_envelope_pace = self.ch_2_2_volume & 0b111;
                    }

                    self.ch_2_4_length_enable = value & 0x40 != 0;
                    self.ch_2_3_period = (self.ch_2_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF1A => { //NR30
                    self.dac_3_enable = value & 0x80 != 0;
                    if !self.dac_3_enable {
                        self.disable_ch_3();
                    }
                    return;
                },
                0xFF1B => &mut self.ch_3_1_length, //NR31
                0xFF1C => &mut self.ch_3_2_level, //NR32
                0xFF1D => { //NR33
                    self.ch_3_3_period &= 0xFF00;
                    self.ch_3_3_period |= value as u16;
                    return;
                }
                0xFF1E => { //NR34
                    if value & 0x80 != 0 {
                        //TODO: code for triggering channel 3
                        self.ch_3_enable = true;
                        if self.ch_3_length_counter == 0 {
                            self.ch_3_length_counter = self.ch_3_1_length;
                        }
                        self.ch_3_period_counter = self.ch_3_3_period;
                        self.ch_3_volume = match (self.ch_3_2_level >> 5) & 0b11 {
                            0b00 => 0.0,
                            0b01 => 1.0,
                            0b10 => 0.5,
                            0b11 => 0.25,
                            _ => panic!("Error: Invalid volume bits")
                        };
                        self.ch_3_sample_index = 0;
                    }

                    self.ch_3_4_length_enable = value & 0x40 != 0;
                    self.ch_3_3_period = (self.ch_3_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF20 => &mut self.ch_4_1_length, //NR41
                0xFF21 => { //NR42
                    self.dac_4_enable = value & 0xF8 != 0;
                    if !self.dac_4_enable {
                        self.disable_ch_4();
                    }

                    &mut self.ch_4_2_volume
                },
                0xFF22 => &mut self.ch_4_3_randomness, //NR43
                0xFF23 => {
                    if value & 0x80 != 0 {
                        //TODO code for triggering channel 4
                        self.ch_4_enable = true;
                        if self.ch_4_length_counter == 64 {
                            self.ch_4_length_counter = self.ch_4_1_length & 0x3F;
                        }
                        self.ch_4_period_counter = self.get_ch_4_divisor();
                        self.ch_4_envelope_counter = 0;
                        self.ch_4_volume = self.ch_4_2_volume >> 4;
                        self.ch_4_envelope_increases = self.ch_4_2_volume & 0b1000 != 0;
                        self.ch_4_envelope_pace = self.ch_4_2_volume & 0b111;
                        self.ch_4_lfsr = 0xEFFF;
                    }

                    self.ch_4_4_length_enable = value & 0x40 != 0;
                    return;
                },

                0xFF24 => &mut self.ch_5_0_volume, //NR50
                0xFF25 => &mut self.ch_5_1_panning, //NR51
                0xFF26 => { //NR52
                    self.ch_5_2_enable = value & 0x80 != 0;
                    if !self.ch_5_2_enable {
                        //TODO: Disable other channels
                        self.disable_ch_1();
                        self.disable_ch_2();
                        self.disable_ch_3();
                    }
                    return;
                },
                _ => {
                    println!("ERROR: Unknown register ${:x}", address);
                    return;
                }
            };

            *register = value;
        }
        else if address >= 0xFF30 && address <= 0xFF3F {
            if !self.ch_3_enable {
                self.wave_ram[(address - 0xFF30) as usize] = value;
            }
        }
        else {
            panic!("ERROR: Address ${:x} out of bounds!", address)
        }
    }

    fn disable_ch_1(&mut self) {
        self.ch_1_enable = false;
        self.ch_1_envelope_counter = 0;
        self.ch_1_length_counter = 0;
        self.ch_1_period_counter = 0;
        self.ch_1_volume = 0;
        self.ch_1_sweep_enabled = false;
        self.ch_1_sweep_pace = 0;
        self.ch_1_sweep_period = 0;
    }

    fn disable_ch_2(&mut self) {
        self.ch_2_enable = false;
        self.ch_2_envelope_counter = 0;
        self.ch_2_length_counter = 0;
        self.ch_2_period_counter = 0;
        self.ch_2_volume = 0;
    }

    fn disable_ch_3(&mut self) {
        self.ch_3_enable = false;
        self.ch_3_length_counter = 0;
        self.ch_3_period_counter = 0;
        self.ch_3_sample_index = 0;
        self.ch_3_volume = 0.0;
    }

    fn disable_ch_4(&mut self) {
        self.ch_4_enable = false;
        self.ch_4_length_counter = 0;
        self.ch_4_envelope_counter = 0;
        self.ch_4_period_counter = 0;
        self.ch_4_volume = 0;
    }

    fn get_ch_4_divisor(&self) -> u16 {
        let divisor_code = self.ch_4_3_randomness & 0b111;
        let divisor = if divisor_code == 0 {8} else {16 * divisor_code} as u16;
        (divisor << (self.ch_4_3_randomness >> 4)) >> 2
    }
    
    pub fn init_device(receiver: Receiver<f32>, sample_send: Sender<f32>) {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("ERROR: failed to find output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::I8 => Self::run::<i8>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::I16 => Self::run::<i16>(receiver, sample_send, &device, &config.into()),
            //cpal::SampleFormat::I24 => Self::run::<I24>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::I32 => Self::run::<i32>(receiver, sample_send, &device, &config.into()),
            //cpal::SampleFormat::I48 => Self::run::<I48>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::I64 => Self::run::<i64>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::U8 => Self::run::<u8>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::U16 => Self::run::<u16>(receiver, sample_send, &device, &config.into()),
            //cpal::SampleFormat::U24 => Self::run::<U24>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::U32 => Self::run::<u32>(receiver, sample_send, &device, &config.into()),
            //cpal::SampleFormat::U48 => Self::run::<U48>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::U64 => Self::run::<u64>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::F32 => Self::run::<f32>(receiver, sample_send, &device, &config.into()),
            cpal::SampleFormat::F64 => Self::run::<f64>(receiver, sample_send, &device, &config.into()),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        }
    }

    fn run<T>(receiver: Receiver<f32>, sample_send: Sender<f32>, device: &cpal::Device, config: &cpal::StreamConfig)
    where 
        T: SizedSample + FromSample<f32>,
    {
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;
        sample_send.send(sample_rate).unwrap();

        let mut left_capacitor = 0.0;
        let mut right_capacitor = 0.0;
        let charge_factor = 0.999958_f32.powf(T_CYCLE_RATE / sample_rate);
        let mut is_left_channel = false;
        let mut high_pass_filter = move |input: f32, enabled: bool| -> f32 {
            let capacitor = if is_left_channel {&mut left_capacitor} else {&mut right_capacitor};

            let mut output = 0.0;
            if enabled {
                output = input - *capacitor;
                *capacitor = input - output * charge_factor;
            }

            output
        };

        let mut next_value = move || {
            let sample = receiver.recv().unwrap();
            //println!("{sample}");

            is_left_channel = ! is_left_channel;
            
            high_pass_filter(sample, true)
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
            
            for sample in frame.iter_mut() {
                let value: T = T::from_sample(next_sample());
                *sample = value;
            }
        }
    }

    pub fn update_apu_timer(&mut self) {
        let apu_counter_before = self.apu_counter;
        self.apu_counter += 1;

        let will_update_envelope;
        {
            let state_before = apu_counter_before & 0b100 != 0;
            let state_after = self.apu_counter & 0b100 != 0;
            will_update_envelope = state_before && !state_after;
        }
        if will_update_envelope {
            if self.ch_1_envelope_pace != 0 {
                self.ch_1_envelope_counter += 1;
                if self.ch_1_envelope_counter == self.ch_1_envelope_pace {
                    if self.ch_1_envelope_increases && self.ch_1_volume < 0xF {
                        self.ch_1_volume += 1;
                    }
                    else if !self.ch_1_envelope_increases && self.ch_1_volume > 0x0 {
                        self.ch_1_volume -= 1;
                    }
                
                    self.ch_1_envelope_counter = 0;
                }
            }

            if self.ch_2_envelope_pace != 0 {
                self.ch_2_envelope_counter += 1;
                if self.ch_2_envelope_counter == self.ch_2_envelope_pace {
                    if self.ch_2_envelope_increases && self.ch_2_volume < 0xF {
                        self.ch_2_volume += 1;
                    }
                    else if !self.ch_2_envelope_increases && self.ch_2_volume > 0x0 {
                        self.ch_2_volume -= 1;
                    }

                    self.ch_2_envelope_counter = 0;
                }
            }

            if self.ch_4_envelope_pace != 0 {
                self.ch_4_envelope_counter += 1;
                if self.ch_4_envelope_counter == self.ch_4_envelope_pace {
                    if self.ch_4_envelope_increases && self.ch_4_volume < 0xF {
                        self.ch_4_volume += 1;
                    }
                    else if !self.ch_4_envelope_increases && self.ch_4_volume > 0x0 {
                        self.ch_4_volume -= 1;
                    }

                    self.ch_4_envelope_counter = 0;
                }
            }
        }

        let will_update_length_timer;
        {
            let state_before = apu_counter_before & 0b1 != 0;
            let state_after = self.apu_counter & 0b1 != 0;
            will_update_length_timer = state_before && !state_after;
        }
        if will_update_length_timer {
            if self.ch_1_4_length_enable && self.ch_1_length_counter < 64 {
                self.ch_1_length_counter += 1;
                if self.ch_1_length_counter == 64 {
                    self.disable_ch_1();
                }
            }

            if self.ch_2_4_length_enable && self.ch_2_length_counter < 64 {
                self.ch_2_length_counter += 1;
                if self.ch_2_length_counter == 64 {
                    self.disable_ch_2();
                }
            }

            if self.ch_3_4_length_enable && self.ch_3_enable {
                self.ch_3_length_counter += 1;
                if self.ch_3_length_counter == 0 {
                    self.disable_ch_3();
                }
            }

            if self.ch_4_4_length_enable && self.ch_4_length_counter < 64 {
                self.ch_4_length_counter += 1;
                if self.ch_4_length_counter == 64 {
                    self.disable_ch_4();
                }
            }
        }

        let will_update_sweep;
        {
            let state_before = apu_counter_before & 0b10 != 0;
            let state_after = self.apu_counter & 0b10 != 0;
            will_update_sweep = state_before && !state_after;
        }
        if will_update_sweep {
            if self.ch_1_sweep_pace > 0 {
                self.ch_1_sweep_pace -= 0;
            }

            if self.ch_1_sweep_pace == 0 {
                let new_pace = (self.ch_1_0_sweep >> 4) & 0b111;
                if new_pace != 0 {
                    self.ch_1_sweep_pace = new_pace;
                }
                else {
                    self.ch_1_sweep_pace = 8;
                }

                if self.ch_1_sweep_enabled && new_pace != 0 {
                    let new_period = self.calculate_sweep();
                    if self.ch_1_enable {
                        self.ch_1_sweep_period = new_period;
                        self.ch_1_3_period = new_period;
                        self.calculate_sweep();
                    }
                }
            }
        }

    }

    fn calculate_sweep(&mut self) -> u16 {
        let shifted_period = self.ch_1_sweep_period >> (self.ch_1_0_sweep & 0b111);

        let new_period;
        if self.ch_1_0_sweep & 0b1000 == 0 {
            new_period = self.ch_1_sweep_period + shifted_period;
        }
        else {
            new_period = self.ch_1_sweep_period - shifted_period;
        }

        if new_period > 0x7FF {
            self.disable_ch_1();
        }

        new_period
    }

    pub fn update_apu(&mut self) {
        if self.ch_5_2_enable {
            if self.dac_1_enable {
                if self.ch_1_enable {
                    const DUTY_VALUES: [[f32; 8]; 4] = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                                                        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
                                                        [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
                                                        [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0]];

                    if self.ch_1_period_counter == 0x7FF {
                        self.ch_1_period_counter = self.ch_1_3_period;

                        let duty_cycle = (self.ch_1_1_length >> 6) as usize;
                        let duty_step = (self.ch_1_duty_counter & 0b111) as usize;

                        self.dac_1_signal = DUTY_VALUES[duty_cycle][duty_step];

                        //Clock the duty step counter
                        self.ch_1_duty_counter += 1;
                        //println!("f: {frequency}, d: {duty_cycle}, v: {}", self.sample_data.ch_2_amp);
                    }
                    else {
                        self.ch_1_period_counter += 1;
                    }
                }
                else {
                    //if the channel is disabled, channel emits a digital 0 (analog -1)
                    //0.0
                };
            }

            if self.dac_2_enable {
                if self.ch_2_enable {
                    const DUTY_VALUES: [[f32; 8]; 4] = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                                                        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
                                                        [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
                                                        [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0]];

                    if self.ch_2_period_counter == 0x7FF {
                        self.ch_2_period_counter = self.ch_2_3_period;

                        let duty_cycle = (self.ch_2_1_length >> 6) as usize;
                        let duty_step = (self.ch_2_duty_counter & 0b111) as usize;

                        self.dac_2_signal = DUTY_VALUES[duty_cycle][duty_step];

                        //Clock the duty step counter
                        self.ch_2_duty_counter += 1;
                        //println!("f: {frequency}, d: {duty_cycle}, v: {}", self.sample_data.ch_2_amp);
                    }
                    else {
                        self.ch_2_period_counter += 1;
                    }
                }
                else {
                    //if the channel is disabled, channel emits a digital 0 (analog -1)
                    //0.0
                };
            }

            if self.dac_3_enable {
                if self.ch_3_enable {
                    for _ in 0..2 {
                        if self.ch_3_period_counter == 0x7FF {
                            self.ch_3_period_counter = self.ch_3_3_period;

                            let is_odd = self.ch_3_sample_index & 0b1 != 0;
                            let index = ((self.ch_3_sample_index & 0x1F) >> 1) as usize;

                            let sample;
                            if is_odd {
                                sample = self.wave_ram[index] >> 4;
                            }
                            else {
                                sample = self.wave_ram[index] & 0xF;
                            }

                            self.dac_3_signal = digital_to_analog(sample);

                            //Clock the sample index
                            self.ch_3_sample_index += 1;
                        }
                        else {
                            self.ch_3_period_counter += 1;
                        }
                    }
                }
                else {
                    //TODO what to do when dac is on and channel is off
                }
            }

            if self.dac_4_enable {
                if self.ch_4_enable {
                    if self.ch_4_period_counter == 0 {
                        self.ch_4_period_counter = self.get_ch_4_divisor();

                        if (self.ch_4_lfsr & 0b1) ^ ((self.ch_4_lfsr & 0b10) >> 1) != 0 {
                            let bits_to_set = if self.ch_4_3_randomness & 0b1000 != 0 {0x8080} else {0x8000};
                            self.ch_4_lfsr &= !bits_to_set;
                            self.ch_4_lfsr |= bits_to_set;
                        }
                        self.ch_4_lfsr >>= 1;

                        self.dac_4_signal = if self.ch_4_lfsr & 0b1 != 0 {0.0} else {1.0};
                    }
                    else {
                        self.ch_4_period_counter -= 1;
                    }
                }
                else {
                    //TODO what to do when dac is on but channel is off
                }
            }
        }

        self.gb_sample_counter += 1.0;
        if self.gb_sample_counter == self.gb_sample_rate {
            //if the APU is disabled, only play silence 
            if !self.ch_5_2_enable {
                self.sender.send(0.0).unwrap();
                self.sender.send(0.0).unwrap();
                return;
            }

            let mut left_sample = 0.0;
            let mut right_sample = 0.0;

            const CH_3_REDUCTION: f32 = 0.25;

            //Mixing and Panning
            if self.ch_5_1_panning & 0b1 != 0 {
                right_sample += self.dac_1_signal * volume_to_analog(self.ch_1_volume);
            }
            if self.ch_5_1_panning & 0b10 != 0 {
                right_sample += self.dac_2_signal * volume_to_analog(self.ch_2_volume);
            }
            if self.ch_5_1_panning & 0b100 != 0 {
                right_sample += self.dac_3_signal * self.ch_3_volume * CH_3_REDUCTION;
            }
            if self.ch_5_1_panning & 0b1000 != 0 {
                right_sample += self.dac_4_signal * volume_to_analog(self.ch_4_volume);
            }
            if self.ch_5_1_panning & 0b10000 != 0 {
                left_sample += self.dac_1_signal * volume_to_analog(self.ch_1_volume);
            }
            if self.ch_5_1_panning & 0b100000 != 0 {
                left_sample += self.dac_2_signal * volume_to_analog(self.ch_2_volume);
            }
            if self.ch_5_1_panning & 0b1000000 != 0 {
                left_sample += self.dac_3_signal * self.ch_3_volume * CH_3_REDUCTION;
            }
            if self.ch_5_1_panning & 0b10000000 != 0 {
                left_sample += self.dac_4_signal * volume_to_analog(self.ch_4_volume);
            }

            //Brings the mixed signal back into the range of -1.0 to +1.0
            left_sample /= 4.0;
            right_sample /= 4.0;

            //Applies the master volume to left and right channels
            let left_volume = ((self.ch_5_0_volume & 0x70) >> 3) + 1;
            let right_volume = ((self.ch_5_0_volume & 0x7) << 1) + 1;
            left_sample *= volume_to_analog(left_volume);
            right_sample *= volume_to_analog(right_volume);

            self.sender.send(left_sample).unwrap();
            self.sender.send(right_sample).unwrap();

            self.gb_sample_counter = 0.0;
        }
    }
}

fn digital_to_analog(digital: u8) -> f32 {
    let digital = (digital & 0x0F) as f32;
    (2.0 / 15.0) * digital - 1.0
}

fn volume_to_analog(volume: u8) -> f32 {
    let volume = (volume & 0x0F) as f32;
    volume / 15.0
}