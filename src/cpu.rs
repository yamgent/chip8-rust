use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

const MEMORY_SIZE: usize = 4096;
const PROGRAM_INIT_LOAD_POS: usize = 0x200;
const MAX_ALLOWED_PROGRAM_SIZE: usize = MEMORY_SIZE - PROGRAM_INIT_LOAD_POS;
const FONT_START_POS: usize = 0x50;
const FONT_END_POS: usize = 0x9F;

const INSTRUCTIONS_PER_SECOND: usize = 700;

pub type CpuScreenMem = [u8; 256];

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
    screen_pixels: CpuScreenMem,
    screen_update_sender: Sender<CpuScreenMem>,

    program_counter: usize,
    index_register: u16,
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

        let screen_pixels = [0u8; 256];
        let program_counter = PROGRAM_INIT_LOAD_POS;
        let index_register = 0;
        let variable_registers = [0; 16];

        Ok(Self {
            memory,
            screen_pixels,
            screen_update_sender,
            program_counter,
            index_register,
            variable_registers,
        })
    }

    fn send_screen_update(&self) {
        self.screen_update_sender
            .send(self.screen_pixels)
            .expect("Update screen failed!");
    }

    pub fn run(&mut self) {
        let duration_per_instruction: Duration =
            Duration::from_secs_f32(1f32 / INSTRUCTIONS_PER_SECOND as f32);

        loop {
            let start_time = Instant::now();

            let instructions = ((self.memory[self.program_counter] as u16) << 8)
                + self.memory[self.program_counter + 1] as u16;
            self.program_counter += 2;

            let op = ((instructions & 0xF000) >> 12) as u8;
            let x = ((instructions & 0x0F00) >> 8) as usize;
            let y = ((instructions & 0x00F0) >> 4) as usize;
            let n = (instructions & 0x000F) as u8;
            let nn = (instructions & 0x00FF) as u8;
            let nnn = instructions & 0x0FFF;

            match op {
                0x0 => {
                    if nnn == 0xE0 {
                        self.screen_pixels = [0; 256];
                        self.send_screen_update();
                    }
                }
                0x1 => {
                    self.program_counter = nnn as usize;
                }
                0x6 => {
                    self.variable_registers[x] = nn;
                }
                0x7 => {
                    self.variable_registers[x] = self.variable_registers[x].wrapping_add(nn);
                }
                0xA => {
                    self.index_register = nnn;
                }
                _ =>
                // TODO: Eventually should be changed to unreachable!()
                {
                    unimplemented!()
                }
            }

            let elapsed = start_time.elapsed();
            if elapsed < duration_per_instruction {
                std::thread::sleep(duration_per_instruction - elapsed);
            }
        }
    }
}
