use std::env;
use std::fs;
use std::time::{Duration, Instant};

use chip8::{Chip8, DISPLAY_HEIGHT, DISPLAY_SIZE, DISPLAY_WIDTH};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};

mod chip8;

const FG_COLOR: u32 = 0x00_E8_FF_D6;
const BG_COLOR: u32 = 0x00_10_14_16;
const CPU_HZ: u64 = 700;
const TIMER_HZ: u64 = 60;
const MAX_CYCLES_PER_UPDATE: usize = 20;
const FRONTEND_WIDTH: usize = 640;
const FRONTEND_HEIGHT: usize = 360;
const MENU_BG: u32 = 0x00_0B_0F_10;
const MENU_FG: u32 = 0x00_E9_F5_E7;
const MENU_DIM: u32 = 0x00_83_96_8D;
const MENU_ACCENT: u32 = 0x00_F0_C8_5A;
const MENU_DANGER: u32 = 0x00_E8_5D_75;
const RPS_ROM: &[u8] = include_bytes!("RPS.ch8");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match env::args().nth(1) {
        Some(rom_path) => run_chip8_rom(&rom_path),
        None => run_frontend(),
    }
}

fn run_chip8_rom(rom_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rom = fs::read(&rom_path)?;

    let mut chip8 = Chip8::new();
    chip8.load_program(&rom)?;

    let mut window = Window::new(
        "CHIP-8 Emulator",
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        WindowOptions {
            resize: false,
            scale: Scale::X8,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(0);

    let mut frame_buffer = vec![0; DISPLAY_SIZE];
    let cycle_period = Duration::from_secs_f64(1.0 / CPU_HZ as f64);
    let timer_period = Duration::from_secs_f64(1.0 / TIMER_HZ as f64);
    let mut next_cycle = Instant::now() + cycle_period;
    let mut next_timer_tick = Instant::now() + timer_period;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        update_keys(&window, &mut chip8);

        let now = Instant::now();
        let mut cycles_this_update = 0;
        while now >= next_cycle && cycles_this_update < MAX_CYCLES_PER_UPDATE {
            chip8.cycle()?;
            next_cycle += cycle_period;
            cycles_this_update += 1;
        }

        while now >= next_timer_tick {
            chip8.tick_timers();
            next_timer_tick += timer_period;
        }

        if chip8.take_draw_flag() {
            draw_frame(chip8.display(), &mut frame_buffer);
            window.update_with_buffer(&frame_buffer, DISPLAY_WIDTH, DISPLAY_HEIGHT)?;
        } else {
            window.update();
        }

        let title = if chip8.sound_active() {
            "CHIP-8 Emulator - BEEP"
        } else {
            "CHIP-8 Emulator"
        };
        window.set_title(title);

        std::thread::sleep(Duration::from_millis(2));
    }

    Ok(())
}

fn draw_frame(display: &[bool; DISPLAY_SIZE], frame_buffer: &mut [u32]) {
    for (pixel, is_on) in frame_buffer.iter_mut().zip(display.iter()) {
        *pixel = if *is_on { FG_COLOR } else { BG_COLOR };
    }
}

fn update_keys(window: &Window, chip8: &mut Chip8) {
    for (chip_key, key) in KEYMAP {
        chip8.set_key(chip_key, window.is_key_down(key));
    }

    for key in window.get_keys_pressed(KeyRepeat::No) {
        if key == Key::F1 {
            eprintln!("Keypad layout: 1 2 3 4 / Q W E R / A S D F / Z X C V");
        }
    }
}

const KEYMAP: [(usize, Key); 16] = [
    (0x1, Key::Key1),
    (0x2, Key::Key2),
    (0x3, Key::Key3),
    (0xC, Key::Key4),
    (0x4, Key::Q),
    (0x5, Key::W),
    (0x6, Key::E),
    (0xD, Key::R),
    (0x7, Key::A),
    (0x8, Key::S),
    (0x9, Key::D),
    (0xE, Key::F),
    (0xA, Key::Z),
    (0x0, Key::X),
    (0xB, Key::C),
    (0xF, Key::V),
];

fn run_frontend() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "CHIP-8 Arcade",
        FRONTEND_WIDTH,
        FRONTEND_HEIGHT,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);

    let mut frontend = Frontend::new();
    let mut buffer = vec![0; FRONTEND_WIDTH * FRONTEND_HEIGHT];
    let mut last_frame = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32().min(0.05);
        last_frame = now;

        frontend.update(&window, dt);
        frontend.draw(&mut buffer);
        window.update_with_buffer(&buffer, FRONTEND_WIDTH, FRONTEND_HEIGHT)?;
    }

    Ok(())
}

enum FrontendMode {
    Menu,
    Pong(Pong),
    Chip8Game(Chip8Game),
}

struct Frontend {
    mode: FrontendMode,
}

impl Frontend {
    fn new() -> Self {
        Self {
            mode: FrontendMode::Menu,
        }
    }

    fn update(&mut self, window: &Window, dt: f32) {
        match &mut self.mode {
            FrontendMode::Menu => {
                if was_pressed(window, Key::Enter) || was_pressed(window, Key::Space) {
                    self.mode = FrontendMode::Chip8Game(Chip8Game::new("RPS.CH8", RPS_ROM));
                } else if was_pressed(window, Key::P) {
                    self.mode = FrontendMode::Pong(Pong::new());
                }
            }
            FrontendMode::Pong(pong) => {
                if was_pressed(window, Key::Backspace) {
                    self.mode = FrontendMode::Menu;
                } else {
                    pong.update(window, dt);
                    if was_pressed(window, Key::R) {
                        pong.reset_match();
                    }
                }
            }
            FrontendMode::Chip8Game(game) => {
                if was_pressed(window, Key::Backspace) {
                    self.mode = FrontendMode::Menu;
                } else if was_pressed(window, Key::R) {
                    game.reset();
                } else {
                    game.update(window);
                }
            }
        }
    }

    fn draw(&self, buffer: &mut [u32]) {
        match &self.mode {
            FrontendMode::Menu => draw_menu(buffer),
            FrontendMode::Pong(pong) => pong.draw(buffer),
            FrontendMode::Chip8Game(game) => game.draw(buffer),
        }
    }
}

struct Chip8Game {
    title: &'static str,
    rom: &'static [u8],
    chip8: Chip8,
    next_cycle: Instant,
    next_timer_tick: Instant,
}

impl Chip8Game {
    fn new(title: &'static str, rom: &'static [u8]) -> Self {
        let mut game = Self {
            title,
            rom,
            chip8: Chip8::new(),
            next_cycle: Instant::now(),
            next_timer_tick: Instant::now(),
        };
        game.reset();
        game
    }

    fn reset(&mut self) {
        self.chip8 = Chip8::new();
        self.chip8
            .load_program(self.rom)
            .expect("embedded CHIP-8 ROM should fit in memory");
        let now = Instant::now();
        self.next_cycle = now;
        self.next_timer_tick = now;
    }

    fn update(&mut self, window: &Window) {
        update_keys(window, &mut self.chip8);

        let now = Instant::now();
        let cycle_period = Duration::from_secs_f64(1.0 / CPU_HZ as f64);
        let timer_period = Duration::from_secs_f64(1.0 / TIMER_HZ as f64);
        let mut cycles_this_update = 0;

        while now >= self.next_cycle && cycles_this_update < MAX_CYCLES_PER_UPDATE {
            if let Err(error) = self.chip8.cycle() {
                eprintln!("{error}");
                break;
            }
            self.next_cycle += cycle_period;
            cycles_this_update += 1;
        }

        while now >= self.next_timer_tick {
            self.chip8.tick_timers();
            self.next_timer_tick += timer_period;
        }
    }

    fn draw(&self, buffer: &mut [u32]) {
        clear(buffer, MENU_BG);
        draw_text(buffer, 32, 16, self.title, 3, MENU_FG);
        draw_text(buffer, 360, 18, "R RESET", 2, MENU_DIM);
        draw_text(buffer, 474, 18, "BACKSPACE MENU", 2, MENU_DIM);
        draw_text(
            buffer,
            32,
            326,
            "CHIP-8 KEYS: 1 2 3 4 / Q W E R / A S D F / Z X C V",
            1,
            MENU_DIM,
        );

        let scale = 8;
        let origin_x = (FRONTEND_WIDTH - DISPLAY_WIDTH * scale) / 2;
        let origin_y = 58;
        fill_rect(
            buffer,
            origin_x - 4,
            origin_y - 4,
            DISPLAY_WIDTH * scale + 8,
            DISPLAY_HEIGHT * scale + 8,
            0x00_1D_27_27,
        );

        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let color = if self.chip8.display()[y * DISPLAY_WIDTH + x] {
                    FG_COLOR
                } else {
                    BG_COLOR
                };
                fill_rect(
                    buffer,
                    origin_x + x * scale,
                    origin_y + y * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }

        if self.chip8.sound_active() {
            draw_text(buffer, 274, 16, "BEEP", 2, MENU_ACCENT);
        }
    }
}

struct Pong {
    left_y: f32,
    right_y: f32,
    ball_x: f32,
    ball_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    left_score: u8,
    right_score: u8,
}

impl Pong {
    fn new() -> Self {
        let mut pong = Self {
            left_y: 150.0,
            right_y: 150.0,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            left_score: 0,
            right_score: 0,
        };
        pong.serve(1.0);
        pong
    }

    fn reset_match(&mut self) {
        self.left_score = 0;
        self.right_score = 0;
        self.left_y = 150.0;
        self.right_y = 150.0;
        self.serve(1.0);
    }

    fn serve(&mut self, direction: f32) {
        self.ball_x = FRONTEND_WIDTH as f32 / 2.0;
        self.ball_y = FRONTEND_HEIGHT as f32 / 2.0;
        self.ball_vx = 220.0 * direction;
        self.ball_vy = 120.0;
    }

    fn update(&mut self, window: &Window, dt: f32) {
        const PADDLE_SPEED: f32 = 280.0;
        const PADDLE_HEIGHT: f32 = 58.0;
        const PADDLE_WIDTH: f32 = 10.0;
        const BALL_SIZE: f32 = 8.0;
        const LEFT_X: f32 = 34.0;
        const RIGHT_X: f32 = FRONTEND_WIDTH as f32 - 44.0;
        const TOP: f32 = 42.0;
        const BOTTOM: f32 = FRONTEND_HEIGHT as f32 - 18.0;

        if window.is_key_down(Key::W) {
            self.left_y -= PADDLE_SPEED * dt;
        }
        if window.is_key_down(Key::S) {
            self.left_y += PADDLE_SPEED * dt;
        }
        if window.is_key_down(Key::Up) {
            self.right_y -= PADDLE_SPEED * dt;
        }
        if window.is_key_down(Key::Down) {
            self.right_y += PADDLE_SPEED * dt;
        }

        self.left_y = self.left_y.clamp(TOP, BOTTOM - PADDLE_HEIGHT);
        self.right_y = self.right_y.clamp(TOP, BOTTOM - PADDLE_HEIGHT);
        self.ball_x += self.ball_vx * dt;
        self.ball_y += self.ball_vy * dt;

        if self.ball_y <= TOP || self.ball_y + BALL_SIZE >= BOTTOM {
            self.ball_vy = -self.ball_vy;
            self.ball_y = self.ball_y.clamp(TOP, BOTTOM - BALL_SIZE);
        }

        if rects_overlap(
            self.ball_x,
            self.ball_y,
            BALL_SIZE,
            BALL_SIZE,
            LEFT_X,
            self.left_y,
            PADDLE_WIDTH,
            PADDLE_HEIGHT,
        ) && self.ball_vx < 0.0
        {
            self.hit_paddle(self.left_y, LEFT_X + PADDLE_WIDTH + 1.0, 1.0);
        }

        if rects_overlap(
            self.ball_x,
            self.ball_y,
            BALL_SIZE,
            BALL_SIZE,
            RIGHT_X,
            self.right_y,
            PADDLE_WIDTH,
            PADDLE_HEIGHT,
        ) && self.ball_vx > 0.0
        {
            self.hit_paddle(self.right_y, RIGHT_X - BALL_SIZE - 1.0, -1.0);
        }

        if self.ball_x < 0.0 {
            self.right_score = self.right_score.saturating_add(1);
            self.serve(-1.0);
        } else if self.ball_x > FRONTEND_WIDTH as f32 {
            self.left_score = self.left_score.saturating_add(1);
            self.serve(1.0);
        }
    }

    fn hit_paddle(&mut self, paddle_y: f32, new_x: f32, direction: f32) {
        const PADDLE_HEIGHT: f32 = 58.0;
        let paddle_center = paddle_y + PADDLE_HEIGHT / 2.0;
        let offset = ((self.ball_y - paddle_center) / (PADDLE_HEIGHT / 2.0)).clamp(-1.0, 1.0);
        let speed = (self.ball_vx.abs() + 18.0).min(390.0);
        self.ball_x = new_x;
        self.ball_vx = speed * direction;
        self.ball_vy = offset * 230.0;
    }

    fn draw(&self, buffer: &mut [u32]) {
        clear(buffer, MENU_BG);
        draw_text(buffer, 22, 14, "W/S", 2, MENU_DIM);
        draw_text(buffer, FRONTEND_WIDTH - 98, 14, "UP/DOWN", 2, MENU_DIM);
        draw_text(buffer, 255, 14, "PONG", 3, MENU_FG);
        draw_text(buffer, 18, 334, "BACKSPACE MENU", 2, MENU_DIM);
        draw_text(buffer, 430, 334, "R RESET", 2, MENU_DIM);

        let score = format!("{}   {}", self.left_score, self.right_score);
        draw_text(buffer, 280, 62, &score, 4, MENU_ACCENT);

        for y in (46..330).step_by(22) {
            fill_rect(buffer, FRONTEND_WIDTH / 2 - 2, y, 4, 12, MENU_DIM);
        }

        fill_rect(buffer, 34, self.left_y as usize, 10, 58, MENU_FG);
        fill_rect(
            buffer,
            FRONTEND_WIDTH - 44,
            self.right_y as usize,
            10,
            58,
            MENU_FG,
        );
        fill_rect(
            buffer,
            self.ball_x as usize,
            self.ball_y as usize,
            8,
            8,
            MENU_ACCENT,
        );
    }
}

fn draw_menu(buffer: &mut [u32]) {
    clear(buffer, MENU_BG);
    draw_text(buffer, 82, 46, "CHIP-8 ARCADE", 5, MENU_FG);
    draw_text(buffer, 96, 132, "RPS.CH8", 6, MENU_ACCENT);
    draw_text(buffer, 92, 210, "ENTER / SPACE TO PLAY RPS", 3, MENU_FG);
    draw_text(buffer, 168, 258, "P FOR PONG", 3, MENU_DIM);
    draw_text(buffer, 180, 296, "ESC QUITS", 2, MENU_DANGER);
}

fn was_pressed(window: &Window, key: Key) -> bool {
    window
        .get_keys_pressed(KeyRepeat::No)
        .into_iter()
        .any(|pressed| pressed == key)
}

fn rects_overlap(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn clear(buffer: &mut [u32], color: u32) {
    buffer.fill(color);
}

fn fill_rect(buffer: &mut [u32], x: usize, y: usize, width: usize, height: usize, color: u32) {
    let x_end = (x + width).min(FRONTEND_WIDTH);
    let y_end = (y + height).min(FRONTEND_HEIGHT);
    for py in y..y_end {
        let row = py * FRONTEND_WIDTH;
        for px in x..x_end {
            buffer[row + px] = color;
        }
    }
}

fn draw_text(buffer: &mut [u32], x: usize, y: usize, text: &str, scale: usize, color: u32) {
    let mut cursor = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor += 4 * scale;
            continue;
        }
        if let Some(glyph) = glyph(ch) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        fill_rect(
                            buffer,
                            cursor + col * scale,
                            y + row * scale,
                            scale,
                            scale,
                            color,
                        );
                    }
                }
            }
        }
        cursor += 6 * scale;
    }
}

fn glyph(ch: char) -> Option<[u8; 7]> {
    match ch.to_ascii_uppercase() {
        '0' => Some([0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E]),
        '1' => Some([0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E]),
        '2' => Some([0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F]),
        '3' => Some([0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E]),
        '4' => Some([0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02]),
        '5' => Some([0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E]),
        '6' => Some([0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E]),
        '7' => Some([0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08]),
        '8' => Some([0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E]),
        '9' => Some([0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C]),
        'A' => Some([0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
        'B' => Some([0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E]),
        'C' => Some([0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E]),
        'D' => Some([0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E]),
        'E' => Some([0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F]),
        'F' => Some([0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10]),
        'G' => Some([0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F]),
        'H' => Some([0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
        'I' => Some([0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E]),
        'J' => Some([0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E]),
        'K' => Some([0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11]),
        'L' => Some([0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F]),
        'M' => Some([0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11]),
        'N' => Some([0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11]),
        'O' => Some([0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
        'P' => Some([0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10]),
        'Q' => Some([0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D]),
        'R' => Some([0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11]),
        'S' => Some([0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E]),
        'T' => Some([0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
        'U' => Some([0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
        'V' => Some([0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04]),
        'W' => Some([0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A]),
        'X' => Some([0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11]),
        'Y' => Some([0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04]),
        'Z' => Some([0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F]),
        '-' => Some([0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00]),
        '/' => Some([0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10]),
        ':' => Some([0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00]),
        _ => None,
    }
}
