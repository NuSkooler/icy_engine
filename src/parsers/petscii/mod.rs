use super::{ascii, Buffer, BufferParser};
use crate::{AttributedChar, CallbackAction, Caret, EngineResult, ParserError, Position};

#[derive(Default)]
pub struct Parser {
    underline_mode: bool,
    reverse_mode: bool,
    got_esc: bool,
    shift_mode: bool,
    c_shift: bool,
}

impl Parser {
    pub fn handle_reverse_mode(&self, ch: u8) -> u8 {
        if self.reverse_mode {
            ch + 0x80
        } else {
            ch
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn handle_c128_escapes(
        &mut self,
        buf: &mut Buffer,
        caret: &mut Caret,
        ch: u8,
    ) -> EngineResult<CallbackAction> {
        self.got_esc = false;

        match ch {
            b'O' => {} // Cancel quote and insert mode
            b'Q' => {
                buf.clear_line_end(caret);
            } // Erase to end of current line
            b'P' => {
                buf.clear_line_start(caret);
            } // Cancel quote and insert mode
            b'@' => {
                buf.clear_buffer_down(caret);
            } // Erase to end of screen

            b'J' => {
                caret.cr(buf);
            } // Move to start of current line
            b'K' => {
                caret.eol(buf);
            } // Move to end of current line

            b'A' => {
                eprintln!("enable auto insert mode unsupported.");
            } // Enable auto-insert mode
            b'C' => {
                eprintln!("disable auto insert mode unsupported.");
            } // Disable auto-insert mode

            b'D' => {
                buf.remove_terminal_line(caret.pos.y);
            } // Delete current line
            b'I' => {
                buf.insert_terminal_line(caret.pos.y);
            } // Insert line

            b'Y' => {
                eprintln!("Set default tab stops (8 spaces) unsupported.");
            } // Set default tab stops (8 spaces)
            b'Z' => {
                eprintln!("Clear all tab stops unsupported.");
            } // Clear all tab stops

            b'L' => {
                eprintln!("Enable scrolling unsupported.");
            } // Enable scrolling
            b'M' => {
                eprintln!("Disable scrolling unsupported.");
            } // Disable scrolling

            b'V' => {
                eprintln!("Scroll up unsupported.");
            } // Scroll up
            b'W' => {
                eprintln!("Scroll down unsupported.");
            } // Scroll down

            b'G' => {
                eprintln!("Enable bell unsupported.");
            } // Enable bell (by CTRL G)
            b'H' => {
                eprintln!("Disable bell unsupported.");
            } // Disable bell

            b'E' => {
                eprintln!("Set cursor to non-flashing mode unsupported.");
            } // Set cursor to non-flashing mode
            b'F' => {
                eprintln!("Set cursor to flashing mode unsupported.");
            } // Set cursor to flashing mode

            b'B' => {
                eprintln!("Set bottom of screen window at cursor position unsupported.");
            } // Set bottom of screen window at cursor position
            b'T' => {
                eprintln!("Set top of screen window at cursor position unsupported.");
            } // Set top of screen window at cursor position

            b'X' => {
                eprintln!("Swap 40/80 column display output device unsupported.");
            } // Swap 40/80 column display output device

            b'U' => {
                eprintln!("Change to underlined cursor unsupported.");
            } // Change to underlined cursor
            b'S' => {
                eprintln!("Change to block cursor unsupported.");
            } // Change to block cursor

            b'R' => {
                eprintln!("Set screen to reverse video unsupported.");
            } // Set screen to reverse video
            b'N' => {
                eprintln!("Set screen to normal (non reverse video) state unsupported.");
            } // Set screen to normal (non reverse video) state

            _ => {
                eprintln!(
                    "Unknown C128 escape code: 0x{:02X}/{:?} ",
                    ch,
                    char::from_u32(ch as u32)
                );
            }
        }
        Ok(CallbackAction::None)
    }

    pub fn update_shift_mode(&mut self, buf: &mut Buffer, shift_mode: bool) {
        if self.shift_mode == shift_mode {
            return;
        }
        self.shift_mode = shift_mode;
        for y in 0..buf.get_buffer_height() {
            for x in 0..buf.get_buffer_width() {
                if let Some(ch) = &mut buf.get_char(Position::new(x, y)) {
                    ch.set_font_page(usize::from(shift_mode));
                    buf.set_char(0, Position::new(x, y), Some(*ch));
                }
            }
        }
    }
}

const BLACK: u32 = 0x00;
const WHITE: u32 = 0x01;
const RED: u32 = 0x02;
const CYAN: u32 = 0x03;
const PURPLE: u32 = 0x04;
const GREEN: u32 = 0x05;
const BLUE: u32 = 0x06;
const YELLOW: u32 = 0x07;
const ORANGE: u32 = 0x08;
const BROWN: u32 = 0x09;
const PINK: u32 = 0x0a;
const GREY1: u32 = 0x0b;
const GREY2: u32 = 0x0c;
const LIGHT_GREEN: u32 = 0x0d;
const LIGHT_BLUE: u32 = 0x0e;
const GREY3: u32 = 0x0f;

impl BufferParser for Parser {
    fn convert_from_unicode(&self, ch: char) -> char {
        if let Some(tch) = UNICODE_TO_PETSCII.get(&(ch as u8)) {
            if let Some(out_ch) = char::from_u32(*tch as u32) {
                out_ch
            } else {
                ch
            }
        } else {
            ch
        }
    }

    fn convert_to_unicode(&self, ch: char) -> char {
        // TODO
        ascii::Parser::default().convert_to_unicode(ch)
    }

    fn print_char(
        &mut self,
        buf: &mut Buffer,
        caret: &mut Caret,
        ch: char,
    ) -> EngineResult<CallbackAction> {
        let ch = ch as u8;
        if self.got_esc {
            return self.handle_c128_escapes(buf, caret, ch);
        }

        match ch {
            0x02 => self.underline_mode = true, // C128
            0x05 => caret.set_foreground(WHITE),
            0x07 => return Ok(CallbackAction::Beep),
            0x08 => self.c_shift = false,
            0x09 => self.c_shift = true,
            0x0A => caret.cr(buf),
            0x0D | 0x8D => {
                caret.lf(buf);
                self.reverse_mode = false;
            }
            0x0E => self.update_shift_mode(buf, false),
            0x11 => caret.down(buf, 1),
            0x12 => self.reverse_mode = true,
            0x13 => caret.home(buf),
            0x14 => caret.bs(buf),
            0x1B => self.got_esc = true,
            0x1C => caret.set_foreground(RED),
            0x1D => caret.right(buf, 1),
            0x1E => caret.set_foreground(GREEN),
            0x1F => caret.set_foreground(BLUE),
            0x81 => caret.set_foreground(ORANGE),
            0x8E => self.update_shift_mode(buf, true),
            0x90 => caret.set_foreground(BLACK),
            0x91 => caret.up(buf, 1),
            0x92 => self.reverse_mode = false,
            0x93 => {
                buf.clear_screen(caret);
            }
            0x95 => caret.set_foreground(BROWN),
            0x96 => caret.set_foreground(PINK),
            0x97 => caret.set_foreground(GREY1),
            0x98 => caret.set_foreground(GREY2),
            0x99 => caret.set_foreground(LIGHT_GREEN),
            0x9A => caret.set_foreground(LIGHT_BLUE),
            0x9B => caret.set_foreground(GREY3),
            0x9C => caret.set_foreground(PURPLE),
            0x9D => caret.left(buf, 1),
            0x9E => caret.set_foreground(YELLOW),
            0x9F => caret.set_foreground(CYAN),
            0xFF => buf.print_value(caret, 94), // PI character
            _ => {
                let tch = match ch {
                    0x20..=0x3F => ch,
                    0x40..=0x5F | 0xA0..=0xBF => ch - 0x40,
                    0x60..=0x7F => ch - 0x20,
                    0xC0..=0xFE => ch - 0x80,
                    _ => {
                        return Err(Box::new(ParserError::UnsupportedControlCode(ch as u32)));
                    }
                };
                let mut ch = AttributedChar::new(
                    char::from_u32(self.handle_reverse_mode(tch) as u32).unwrap(),
                    caret.attr,
                );
                ch.set_font_page(usize::from(self.shift_mode));
                buf.print_char(caret, ch);
            }
        }
        Ok(CallbackAction::None)
    }
}

lazy_static::lazy_static! {
    static ref UNICODE_TO_PETSCII: std::collections::HashMap<u8,u8> = vec![
        (0x41, 0x61),
        (0x42, 0x62),
        (0x43, 0x63),
        (0x44, 0x64),
        (0x45, 0x65),
        (0x46, 0x66),
        (0x47, 0x67),
        (0x48, 0x68),
        (0x49, 0x69),
        (0x4A, 0x6A),
        (0x4B, 0x6B),
        (0x4C, 0x6C),
        (0x4D, 0x6D),
        (0x4E, 0x6E),
        (0x4F, 0x6F),
        (0x50, 0x70),
        (0x51, 0x71),
        (0x52, 0x72),
        (0x53, 0x73),
        (0x54, 0x74),
        (0x55, 0x75),
        (0x56, 0x76),
        (0x57, 0x77),
        (0x58, 0x78),
        (0x59, 0x79),
        (0x5A, 0x7A),
        (0x5C, 0x9C),
        (0x5E, 0x18),
        (0x5F, 0x1B),
        (0x60, 0xC4),
        (0x61, 0x41),
        (0x62, 0x42),
        (0x63, 0x43),
        (0x64, 0x44),
        (0x65, 0x45),
        (0x66, 0x46),
        (0x67, 0x47),
        (0x68, 0x48),
        (0x69, 0x49),
        (0x6A, 0x4A),
        (0x6B, 0x4B),
        (0x6C, 0x4C),
        (0x6D, 0x4D),
        (0x6E, 0x4E),
        (0x6F, 0x4F),
        (0x70, 0x50),
        (0x71, 0x51),
        (0x72, 0x52),
        (0x73, 0x53),
        (0x74, 0x54),
        (0x75, 0x55),
        (0x76, 0x56),
        (0x77, 0x57),
        (0x78, 0x58),
        (0x79, 0x59),
        (0x7A, 0x5A),
        (0x7B, 0xC5),
        (0x7C, 0xB5),
        (0x7D, 0xB3),
        (0x7E, 0xB2),
        (0x7F, 0xB0),
        (0xA0, 0xFF),
        (0xA1, 0xDD),
        (0xA2, 0xDC),
        (0xA3, 0x5E),
        (0xA4, 0x5F),
        (0xA5, 0x7B),
        (0xA6, 0xB1),
        (0xA7, 0x7D),
        (0xA8, 0xD2),
        (0xA9, 0x1F),
        (0xAA, 0xF5),
        (0xAB, 0xC3),
        (0xAC, 0xC9),
        (0xAD, 0xC0),
        (0xAE, 0xBF),
        (0xAF, 0xCD),
        (0xB0, 0xDA),
        (0xB1, 0xC1),
        (0xB2, 0xC2),
        (0xB3, 0xB4),
        (0xB4, 0xF4),
        (0xB5, 0xB9),
        (0xB6, 0xDE),
        (0xB7, 0xA9),
        (0xB8, 0xDF),
        (0xB9, 0x16),
        (0xBA, 0xFB),
        (0xBC, 0xC8),
        (0xBD, 0xD9),
        (0xBE, 0xBC),
        (0xBF, 0xCE),
    ].into_iter().collect();
}
