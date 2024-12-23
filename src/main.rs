mod font;
use std::fs::File;
use std::{io::BufReader, io::Read};

use font::FONT;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const SCALING_FACTOR: usize = 16;

const FONT_START: usize = 0x50;
// chip8 programs should beloaded starting at 0x200 (512)
const PROGRAM_START: usize = 0x200;

const WHITE: u32 = 0xFFFFFF;
const BLACK: u32 = 0x0;
const INSTRUCTIONS_PER_FRAME: usize = 12;

#[inline(always)]
fn first_nibble(num: u16) -> u16 {
    (num & 0xF000) >> 12
}
#[inline(always)]
fn second_nibble(num: u16) -> u16 {
    (num & 0x0F00) >> 8
}
#[inline(always)]
fn third_nibble(num: u16) -> u16 {
    (num & 0x00F0) >> 4
}
#[inline(always)]
fn fourth_nibble(num: u16) -> u16 {
    num & 0x000F
}

#[inline(always)]
fn screen_coords(x: u16, y: u16) -> usize {
    ((y as usize) * (WIDTH) + (x as usize)) & (2047)
}

static KEYMAP: &'static [Key] = &[
    Key::X,    // 0x0 = X
    Key::Key1, // 1 = 1
    Key::Key2, // 2 = 2
    Key::Key3, // 3 = 3
    Key::Q,    // 4 = Q
    Key::W,    //5 = W
    Key::E,    // 6 = E
    Key::A,    // 7 = A
    Key::S,    // 8 = S
    Key::D,    // 9 = D
    Key::Z,    // 0xA = Z
    Key::C,    // 0xB = C
    Key::Key4, // 0xC = 4
    Key::R,    // 0xD = R
    Key::F,    // 0xE = F
    Key::V,    // 0xF = V
];

fn main() {
    let mut MEMORY: [u8; 4096] = [0; 4096];
    let mut pc = PROGRAM_START;
    let mut index_register: u16 = 0;
    let mut stack: Vec<u16> = vec![];

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut bigBuffer: Vec<u32> = vec![0; SCALING_FACTOR * SCALING_FACTOR * (WIDTH * HEIGHT)];
    let mut registers: [u8; 16] = [0; 16];

    // decrement by 60 every second
    let mut delay_timer: u8 = 0;
    let mut sound_timer: u8 = 0;

    // SETUP FONT
    for i in 0..FONT.len() {
        MEMORY[i + FONT_START] = FONT[i];
    }

    // LOAD PROGRAM
    let my_buf = BufReader::new(File::open("./glitchGhost.ch8").unwrap());
    for (i, byte) in my_buf.bytes().enumerate() {
        let byte = byte.unwrap();
        MEMORY[i + PROGRAM_START] = byte;
        // println!("{:b}", byte);
    }

    let mut window = Window::new(
        "CHIP8 - ESC to exit",
        WIDTH * SCALING_FACTOR,
        HEIGHT * SCALING_FACTOR,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for _ in 0..INSTRUCTIONS_PER_FRAME {
            // FETCH
            let instruction: u16 = ((MEMORY[pc] as u16) << 8) as u16 | MEMORY[pc + 1] as u16;
            //if instruction != 0x1228 {
            //    println!("INSTRUCTION {instruction:#06x} PC {pc}");
            // }
            pc += 2;

            // DECODE
            // CHIP-8 instructions are divided into categories by the first nibble
            // (half-byte), or 4 bits
            let category = first_nibble(instruction);
            let x = second_nibble(instruction);
            let y = third_nibble(instruction);
            let n = fourth_nibble(instruction);
            let nn = (y << 4) | n;
            let nnn = (x << 8) | nn;
            //if instruction != 0x1228 {
            //    println!("category: {category:#06x} x: {x:#06x} y: {y:#06x} n: {n:#06x}");
            //}
            match category {
                0 => {
                    // 00E0
                    match nnn {
                        0x0E0 => {
                            for i in buffer.iter_mut() {
                                *i = BLACK;
                            }
                        }
                        0x0EE => {
                            assert!(!stack.is_empty());
                            pc = stack.pop().unwrap() as usize;
                        }
                        _ => continue,
                    }
                    // 00EE
                }
                1 => {
                    pc = nnn as usize;
                }
                2 => {
                    stack.push(pc as u16);
                    pc = nnn as usize;
                }
                3 => {
                    if registers[x as usize] == nn as u8 {
                        pc += 2;
                    }
                }
                4 => {
                    if registers[x as usize] != nn as u8 {
                        pc += 2;
                    }
                }
                5 => {
                    if registers[x as usize] == registers[y as usize] {
                        pc += 2;
                    }
                }
                6 => {
                    registers[x as usize] = nn as u8;
                }
                7 => {
                    registers[x as usize] = registers[x as usize].overflowing_add(nn as u8).0;
                }
                8 => match n {
                    0 => registers[x as usize] = registers[y as usize],
                    1 => registers[x as usize] |= registers[y as usize],
                    2 => registers[x as usize] &= registers[y as usize],
                    3 => registers[x as usize] ^= registers[y as usize],
                    4 => {
                        // overflows
                        let z = registers[x as usize].checked_add(registers[y as usize]);
                        if z.is_none() {
                            registers[0xF] = 1;
                        } else {
                            registers[0xF] = 0;
                        }
                        registers[x as usize] = registers[y as usize].overflowing_add(nn as u8).0;
                    }
                    5 => {
                        let z = registers[x as usize].checked_sub(registers[y as usize]);
                        if z.is_none() {
                            registers[0xF] = 0;
                        } else {
                            registers[0xF] = 1;
                        }

                        // registers[x as usize] -= registers[y as usize];
                        registers[x as usize] = registers[x as usize]
                            .overflowing_sub(registers[y as usize])
                            .0;
                    }
                    6 => {
                        // ORIGINAL BEHAVIOR
                        // registers[x as usize] = registers[y as usize];
                        registers[0xF] = registers[x as usize] & 1;
                        registers[x as usize] >>= 1;
                    }
                    7 => {
                        let z = registers[y as usize].checked_sub(registers[x as usize]);
                        if z.is_none() {
                            registers[0xF] = 0;
                        } else {
                            registers[0xF] = 1;
                        }
                        registers[y as usize] = registers[y as usize]
                            .overflowing_sub(registers[x as usize])
                            .0;

                        // registers[y as usize] -= registers[x as usize];
                    }
                    0xE => {
                        // ORIGINAL BEHAVIOR
                        // registers[x as usize] = registers[y as usize];
                        registers[0xF] = (registers[x as usize] >> 7) & 1;
                        registers[x as usize] <<= 1;
                    }
                    _ => continue, // _ => panic!("Unexpected 8 instruction: {n:#06x}"),
                },
                9 => {
                    if registers[x as usize] != registers[y as usize] {
                        pc += 2;
                    }
                }
                0xA => {
                    index_register = nnn;
                }
                0xB => {
                    // ORIGINAL BEHAVIOR
                    pc = (nnn + registers[0] as u16) as usize;
                }
                0xC => {
                    registers[x as usize] = (rand::random::<u16>() & nn) as u8;
                }
                0xD => {
                    let mut X = registers[x as usize] & 63;
                    let mut Y = registers[y as usize] & 31;
                    registers[0xF] = 0;

                    for i in 0..n {
                        let sprite_byte = MEMORY[(index_register + i) as usize];
                        for j in 0..8 {
                            let curIsOn = (sprite_byte & (0x80 >> j)) != 0;
                            let screenIsOn = buffer
                                [screen_coords(X as u16 + j as u16, Y as u16 + i as u16)]
                                == WHITE;
                            if curIsOn && screenIsOn {
                                buffer
                                    [screen_coords((X as u16 + j as u16), (Y as u16 + i as u16))] =
                                    BLACK;
                                registers[0xF] = 1;
                            } else if curIsOn && !screenIsOn {
                                buffer[screen_coords(X as u16 + j as u16, Y as u16 + i as u16)] =
                                    WHITE;
                            }
                        }
                    }

                    for i in 0..(SCALING_FACTOR * HEIGHT) {
                        for j in 0..(SCALING_FACTOR * WIDTH) {
                            let row = i / SCALING_FACTOR;
                            let col = j / SCALING_FACTOR;
                            let big_index = (i * (SCALING_FACTOR * WIDTH)) + (j);
                            bigBuffer[big_index] = buffer[screen_coords(col as u16, row as u16)];
                        }
                    }
                }
                0xE => match nn {
                    0x9E => {
                        if window.is_key_down(KEYMAP[registers[x as usize] as usize]) {
                            pc += 2;
                        }
                    }
                    0xA1 => {
                        if !window.is_key_down(KEYMAP[registers[x as usize] as usize]) {
                            pc += 2;
                        }
                    }
                    _ => panic!("Bad E instruction: {nn:#06x}"),
                },
                0xF => match nn {
                    0x07 => registers[x as usize] = delay_timer,
                    0x15 => delay_timer = registers[x as usize],
                    0x18 => sound_timer = registers[x as usize],
                    0x1E => {
                        index_register += registers[x as usize] as u16;
                        if index_register >= 0x1000 {
                            registers[0xF] = 1;
                        }
                    }
                    0x0A => {
                        let keys = window.get_keys();
                        if keys.is_empty() {
                            pc -= 2;
                        } else {
                            for i in 0..KEYMAP.len() {
                                if keys.contains(&KEYMAP[i]) {
                                    registers[x as usize] = i as u8;
                                    break;
                                }
                            }
                        }
                    }
                    0x29 => {
                        index_register =
                            registers[x as usize] as u16 * 5 as u16 + FONT_START as u16;
                    }
                    0x33 => {
                        let first_digit = (registers[x as usize] / 100) % 10;
                        let second_digit = (registers[x as usize] / 10) % 10;
                        let third_digit = registers[x as usize] % 10;
                        MEMORY[index_register as usize] = first_digit;
                        MEMORY[index_register as usize + 1] = second_digit;
                        MEMORY[index_register as usize + 2] = third_digit;
                    }
                    0x55 => {
                        // modern behavior
                        // println!("registers[x]: {}", registers[x as usize]);
                        for i in 0..(u8::min(registers[x as usize] + 1, 16)) {
                            MEMORY[index_register as usize + i as usize] = registers[i as usize];
                        }
                    }
                    0x65 => {
                        // modern behavior
                        for i in 0..(u8::min(registers[x as usize] + 1, 16)) {
                            registers[i as usize] = MEMORY[index_register as usize + i as usize];
                        }
                    }
                    _ => panic!("Unexpected F instruction: {nn:#06x}"),
                },
                _ => panic!("Unexpected instruction category: {category:#06x}"),
            }
            // EXECUTE
            //
        }

        if delay_timer > 0 {
            delay_timer -= 1;
        }
        if sound_timer > 0 {
            // TODO: make a beep lmao
            sound_timer -= 1;
        }

        window
            .update_with_buffer(&bigBuffer, WIDTH * SCALING_FACTOR, HEIGHT * SCALING_FACTOR)
            .unwrap();
    }

    println!("Hello, world!");
}
