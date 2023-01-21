const MEMORY_SIZE: usize = 4096;
const PROGRAM_INIT_LOAD_POS: usize = 0x200;
const MAX_ALLOWED_PROGRAM_SIZE: usize = MEMORY_SIZE - PROGRAM_INIT_LOAD_POS;
const FONT_START_POS: usize = 0x50;
const FONT_END_POS: usize = 0x9F;

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
}

#[derive(Debug)]
pub enum InitCpuError {
    ProgramTooBig { actual: usize, allowed: usize },
}

impl Cpu {
    pub fn new(program: &[u8]) -> Result<Self, InitCpuError> {
        let mut memory = [0; MEMORY_SIZE];

        // insert program to memory
        if program.len() > MAX_ALLOWED_PROGRAM_SIZE {
            return Err(InitCpuError::ProgramTooBig {
                actual: program.len(),
                allowed: MAX_ALLOWED_PROGRAM_SIZE,
            });
        }

        memory[PROGRAM_INIT_LOAD_POS..(PROGRAM_INIT_LOAD_POS + program.len())]
            .copy_from_slice(program);

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

        Ok(Self { memory })
    }
}
