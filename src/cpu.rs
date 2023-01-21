use std::{sync::mpsc::Sender, time::Duration};

const MEMORY_SIZE: usize = 4096;
const PROGRAM_INIT_LOAD_POS: usize = 0x200;
const MAX_ALLOWED_PROGRAM_SIZE: usize = MEMORY_SIZE - PROGRAM_INIT_LOAD_POS;
const FONT_START_POS: usize = 0x50;
const FONT_END_POS: usize = 0x9F;

pub type CpuScreenMem = [u8; 256];

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
    screen_update_sender: Sender<CpuScreenMem>,
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

        Ok(Self {
            memory,
            screen_update_sender,
        })
    }

    pub fn run(&self) {
        // TODO: Actual interpreter
        let mut pixels = [0u8; 256];
        let mut current = 0;
        pixels[current] = 0x80;

        loop {
            pixels[current] >>= 1;
            if pixels[current] == 0 {
                current = (current + 1) % pixels.len();
                pixels[current] = 0x80;
            }
            self.screen_update_sender
                .send(pixels)
                .expect("Update screen failed!");

            std::thread::sleep(Duration::from_millis(250));
        }
    }
}
