use std::{
    cmp::Ordering,
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};

use rand::{rngs::ThreadRng, Rng};

const MEMORY_SIZE: usize = 4096;
const PROGRAM_INIT_LOAD_POS: usize = 0x200;
const MAX_ALLOWED_PROGRAM_SIZE: usize = MEMORY_SIZE - PROGRAM_INIT_LOAD_POS;
const FONT_START_POS: usize = 0x50;
const FONT_END_POS: usize = 0x9F;

const INSTRUCTIONS_PER_SECOND: usize = 700;

pub type CpuScreenMem = [u64; 32];

pub enum CpuIoEvents {
    KeyPressed(u8),
    KeyReleased(u8),
}

fn get_keypad_state_mask(key: u8) -> u16 {
    1 << key
}

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
    screen_pixels: CpuScreenMem,
    screen_update_sender: Sender<CpuScreenMem>,
    cpu_io_receiver: Receiver<CpuIoEvents>,
    rng: ThreadRng,
    keypad_state: u16,

    program_counter: usize,
    index_register: usize,
    stack: Vec<u16>,
    variable_registers: [u8; 16],
}

#[derive(Debug)]
pub enum InitCpuError {
    ProgramTooBig { actual: usize, allowed: usize },
}

impl Cpu {
    pub fn new(
        program: Vec<u8>,
        screen_update_sender: Sender<CpuScreenMem>,
        cpu_io_receiver: Receiver<CpuIoEvents>,
    ) -> Result<Self, InitCpuError> {
        let mut memory = [0; MEMORY_SIZE];

        // insert program to memory
        if program.len() > MAX_ALLOWED_PROGRAM_SIZE {
            return Err(InitCpuError::ProgramTooBig {
                actual: program.len(),
                allowed: MAX_ALLOWED_PROGRAM_SIZE,
            });
        }

        memory[PROGRAM_INIT_LOAD_POS..(PROGRAM_INIT_LOAD_POS + program.len())]
            .copy_from_slice(&program);

        // insert font to memory
        // font taken from https://tobiasvl.github.io/blog/write-a-chip-8-emulator/
        memory[FONT_START_POS..(FONT_END_POS + 1)].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]);

        let screen_pixels = [0; 32];
        let rng = rand::thread_rng();
        let keypad_state = 0;

        let program_counter = PROGRAM_INIT_LOAD_POS;
        let index_register = 0;
        let stack = Vec::with_capacity(16);
        let variable_registers = [0; 16];

        Ok(Self {
            memory,
            screen_pixels,
            screen_update_sender,
            cpu_io_receiver,
            rng,
            keypad_state,
            program_counter,
            index_register,
            stack,
            variable_registers,
        })
    }

    fn send_screen_update(&self) {
        self.screen_update_sender
            .send(self.screen_pixels)
            .expect("Update screen failed!");
    }

    fn process_cpu_io_event(&mut self, event: &CpuIoEvents) {
        match event {
            CpuIoEvents::KeyPressed(key) => {
                self.keypad_state |= get_keypad_state_mask(*key);
            }
            CpuIoEvents::KeyReleased(key) => {
                self.keypad_state &= !get_keypad_state_mask(*key);
            }
        }
    }

    pub fn run(&mut self) {
        let duration_per_instruction: Duration =
            Duration::from_secs_f32(1f32 / INSTRUCTIONS_PER_SECOND as f32);

        loop {
            let start_time = Instant::now();

            while let Ok(event) = self.cpu_io_receiver.try_recv() {
                self.process_cpu_io_event(&event);
            }

            let instruction = ((self.memory[self.program_counter] as u16) << 8)
                + self.memory[self.program_counter + 1] as u16;
            self.program_counter += 2;

            let op = ((instruction & 0xF000) >> 12) as u8;
            let x = ((instruction & 0x0F00) >> 8) as usize;
            let y = ((instruction & 0x00F0) >> 4) as usize;
            let n = (instruction & 0x000F) as u8;
            let nn = (instruction & 0x00FF) as u8;
            let nnn = instruction & 0x0FFF;

            let mut skip = false;

            match op {
                0x0 => {
                    if nnn == 0xE0 {
                        self.screen_pixels = [0; 32];
                        self.send_screen_update();
                    } else if nnn == 0xEE {
                        self.program_counter = self
                            .stack
                            .pop()
                            .expect("Should not call 0x00EE on an empty stack.")
                            as usize;
                    } else {
                        panic!("{:#06x} might be a call to a machine assembly routine, but this emulator does not support that.", instruction);
                    }
                }
                0x1 => {
                    self.program_counter = nnn as usize;
                }
                0x2 => {
                    self.stack.push(self.program_counter as u16);
                    self.program_counter = nnn as usize;
                }
                0x3 => {
                    if self.variable_registers[x] == nn {
                        skip = true;
                    }
                }
                0x4 => {
                    if self.variable_registers[x] != nn {
                        skip = true;
                    }
                }
                0x5 => {
                    if self.variable_registers[x] == self.variable_registers[y] {
                        skip = true;
                    }
                }
                0x6 => {
                    self.variable_registers[x] = nn;
                }
                0x7 => {
                    self.variable_registers[x] = self.variable_registers[x].wrapping_add(nn);
                }
                0x8 => match n {
                    0x0 => {
                        self.variable_registers[x] = self.variable_registers[y];
                    }
                    0x1 => {
                        self.variable_registers[x] |= self.variable_registers[y];
                    }
                    0x2 => {
                        self.variable_registers[x] &= self.variable_registers[y];
                    }
                    0x3 => {
                        self.variable_registers[x] ^= self.variable_registers[y];
                    }
                    0x4 => {
                        let (final_value, overflowed) =
                            self.variable_registers[x].overflowing_add(self.variable_registers[y]);

                        self.variable_registers[0xF] = if overflowed { 1 } else { 0 };
                        self.variable_registers[x] = final_value;
                    }
                    0x5 => {
                        self.variable_registers[0xF] =
                            if self.variable_registers[x] > self.variable_registers[y] {
                                1
                            } else {
                                0
                            };
                        self.variable_registers[x] =
                            self.variable_registers[x].wrapping_sub(self.variable_registers[y]);
                    }
                    0x6 => {
                        // TODO: Ambiguous instruction - provide configuration
                        self.variable_registers[0xF] = self.variable_registers[x] & 1;
                        self.variable_registers[x] >>= 1;
                    }
                    0x7 => {
                        self.variable_registers[0xF] =
                            if self.variable_registers[y] > self.variable_registers[x] {
                                1
                            } else {
                                0
                            };
                        self.variable_registers[x] =
                            self.variable_registers[y].wrapping_sub(self.variable_registers[x]);
                    }
                    0xE => {
                        // TODO: Ambiguous instruction - provide configuration
                        self.variable_registers[0xF] = (self.variable_registers[x] & 0x80) >> 7;
                        self.variable_registers[x] <<= 1;
                    }
                    _ => {
                        panic!("{:#06x} is not a valid 0x8 instruction.", instruction);
                    }
                },
                0x9 => {
                    if self.variable_registers[x] != self.variable_registers[y] {
                        skip = true;
                    }
                }
                0xA => {
                    self.index_register = nnn as usize;
                }
                0xB => {
                    // TODO: Ambiguous instruction - provide configuration
                    self.program_counter = nnn as usize + self.variable_registers[0x0] as usize;
                }
                0xC => {
                    self.variable_registers[x] = self.rng.gen::<u8>() & nn;
                }
                0xD => {
                    let x_start = (self.variable_registers[x] % 64) as u16;
                    let y_start = (self.variable_registers[y] % 32) as u16;
                    self.variable_registers[0xF] = 0;

                    let total_len = self.screen_pixels.len();
                    (y_start..(y_start + n as u16))
                        .into_iter()
                        .filter(|y| (*y as usize) < total_len)
                        .for_each(|y| {
                            let nth_byte =
                                self.memory[self.index_register + ((y - y_start) as usize)] as u64;
                            let mask = match x_start.cmp(&56) {
                                Ordering::Equal => nth_byte,
                                Ordering::Less => nth_byte << (56 - x_start),
                                Ordering::Greater => nth_byte >> (x_start - 56),
                            };
                            let is_updated = (mask & self.screen_pixels[y as usize]) != 0;
                            if is_updated {
                                self.variable_registers[0xF] = 1;
                            }
                            self.screen_pixels[y as usize] ^= mask;
                        });
                    self.send_screen_update();
                }
                0xE => {
                    if nn == 0x9E {
                        skip = self.keypad_state
                            & get_keypad_state_mask(self.variable_registers[x])
                            != 0;
                    } else if nn == 0xA1 {
                        skip = self.keypad_state
                            & get_keypad_state_mask(self.variable_registers[x])
                            == 0;
                    } else {
                        panic!("{:#06x} is not a valid 0xE instruction.", instruction);
                    }
                }
                0xF => {
                    // TODO: Actually implement 0xF
                    // panic!("{:#06x} is not a valid 0xF instruction.", instruction);
                }
                _ => {
                    unreachable!()
                }
            }

            if skip {
                self.program_counter += 2
            }

            let elapsed = start_time.elapsed();
            if elapsed < duration_per_instruction {
                std::thread::sleep(duration_per_instruction - elapsed);
            }
        }
    }
}
