mod font;
use std::fs::File;
use std::{io::BufReader, io::Read};

use font::FONT;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

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
    ((y as usize) * (WIDTH) + (x as usize)) % 2048
}

fn main() {
    let mut MEMORY: [u8; 4096] = [0; 4096];
    let mut pc = PROGRAM_START;
    let mut index_register: u16 = 0;
    let mut stack: Vec<u16> = vec![];

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut registers: [u8; 16] = [0; 16];

    // decrement by 60 every second
    let mut delay_timer: u8 = 0;
    let mut sound_timer: u8 = 0;

    // SETUP FONT
    for i in 0..FONT.len() {
        MEMORY[i + FONT_START] = FONT[i];
    }

    // LOAD PROGRAM
    let my_buf = BufReader::new(File::open("./ibm.ch8").unwrap());
    for (i, byte) in my_buf.bytes().enumerate() {
        let byte = byte.unwrap();
        MEMORY[i + PROGRAM_START] = byte;
        // println!("{:b}", byte);
    }

    let mut window = Window::new(
        "CHIP8 - ESC to exit",
        WIDTH,
        HEIGHT,
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
                        _ => panic!("Unexpected 0 instruction: {nnn:#05x}"),
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
                //3 => {}
                //4 => {}
                //5 => {}
                6 => {
                    registers[x as usize] = nn as u8;
                }
                7 => {
                    registers[x as usize] += nn as u8;
                }
                0xA => {
                    index_register = nnn;
                }
                0xD => {
                    let mut X = registers[x as usize] & 63;
                    let mut Y = registers[y as usize] & 31;
                    registers[0xF] = 0;

                    for i in 0..n {
                        let sprite_byte = MEMORY[(index_register + i) as usize];
                        for j in (0..8) {
                            let curIsOn = (sprite_byte & (0x80 >> j)) != 0;
                            let screenIsOn = buffer
                                [screen_coords(X as u16 + j as u16, Y as u16 + j as u16)]
                                == WHITE;
                            if curIsOn && screenIsOn {
                                buffer
                                    [screen_coords((X as u16 + j as u16), (Y as u16 + j as u16))] =
                                    BLACK;
                                registers[0xF] = 1;
                            } else if curIsOn && !screenIsOn {
                                buffer[screen_coords(X as u16 + j as u16, Y as u16 + i as u16)] =
                                    WHITE;
                            }
                            if (X + j + 1) >= WIDTH as u8 {
                                break;
                            }
                        }
                        if (Y + i as u8 + 1) >= HEIGHT as u8 {
                            break;
                        }
                    }
                }
                _ => panic!("Unexpected instruction category: {category:#06x}"),
            }
            // EXCUTE
            //
        }

        if delay_timer > 0 {
            delay_timer -= 1;
        }
        if sound_timer > 0 {
            // TODO: make a beep lmao
            sound_timer -= 1;
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    println!("Hello, world!");
}
