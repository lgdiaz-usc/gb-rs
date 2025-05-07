use std::{sync::mpsc::{channel, Receiver, Sender}, thread};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, FromSample, Sample, SizedSample};

const T_CYCLE_RATE: f32 = 4194304.0;
const M_CYCLE_RATE: f32 = 1048576.0;

pub struct APU {
    //Channel 1 registers
    ch_1_1_length: u8,   //NR11
    ch_1_2_volume: u8,   //NR12
    ch_1_3_period: u16,      //NR13
    ch_1_4_length_enable: bool,     //NR14

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
    dac_1_enable: bool,
    dac_2_enable: bool,

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
    ch_1_sweep_pace: u8,
    ch_1_volume: u8,

    //Channel 2
    ch_2_duty_counter: u8,
    ch_2_period_counter: u16,
    ch_2_length_counter: u8,
    ch_2_envelope_counter: u8,
    ch_2_envelope_increases: bool,
    ch_2_sweep_pace: u8,
    ch_2_volume: u8,

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
            ch_1_1_length: 0x3F,
            ch_1_2_volume: 0x00,
            ch_1_3_period: 0x0000,
            ch_1_4_length_enable: true,
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
            dac_1_enable: false,
            dac_2_enable: false,
            ch_1_duty_counter: 0,
            ch_1_envelope_counter: 0,
            ch_1_envelope_increases: false,
            ch_1_sweep_pace: 0,
            ch_1_length_counter: 0,
            ch_1_period_counter: 0,
            ch_1_volume: 0,
            ch_2_duty_counter: 0,
            ch_2_envelope_counter: 0,
            ch_2_envelope_increases: false,
            ch_2_sweep_pace: 0,
            ch_2_length_counter: 0,
            ch_2_period_counter: 0,
            ch_2_volume: 0,
            apu_counter: 0,
            dac_1_signal: 0.0,
            dac_2_signal: 0.0,
            dac_3_signal: 0.0,
            dac_4_signal: 0.0,
            gb_sample_rate: (M_CYCLE_RATE / sample_rate).trunc(),
            gb_sample_counter: 0.0,
            sender
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address >= 0xFF10 && address <= 0xFF26 {
            match address {
                0xFF11 => self.ch_1_1_length | 0b111111, //NR11
                0xFF12 => self.ch_1_2_volume, //NR12
                0xFF13 => 0xFF, //NR13
                0xFF14 => if self.ch_1_4_length_enable {0xFF} else {0xBF}, //NR14

                0xFF16 => self.ch_2_1_length | 0b111111, //NR21
                0xFF17 => self.ch_2_2_volume, //NR22
                0xFF18 => 0xFF, //NR23
                0xFF19 => if self.ch_2_4_length_enable {0xFF} else {0xBF}, //NR24

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
                        self.ch_1_sweep_pace = self.ch_1_2_volume & 0b111;
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
                        self.ch_2_sweep_pace = self.ch_2_2_volume & 0b111;
                    }

                    self.ch_2_4_length_enable = value & 0x40 != 0;
                    self.ch_2_3_period = (self.ch_2_3_period & 0x00FF) | ((value as u16 & 0b111) << 8);
                    return;
                },

                0xFF24 => &mut self.ch_5_0_volume, //NR50
                0xFF25 => &mut self.ch_5_1_panning, //NR51
                0xFF26 => { //NR52
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

    fn disable_ch_1(&mut self) {
        self.ch_1_enable = false;
        self.ch_1_envelope_counter = 0;
        self.ch_1_length_counter = 0;
        self.ch_1_period_counter = 0;
        self.ch_1_volume = 0;
    }

    fn disable_ch_2(&mut self) {
        self.ch_2_enable = false;
        self.ch_2_envelope_counter = 0;
        self.ch_2_length_counter = 0;
        self.ch_2_period_counter = 0;
        self.ch_2_volume = 0;
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

        //let mut sample_buffer = VecDeque::new();

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
            println!("{sample}");

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

        //TODO: Implement events that occur every N DIV-APU ticks
        let will_update_envelope;
        {
            let state_before = apu_counter_before & 0b100 != 0;
            let state_after = self.apu_counter & 0b100 != 0;
            will_update_envelope = state_before && !state_after;
        }
        if will_update_envelope {
            if self.ch_1_sweep_pace != 0 {
                self.ch_1_envelope_counter += 1;
                if self.ch_1_envelope_counter == self.ch_1_sweep_pace {
                    if self.ch_1_envelope_increases && self.ch_1_volume < 0xF {
                        self.ch_1_volume += 1;
                    }
                    else if !self.ch_1_envelope_increases && self.ch_1_volume > 0x0 {
                        self.ch_1_volume -= 1;
                    }
                    
                    if self.dac_1_signal != 0.0 {
                        self.dac_1_signal = volume_to_analog(self.ch_1_volume);
                    }
                
                    self.ch_1_envelope_counter = 0;
                }
            }

            if self.ch_2_sweep_pace != 0 {
                self.ch_2_envelope_counter += 1;
                if self.ch_2_envelope_counter == self.ch_2_sweep_pace {
                    if self.ch_2_envelope_increases && self.ch_2_volume < 0xF {
                        self.ch_2_volume += 1;
                    }
                    else if !self.ch_2_envelope_increases && self.ch_2_volume > 0x0 {
                        self.ch_2_volume -= 1;
                    }

                    if self.dac_2_signal != 0.0 {
                        self.dac_2_signal = volume_to_analog(self.ch_2_volume);
                    }

                    self.ch_2_envelope_counter = 0;
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
        }

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

                        //Clock the duty step counter
                        self.ch_1_duty_counter += 1;

                        let duty_cycle = (self.ch_1_1_length >> 6) as usize;
                        let duty_step = (self.ch_1_duty_counter & 0b111) as usize;

                        self.dac_1_signal = DUTY_VALUES[duty_cycle][duty_step] * volume_to_analog(self.ch_1_volume);
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

                        //Clock the duty step counter
                        self.ch_2_duty_counter += 1;

                        let duty_cycle = (self.ch_2_1_length >> 6) as usize;
                        let duty_step = (self.ch_2_duty_counter & 0b111) as usize;

                        self.dac_2_signal = DUTY_VALUES[duty_cycle][duty_step] * volume_to_analog(self.ch_2_volume);
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

            //Mixing and Panning
            if self.ch_5_1_panning & 0b1 != 0 {
                right_sample += self.dac_1_signal;
            }
            if self.ch_5_1_panning & 0b10 != 0 {
                right_sample += self.dac_2_signal;
            }
            if self.ch_5_1_panning & 0b100 != 0 {
                right_sample += self.dac_3_signal;
            }
            if self.ch_5_1_panning & 0b1000 != 0 {
                right_sample += self.dac_4_signal;
            }
            if self.ch_5_1_panning & 0b10000 != 0 {
                left_sample += self.dac_1_signal;
            }
            if self.ch_5_1_panning & 0b100000 != 0 {
                left_sample += self.dac_2_signal;
            }
            if self.ch_5_1_panning & 0b1000000 != 0 {
                left_sample += self.dac_3_signal;
            }
            if self.ch_5_1_panning & 0b10000000 != 0 {
                left_sample += self.dac_4_signal;
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

fn _digital_to_analog(digital: u8) -> f32 {
    let digital = (digital & 0x0F) as f32;
    (2.0 / 15.0) * digital - 1.0
}

fn volume_to_analog(volume: u8) -> f32 {
    let volume = (volume & 0x0F) as f32;
    volume / 15.0
}