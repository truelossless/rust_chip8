extern crate log;

use log::{debug, error, trace, warn};
use std::fs::read;

// size of the screen, in pixels
const XPX: usize = 64;
const YPX: usize = 32;

// size of the internal memory (4K)
const MEM_SIZE: usize = 4096;

// memory reserved to store the fontset
const FONTSET_SIZE: usize = 80;

// memory reserved for the display functions
const DISPLAY_SIZE: usize = 256;

// memory reserved for the stack
const STACK_SIZE: usize = 96;

const REGISTER_NUM: usize = 16;
const KEY_NUM: usize = 16;
const STACK_LAYERS: usize = 16;

// adress of memory where the program counter start
const PC_START: usize = 512; // 512 == 0x200

const CHIP8_FONTSET: [u8; FONTSET_SIZE] = [
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

pub struct Chip8 {
    // memory of the chip8 system
    memory: [u8; MEM_SIZE],

    // registers
    register: [u8; REGISTER_NUM],

    // program counter
    program_counter: usize,

    // index register
    index_register: u16,

    // representation of the screen: 64*32
    // the screen is black and white, so the value taken can either be 0 or 1;
    display: [[u8; YPX]; XPX], // called later as display[x][y]

    // timers, decrementing every 1/60 second
    delay_timer: u8, // used for game animations & timing
    sound_timer: u8, // used for making sounds

    // the stack, used to store the program counter after a subroutine call
    stack: [u16; STACK_LAYERS], // the stack has 16 levels
    stack_pointer: usize,       // "pointer" to track the current stack level

    // hex keycodes for the chip8 keyboard, which has 16 keys
    key: [u8; KEY_NUM],

    // wether or not the program should stop it's execution until a key is pressed
    wait_for_key: bool,
    // this holds a reference to the register which will contain the pressed key
    wait_for_key_register: usize,
}

impl Chip8 {
    // returns a new emulator
    pub fn new() -> Self {
        let mut chip8 = Chip8 {
            memory: [0; MEM_SIZE],
            register: [0; REGISTER_NUM],
            // first byte of the program
            program_counter: PC_START,
            index_register: 0,
            display: [[0; YPX]; XPX],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; STACK_LAYERS],
            stack_pointer: 0,
            key: [0; KEY_NUM],
            wait_for_key: false,
            wait_for_key_register: 0,
        };

        // load the fontset into the emulator memory
        for i in 0..FONTSET_SIZE {
            chip8.memory[i] = CHIP8_FONTSET[i];
        }

        chip8
    }

    /// get the virtual screen
    pub fn display(&self) -> &[[u8; YPX]; XPX] {
        &self.display
    }

    /// load the game into the emulator
    pub fn load(&mut self, file_path: &str) -> Result<(), String> {
        let binary_file = read(file_path).map_err(|err| err.to_string())?;

        // check if the program fits in the emulator's memory
        if binary_file.len() > MEM_SIZE - (FONTSET_SIZE + DISPLAY_SIZE + STACK_SIZE) {
            return Err("The program doesn't fit in the emulator's memory !".to_string());
        }

        for (i, &byte) in binary_file.iter().enumerate() {
            self.memory[PC_START + i] = byte;
        }
        Ok(())
    }

    /// reset all key states to unpressed
    pub fn clear_keys(&mut self) {
        for key in self.key.iter_mut() {
            *key = 0;
        }
    }

    /// mark a key as pressed
    pub fn register_key(&mut self, key: u8) {
        self.key[key as usize] = 1;
    }

    /// emulate one step of the chip8
    pub fn emulate(&mut self) -> Result<(), String> {
        // get the opcode, which corresponds to a processor instruction. see:
        // https://en.wikipedia.org/wiki/CHIP-8
        // for an exhaustive list.

        // we might need to stop the program until a certain key is pressed

        if self.wait_for_key {
            for (keycode, &key_state) in self.key.iter().enumerate() {
                // a key is pressed
                if key_state == 1 {
                    self.wait_for_key = false;
                    debug!("got key {}", keycode);
                    self.register[self.wait_for_key_register] = keycode as u8;
                    break;
                }
            }

            // still no key pressed, return from the function
            if self.wait_for_key {
                return Ok(());
            }
        }

        // opcodes are 2 bytes long.
        // get the first byte, shift by a byte, combine with the second byte.
        let opcode: u16 = (self.memory[self.program_counter] as u16) << 8
            | (self.memory[self.program_counter + 1] as u16);

        debug!(
            "----------- chip8 cycle: got opcode {:X} -----------",
            opcode
        );

        // increase the program counter for the next opcode
        self.program_counter += 2;

        // decrement both timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        // process our opcode here
        match opcode & 0xF000 {
            // multiple functions exist here, so we need another match
            0x0000 => {
                match opcode & 0xFFFF {
                    // return from a subroutine
                    0x00EE => {
                        // jump back to the right address
                        self.program_counter = self.stack[self.stack_pointer] as usize;

                        if self.stack_pointer != 0 {
                            self.stack_pointer -= 1;
                            debug!("exiting subroutine.");
                        } else {
                            error!("no subroutine to exit !");
                        }
                    }

                    // clear the display
                    0x00E0 => {
                        self.display = [[0; YPX]; XPX];
                        debug!("cleared display.");
                    }

                    _ => warn!("warning: ran into unknown opcode: {:X}", opcode),
                }
            }

            // jump
            0x1000 => {
                let jump_address = opcode & 0x0FFF;
                self.program_counter = jump_address as usize;
                debug!("jumping to address {}", jump_address);
                trace!(
                    "corresponding address in the .ch8 file: {}",
                    jump_address - PC_START as u16
                );
            }

            // call a subroutine
            0x2000 => {
                // where is the subroutine to call
                let subroutine_address = opcode & 0x0FFF;
                self.stack_pointer += 1;

                if self.stack_pointer == 16 {
                    error!("recursion of more than 16 routines !");
                }

                // store on the stack where to return after the subroutine exited
                self.stack[self.stack_pointer] = self.program_counter as u16;

                // go to the subroutine
                self.program_counter = subroutine_address as usize;

                debug!("jumping to subroutine at address {}", subroutine_address);
                trace!(
                    "corresponding address in the .ch8 file: {}",
                    subroutine_address - PC_START as u16
                );
            }

            // condition: skip the next instruction if the register is equal to a constant
            0x3000 => {
                let register_number = (opcode & 0x0F00) >> 8;
                let constant = opcode & 0x00FF;

                debug!(
                    "checking register number {:X} if {} is equal to the constant {}",
                    register_number, self.register[register_number as usize], constant
                );

                if self.register[register_number as usize] == constant as u8 {
                    // skip the next 2 bytes
                    self.program_counter += 2;
                    debug!("test passed, skipping next opcode.");
                } else {
                    trace!("test failed.");
                }
            }

            // condition: skip the next instruction if the register is NOT equal to a constant
            0x4000 => {
                let register_number = (opcode & 0x0F00) >> 8;
                let constant = opcode & 0x00FF;

                debug!(
                    "checking register number {:X} if {} is different from the constant {}",
                    register_number, self.register[register_number as usize], constant
                );

                if self.register[register_number as usize] != constant as u8 {
                    // skip the next 2 bytes
                    self.program_counter += 2;
                    debug!("test passed, skipping next opcode.");
                } else {
                    trace!("test failed.");
                }
            }

            // assign to register
            0x6000 => {
                let register_number = (opcode & 0x0F00) >> 8;
                self.register[register_number as usize] = (opcode & 0x00FF) as u8;
                debug!(
                    "assigning {} to register number {:X} ",
                    self.register[register_number as usize], register_number
                );
            }

            // add to a register
            0x7000 => {
                let register_number = (opcode & 0x0F00) >> 8;
                debug!(
                    "adding {} to register number {:X} with value {}",
                    opcode & 0x00FF,
                    register_number,
                    self.register[register_number as usize]
                );

                // it seems that some roms use register overflowing as a feature, so make sure we don't make rust panic
                let tmp_register: u16 =
                    self.register[register_number as usize] as u16 + (opcode & 0x00FF);

                self.register[register_number as usize] = (tmp_register % 0x100) as u8;
                trace!("result: {}", self.register[register_number as usize]);
            }

            // multiple functions exist here, so we need another match
            // these functions handle arithmetic operations between registers.
            0x8000 => {
                match opcode & 0x000F {
                    // assign the value of a register to another one
                    0x0000 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;

                        debug!(
                            "setting the register {:X} to {}, the value of the register {:X}",
                            first_register,
                            self.register[second_register as usize],
                            second_register
                        );
                        trace!("overwriting {}", self.register[first_register as usize]);

                        self.register[first_register as usize] =
                            self.register[second_register as usize];
                    }

                    // bitwise OR between two registers
                    0x0001 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;
                        debug!(
                            "storing bitwise operation {} from {:X} | {} from {:X} to {:X}",
                            self.register[first_register as usize],
                            first_register,
                            self.register[second_register as usize],
                            second_register,
                            first_register
                        );
                        self.register[first_register as usize] |=
                            self.register[second_register as usize];
                        trace!("result: {}", self.register[first_register as usize]);
                    }

                    // bitwise AND between two registers
                    0x0002 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;
                        debug!(
                            "storing bitwise operation {} from {:X} & {} from {:X} to {:X}",
                            self.register[first_register as usize],
                            first_register,
                            self.register[second_register as usize],
                            second_register,
                            first_register
                        );
                        self.register[first_register as usize] &=
                            self.register[second_register as usize];
                        trace!("result: {}", self.register[first_register as usize]);
                    }

                    // bitwise XOR between two registers
                    0x0003 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;
                        debug!(
                            "storing bitwise operation {} from {:X} ^ {} from {:X} to {:X}",
                            self.register[first_register as usize],
                            first_register,
                            self.register[second_register as usize],
                            second_register,
                            first_register
                        );
                        self.register[first_register as usize] ^=
                            self.register[second_register as usize];
                        trace!("result: {}", self.register[first_register as usize]);
                    }

                    // add one register to another
                    0x0004 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;
                        debug!(
                            "adding {} to register {:X} containing {} from register {:X}",
                            self.register[second_register as usize],
                            first_register,
                            self.register[first_register as usize],
                            second_register
                        );

                        let mut res: u16 = self.register[first_register as usize] as u16
                            + self.register[second_register as usize] as u16;
                        // the result takes 9 bit: store the MSB in the carry flag register
                        if res > 255 {
                            debug!("{} is a 9 bit result, setting the carry flag", res);
                            // store the first in the F register
                            self.register[15] = 1;
                            // discard the firt bit to have a valid 8 bit variable
                            res &= 0b011111111;
                            trace!("new 8 bit result (without the carry bit): {}", res);
                        } else {
                            debug!("result: {}", res);
                        }

                        self.register[first_register as usize] = res as u8;
                    }

                    // substract the first register by the second register
                    0x0005 => {
                        let first_register = (opcode & 0x0F00) >> 8;
                        let second_register = (opcode & 0x0F0) >> 4;

                        // set the borrow flag if the second register is greater than the first one
                        if self.register[second_register as usize]
                            > self.register[first_register as usize]
                        {
                            self.register[15] = 0; // it's a bit confusing: 0 means borrowing
                        } else {
                            self.register[15] = 1; //
                        }

                        self.register[first_register as usize] -=
                            self.register[second_register as usize];
                    }

                    // stores LSB in register F and shift the register to the right
                    0x0006 => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        debug!(
                            "shifting right by one {} in {:X}",
                            self.register[register_number as usize], register_number
                        );
                        // store the lsb in the F register
                        self.register[15] = self.register[register_number as usize] & 1;
                        debug!("lsb {} stored in register F", self.register[15]);

                        // store the shift back in the register
                        self.register[register_number as usize] >>= 1;
                        debug!("result: {}", self.register[register_number as usize]);
                    }
                    // stores MSB in register F and shift the register to the left
                    0x000E => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        debug!(
                            "shifting left by one {} in {:X}",
                            self.register[register_number as usize], register_number
                        );
                        // store the msb in the F register
                        self.register[15] =
                            (self.register[register_number as usize] & 0b10000000) >> 7;
                        debug!("msb {} stored in register F", self.register[15]);

                        // store the shift back in the register
                        self.register[register_number as usize] <<= 1;
                        debug!("result: {}", self.register[register_number as usize]);
                    }

                    _ => warn!("warning: ran into unknown opcode: {:X}", opcode),
                }
            }

            // set the value of the index register
            0xA000 => {
                self.index_register = opcode & 0x0FFF;
                debug!("setting index register to {}", self.index_register);
            }

            // draw to the screen
            0xD000 => {
                // clear the F register; it's going to be used for collision detection.
                self.register[15] = 0;

                // get the x coordinate of where to draw on the display
                let x = self.register[((opcode & 0x0F00) >> 8) as usize] as u16;
                // get the y coordinate
                let y = self.register[((opcode & 0x00F0) >> 4) as usize] as u16;
                debug!("starting drawing operation at ({};{})", x, y);

                // sprite height
                let height = opcode & 0x000F;
                trace!("height of the drawing: {}", height);

                for i in y..y + height {
                    // get the pixels data from the memory, using the index register
                    // make sure we're not drawing out of the screen
                    if i >= YPX as u16 {
                        trace!("attempt to draw out of the screen catched !");
                        continue;
                    }

                    let px_row = self.memory[(self.index_register + (i - y) as u16) as usize];

                    for j in x..x + 8 {
                        // make sure we're not drawing out of the screen
                        if j >= XPX as u16 {
                            trace!("attempt to draw out of the screen catched !");
                            continue;
                        }

                        // evaluate the value of the pixel
                        // 0x80 >> (j - x) will get evaluated like that:
                        // 10000000
                        // 01000000
                        // 00100000 ...
                        // with the and operator, we can ensure the pixel is set if the resulting
                        // value is different from 0
                        if px_row & (0x80 >> (j - x)) != 0 {
                            // collision detected
                            if self.display[j as usize][i as usize] == 1 {
                                self.register[15] = 1; // update the F register accordingly
                                trace!("collision detected at ({};{})", i, j);
                            }
                            // the pixel needs to change apply the xor operator
                            self.display[j as usize][i as usize] ^= 1;
                        }
                    }
                }

                trace!("finished drawing call.");
            }

            // multiple functions exist here, so we need another match
            0xE000 => {
                match opcode & 0x00FF {
                    // conditional based on input: skip next instruction if the key is pressed
                    0x009E => {
                        trace!("key pressed: {:?}", self.key);

                        let register_number = (opcode & 0x0F00) >> 8;
                        let keycode = self.register[register_number as usize];

                        debug!(
                            "checking if key {:X} contained in register {:X} is pressed",
                            keycode, register_number
                        );

                        if self.key[keycode as usize] == 1 {
                            self.program_counter += 2;
                            debug!("the key was pressed: skipping next instruction.");
                        } else {
                            debug!("the key wasn't pressed, nothing to do.");
                        }
                    }

                    // conditional based on input: skip next instruction if the key isn't pressed
                    0x00A1 => {
                        trace!("key pressed: {:?}", self.key);

                        let register_number = (opcode & 0x0F00) >> 8;
                        let keycode = self.register[register_number as usize];

                        debug!(
                            "checking if key {:X} contained in register {:X} is not pressed",
                            keycode, register_number
                        );

                        if self.key[keycode as usize] != 1 {
                            self.program_counter += 2;
                            debug!("the key wasn't pressed: skipping next instruction.");
                        } else {
                            debug!("the key was pressed, nothing to do.");
                        }
                    }

                    _ => warn!("warning: ran into unknown opcode: {:X}", opcode),
                }
            }

            // multiple functions exist here, so we need another match
            0xF000 => {
                match opcode & 0x00FF {
                    // set a register to the value of the delay timer
                    0x0007 => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        self.register[register_number as usize] = self.delay_timer;
                        debug!(
                            "register {:X} set to the value of the delay timer ({})",
                            register_number, self.delay_timer
                        );
                    }

                    // block program execution until one key is pressed
                    0x000A => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        self.wait_for_key = true;
                        self.wait_for_key_register = register_number as usize;
                        debug!(
                            "waiting for key; the key will be stored in register {:X}",
                            register_number,
                        );
                    }

                    // set the value of the delay timer
                    0x0015 => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        self.delay_timer = self.register[register_number as usize];
                        debug!(
                            "setting the delay timer to the value {} of the register {:X}",
                            self.delay_timer, register_number
                        );
                    }

                    // set the value of the sound timer
                    0x0018 => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        self.sound_timer = self.register[register_number as usize];
                        debug!(
                            "setting the sound timer to the value {} of the register {:X}",
                            self.delay_timer, register_number
                        );
                    }

                    // add the register value to the index register
                    0x001E => {
                        let register_number = (opcode & 0x0F00) >> 8;
                        self.index_register += self.register[register_number as usize] as u16;
                        debug!(
                            "setting index register to register {:X} value of {}",
                            register_number, self.register[register_number as usize]
                        );
                    }

                    // set the index register to the font sprite address of the character contained in the register
                    0x0029 => {
                        let character = self.register[((opcode & 0x0F00) >> 8) as usize] as u16;
                        self.index_register = 5 * character;

                        debug!(
                            "storing in the index register the address of the character {}",
                            character
                        );
                        debug!("character address: {}", 5 * character);
                    }

                    // fill the registers with data
                    0x0065 => {
                        let registers = (opcode & 0x0F00) >> 8;

                        debug!(
                            "filling registeries from 0 to {:X} of data stored at address {}",
                            registers, self.index_register
                        );

                        for i in 0..registers {
                            self.register[i as usize] =
                                self.memory[(self.index_register + i) as usize];
                            trace!("new value of {:X}: {}", i, self.register[i as usize]);
                        }
                    }

                    _ => warn!("warning: ran into unknown opcode: {:X}", opcode),
                }
            }

            _ => warn!("warning: ran into unknown opcode: {:X}", opcode),
        }

        Ok(())
    }
}
