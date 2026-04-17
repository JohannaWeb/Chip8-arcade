use std::fmt;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;
pub const MEMORY_SIZE: usize = 4096;
pub const PROGRAM_START: usize = 0x200;
const FONT_START: usize = 0x50;

const FONT_SET: [u8; 80] = [
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
];

#[derive(Debug)]
pub enum Chip8Error {
    ProgramTooLarge { size: usize, capacity: usize },
    StackOverflow,
    StackUnderflow,
    UnknownOpcode(u16),
}

impl fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramTooLarge { size, capacity } => {
                write!(f, "program is {size} bytes, but only {capacity} bytes fit")
            }
            Self::StackOverflow => write!(f, "stack overflow"),
            Self::StackUnderflow => write!(f, "stack underflow"),
            Self::UnknownOpcode(opcode) => write!(f, "unknown opcode: {opcode:#06x}"),
        }
    }
}

impl std::error::Error for Chip8Error {}

pub struct Chip8 {
    memory: [u8; MEMORY_SIZE],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; 16],
    sp: usize,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; 16],
    display: [bool; DISPLAY_SIZE],
    waiting_for_key: Option<usize>,
    rng_state: u32,
    draw_flag: bool,
}

impl Default for Chip8 {
    fn default() -> Self {
        Self::new()
    }
}

impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Self {
            memory: [0; MEMORY_SIZE],
            v: [0; 16],
            i: 0,
            pc: PROGRAM_START as u16,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            display: [false; DISPLAY_SIZE],
            waiting_for_key: None,
            rng_state: 0xC0DE_1234,
            draw_flag: true,
        };

        chip8.memory[FONT_START..FONT_START + FONT_SET.len()].copy_from_slice(&FONT_SET);
        chip8
    }

    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Chip8Error> {
        let capacity = MEMORY_SIZE - PROGRAM_START;
        if program.len() > capacity {
            return Err(Chip8Error::ProgramTooLarge {
                size: program.len(),
                capacity,
            });
        }

        self.memory[PROGRAM_START..PROGRAM_START + program.len()].copy_from_slice(program);
        self.pc = PROGRAM_START as u16;
        Ok(())
    }

    pub fn tick_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn cycle(&mut self) -> Result<(), Chip8Error> {
        if self.waiting_for_key.is_some() {
            return Ok(());
        }

        let opcode = self.fetch_opcode();
        self.pc = self.pc.wrapping_add(2);
        self.execute(opcode)
    }

    pub fn set_key(&mut self, key: usize, pressed: bool) {
        if key >= self.keypad.len() {
            return;
        }

        self.keypad[key] = pressed;
        if pressed {
            if let Some(register) = self.waiting_for_key.take() {
                self.v[register] = key as u8;
            }
        }
    }

    pub fn display(&self) -> &[bool; DISPLAY_SIZE] {
        &self.display
    }

    pub fn sound_active(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn take_draw_flag(&mut self) -> bool {
        let flag = self.draw_flag;
        self.draw_flag = false;
        flag
    }

    fn fetch_opcode(&self) -> u16 {
        let pc = self.pc as usize;
        ((self.memory[pc] as u16) << 8) | self.memory[pc + 1] as u16
    }

    fn execute(&mut self, opcode: u16) -> Result<(), Chip8Error> {
        let nnn = opcode & 0x0FFF;
        let n = (opcode & 0x000F) as u8;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let kk = (opcode & 0x00FF) as u8;

        match opcode & 0xF000 {
            0x0000 => match opcode {
                0x00E0 => {
                    self.display = [false; DISPLAY_SIZE];
                    self.draw_flag = true;
                }
                0x00EE => {
                    if self.sp == 0 {
                        return Err(Chip8Error::StackUnderflow);
                    }
                    self.sp -= 1;
                    self.pc = self.stack[self.sp];
                }
                _ => return Err(Chip8Error::UnknownOpcode(opcode)),
            },
            0x1000 => self.pc = nnn,
            0x2000 => {
                if self.sp == self.stack.len() {
                    return Err(Chip8Error::StackOverflow);
                }
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }
            0x3000 => {
                if self.v[x] == kk {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            0x4000 => {
                if self.v[x] != kk {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            0x5000 if n == 0 => {
                if self.v[x] == self.v[y] {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            0x6000 => self.v[x] = kk,
            0x7000 => self.v[x] = self.v[x].wrapping_add(kk),
            0x8000 => match n {
                0x0 => self.v[x] = self.v[y],
                0x1 => self.v[x] |= self.v[y],
                0x2 => self.v[x] &= self.v[y],
                0x3 => self.v[x] ^= self.v[y],
                0x4 => {
                    let (value, carry) = self.v[x].overflowing_add(self.v[y]);
                    self.v[x] = value;
                    self.v[0xF] = carry as u8;
                }
                0x5 => {
                    let (value, borrow) = self.v[x].overflowing_sub(self.v[y]);
                    self.v[x] = value;
                    self.v[0xF] = (!borrow) as u8;
                }
                0x6 => {
                    self.v[0xF] = self.v[x] & 1;
                    self.v[x] >>= 1;
                }
                0x7 => {
                    let (value, borrow) = self.v[y].overflowing_sub(self.v[x]);
                    self.v[x] = value;
                    self.v[0xF] = (!borrow) as u8;
                }
                0xE => {
                    self.v[0xF] = (self.v[x] & 0x80) >> 7;
                    self.v[x] <<= 1;
                }
                _ => return Err(Chip8Error::UnknownOpcode(opcode)),
            },
            0x9000 if n == 0 => {
                if self.v[x] != self.v[y] {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            0xA000 => self.i = nnn,
            0xB000 => self.pc = nnn + self.v[0] as u16,
            0xC000 => self.v[x] = self.next_random() & kk,
            0xD000 => self.draw_sprite(x, y, n),
            0xE000 => match kk {
                0x9E => {
                    if self.is_key_pressed(self.v[x]) {
                        self.pc = self.pc.wrapping_add(2);
                    }
                }
                0xA1 => {
                    if !self.is_key_pressed(self.v[x]) {
                        self.pc = self.pc.wrapping_add(2);
                    }
                }
                _ => return Err(Chip8Error::UnknownOpcode(opcode)),
            },
            0xF000 => match kk {
                0x07 => self.v[x] = self.delay_timer,
                0x0A => self.waiting_for_key = Some(x),
                0x15 => self.delay_timer = self.v[x],
                0x18 => self.sound_timer = self.v[x],
                0x1E => self.i = self.i.wrapping_add(self.v[x] as u16),
                0x29 => self.i = FONT_START as u16 + (self.v[x] as u16 & 0x0F) * 5,
                0x33 => {
                    let value = self.v[x];
                    let i = self.i as usize;
                    self.memory[i] = value / 100;
                    self.memory[i + 1] = (value / 10) % 10;
                    self.memory[i + 2] = value % 10;
                }
                0x55 => {
                    let i = self.i as usize;
                    self.memory[i..=i + x].copy_from_slice(&self.v[..=x]);
                }
                0x65 => {
                    let i = self.i as usize;
                    self.v[..=x].copy_from_slice(&self.memory[i..=i + x]);
                }
                _ => return Err(Chip8Error::UnknownOpcode(opcode)),
            },
            _ => return Err(Chip8Error::UnknownOpcode(opcode)),
        }

        Ok(())
    }

    fn draw_sprite(&mut self, x_register: usize, y_register: usize, rows: u8) {
        self.v[0xF] = 0;
        let x_start = self.v[x_register] as usize % DISPLAY_WIDTH;
        let y_start = self.v[y_register] as usize % DISPLAY_HEIGHT;

        for row in 0..rows as usize {
            let sprite_byte = self.memory[self.i as usize + row];
            let y = (y_start + row) % DISPLAY_HEIGHT;

            for bit in 0..8 {
                if (sprite_byte & (0x80 >> bit)) == 0 {
                    continue;
                }

                let x = (x_start + bit) % DISPLAY_WIDTH;
                let index = y * DISPLAY_WIDTH + x;
                if self.display[index] {
                    self.v[0xF] = 1;
                }
                self.display[index] ^= true;
            }
        }

        self.draw_flag = true;
    }

    fn next_random(&mut self) -> u8 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.rng_state >> 24) as u8
    }

    fn is_key_pressed(&self, key: u8) -> bool {
        self.keypad.get(key as usize).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chip_with_program(program: &[u8]) -> Chip8 {
        let mut chip8 = Chip8::new();
        chip8.load_program(program).unwrap();
        chip8
    }

    #[test]
    fn loads_program_at_0x200() {
        let mut chip8 = Chip8::new();
        chip8.load_program(&[0x60, 0x2A]).unwrap();
        assert_eq!(chip8.fetch_opcode(), 0x602A);
    }

    #[test]
    fn executes_register_load_and_add() {
        let mut chip8 = chip_with_program(&[0x60, 0x01, 0x70, 0xFF]);
        chip8.cycle().unwrap();
        chip8.cycle().unwrap();
        assert_eq!(chip8.v[0], 0);
    }

    #[test]
    fn draws_sprite_and_sets_collision() {
        let mut chip8 = chip_with_program(&[
            0x60, 0x00, // V0 = 0
            0x61, 0x00, // V1 = 0
            0xA3, 0x00, // I = 0x300
            0xD0, 0x11, // draw 1 row
            0xD0, 0x11, // draw again, collision
        ]);
        chip8.memory[0x300] = 0b1000_0000;

        for _ in 0..4 {
            chip8.cycle().unwrap();
        }
        assert!(chip8.display[0]);

        chip8.cycle().unwrap();
        assert!(!chip8.display[0]);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn waits_for_key_press() {
        let mut chip8 = chip_with_program(&[
            0xF0, 0x0A, // wait for key into V0
            0x61, 0x01, // V1 = 1
        ]);

        chip8.cycle().unwrap();
        chip8.cycle().unwrap();
        assert_eq!(chip8.v[1], 0);

        chip8.set_key(0xA, true);
        chip8.cycle().unwrap();
        assert_eq!(chip8.v[0], 0xA);
        assert_eq!(chip8.v[1], 1);
    }
}
