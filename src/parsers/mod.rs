use crate::{EngineResult, Line};
use std::cmp::{max, min};

use super::{AttributedChar, Buffer, Caret, Position};

mod parser_errors;
pub use parser_errors::*;

pub mod ansi;
pub mod ascii;
pub mod atascii;
pub mod avatar;
pub mod pcboard;
pub mod petscii;
pub mod rip;
pub mod viewdata;

pub const BEL: char = '\x07';
pub const LF: char = '\n';
pub const CR: char = '\r';
pub const BS: char = '\x08';
pub const FF: char = '\x0C';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicStyle {
    Foreground,
    Background,
    Normal,
    Legato,
    Staccato,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MusicAction {
    PlayNote(f32, u32), // freq / note length
    Pause(u32),
    SetStyle(MusicStyle),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnsiMusic {
    pub music_actions: Vec<MusicAction>,
}

#[derive(Debug, PartialEq)]
pub enum CallbackAction {
    None,
    Beep,
    SendString(String),
    PlayMusic(AnsiMusic),
    ChangeBaudRate(u32),
}

pub trait BufferParser {
    fn convert_from_unicode(&self, ch: char) -> char;
    fn convert_to_unicode(&self, ch: char) -> char;

    /// Prints a character to the buffer. Gives back an optional string returned to the sender (in case for terminals).
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn print_char(
        &mut self,
        buffer: &mut Buffer,
        caret: &mut Caret,
        c: char,
    ) -> EngineResult<CallbackAction>;
}

impl Caret {
    /// (line feed, LF, \n, ^J), moves the print head down one line, or to the left edge and down. Used as the end of line marker in most UNIX systems and variants.
    pub fn lf(&mut self, buf: &mut Buffer) {
        let was_ooe = self.pos.y > buf.get_last_editable_line();

        self.pos.x = 0;
        self.pos.y += 1;
        while self.pos.y >= buf.layers[0].lines.len() as i32 {
            let len = buf.layers[0].lines.len();
            buf.layers[0].lines.insert(len, Line::new());
        }
        if !buf.is_terminal_buffer {
            return;
        }
        if was_ooe {
            buf.terminal_state.limit_caret_pos(buf, self);
        } else {
            self.check_scrolling_on_caret_down(buf, false);
        }
    }

    /// (form feed, FF, \f, ^L), to cause a printer to eject paper to the top of the next page, or a video terminal to clear the screen.
    pub fn ff(&mut self, buf: &mut Buffer) {
        buf.terminal_state.reset();
        buf.clear();
        self.pos = Position::default();
        self.is_visible = true;
        self.attr = super::TextAttribute::default();
    }

    /// (carriage return, CR, \r, ^M), moves the printing position to the start of the line.
    pub fn cr(&mut self, _buf: &Buffer) {
        self.pos.x = 0;
    }

    pub fn eol(&mut self, buf: &Buffer) {
        self.pos.x = buf.get_buffer_width() - 1;
    }

    pub fn home(&mut self, buf: &Buffer) {
        self.pos = buf.upper_left_position();
    }

    /// (backspace, BS, \b, ^H), may overprint the previous character
    pub fn bs(&mut self, buf: &mut Buffer) {
        self.pos.x = max(0, self.pos.x - 1);
        buf.set_char(0, self.pos, Some(AttributedChar::new(' ', self.attr)));
    }

    pub fn del(&mut self, buf: &mut Buffer) {
        if let Some(line) = buf.layers[0].lines.get_mut(self.pos.y as usize) {
            let i = self.pos.x as usize;
            if i < line.chars.len() {
                line.chars.remove(i);
            }
        }
    }

    pub fn ins(&mut self, buf: &mut Buffer) {
        if let Some(line) = buf.layers[0].lines.get_mut(self.pos.y as usize) {
            let i = self.pos.x as usize;
            if i < line.chars.len() {
                line.chars
                    .insert(i, Some(AttributedChar::new(' ', self.attr)));
            }
        }
    }

    pub fn erase_charcter(&mut self, buf: &mut Buffer, number: i32) {
        let mut i = self.pos.x as usize;
        let number = min(buf.get_buffer_width() - i as i32, number);
        if number <= 0 {
            return;
        }
        if let Some(line) = buf.layers[0].lines.get_mut(self.pos.y as usize) {
            for _ in 0..number {
                line.set_char(i as i32, Some(AttributedChar::new(' ', self.attr)));
                i += 1;
            }
        }
    }

    pub fn left(&mut self, buf: &Buffer, num: i32) {
        self.pos.x = self.pos.x.saturating_sub(num);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    pub fn right(&mut self, buf: &mut Buffer, num: i32) {
        self.pos.x += num;
        if self.pos.x > buf.get_buffer_width() && self.pos.y < buf.get_last_editable_line() {
            self.pos.y += self.pos.x / buf.get_buffer_width();
            while self.pos.y >= buf.layers[0].lines.len() as i32 {
                let len = buf.layers[0].lines.len();
                buf.layers[0].lines.insert(len, Line::new());
            }
            self.pos.x %= buf.get_buffer_width();
        }
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    pub fn up(&mut self, buf: &mut Buffer, num: i32) {
        self.pos.y = self.pos.y.saturating_sub(num);
        self.check_scrolling_on_caret_up(buf, false);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    pub fn down(&mut self, buf: &mut Buffer, num: i32) {
        self.pos.y += num;
        self.check_scrolling_on_caret_down(buf, false);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    /// Moves the cursor down one line in the same column. If the cursor is at the bottom margin, the page scrolls up.
    pub fn index(&mut self, buf: &mut Buffer) {
        self.pos.y += 1;
        self.check_scrolling_on_caret_down(buf, true);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    /// Moves the cursor up one line in the same column. If the cursor is at the top margin, the page scrolls down.
    pub fn reverse_index(&mut self, buf: &mut Buffer) {
        self.pos.y = self.pos.y.saturating_sub(1);
        self.check_scrolling_on_caret_up(buf, true);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    pub fn next_line(&mut self, buf: &mut Buffer) {
        self.pos.y += 1;
        self.pos.x = 0;
        self.check_scrolling_on_caret_down(buf, true);
        buf.terminal_state.limit_caret_pos(buf, self);
    }

    fn check_scrolling_on_caret_up(&mut self, buf: &mut Buffer, force: bool) {
        if buf.needs_scrolling() || force {
            while self.pos.y < buf.get_first_editable_line() {
                buf.scroll_down();
                self.pos.y += 1;
            }
        }
    }

    fn check_scrolling_on_caret_down(&mut self, buf: &mut Buffer, force: bool) {
        if (buf.needs_scrolling() || force) && self.pos.y > buf.get_last_editable_line() {
            buf.scroll_up();
            self.pos.y -= 1;
        }
    }
}

impl Buffer {
    fn print_value(&mut self, caret: &mut Caret, ch: u16) {
        let ch = AttributedChar::new(char::from_u32(ch as u32).unwrap(), caret.attr);
        self.print_char(caret, ch);
    }

    fn print_char(&mut self, caret: &mut Caret, ch: AttributedChar) {
        if caret.insert_mode {
            let layer = &mut self.layers[0];
            if layer.lines.len() < caret.pos.y as usize + 1 {
                layer.lines.resize(caret.pos.y as usize + 1, Line::new());
            }
            layer.lines[caret.pos.y as usize]
                .insert_char(caret.pos.x, Some(AttributedChar::default()));
        }
        if caret.pos.x >= self.get_buffer_width() {
            if let crate::AutoWrapMode::AutoWrap = self.terminal_state.auto_wrap_mode {
                caret.lf(self);
            } else {
                caret.pos.x -= 1;
            }
        }

        self.set_char(0, caret.pos, Some(ch));

        caret.pos.x += 1;
    }

    /*fn get_buffer_last_line(&mut self) -> i32
    {
        if let Some((_, end)) = self.terminal_state.margins {
            self.get_first_visible_line() + end
        } else {
            max(self.terminal_state.height, self.layers[0].lines.len() as i32)
        }
    }*/

    fn scroll_up(&mut self) {
        let start_line: i32 = self.get_first_editable_line();
        let end_line = self.get_last_editable_line();

        let start_column = self.get_first_editable_column();
        let end_column = self.get_last_editable_column();

        for layer in &mut self.layers {
            for x in start_column..=end_column {
                (start_line..end_line).for_each(|y| {
                    let ch = layer.get_char(Position::new(x, y + 1));
                    layer.set_char(Position::new(x, y), ch);
                });
                layer.set_char(Position::new(x, end_line), Some(AttributedChar::default()));
            }
        }
    }

    fn scroll_down(&mut self) {
        let start_line: i32 = self.get_first_editable_line();
        let end_line = self.get_last_editable_line();

        let start_column = self.get_first_editable_column();
        let end_column = self.get_last_editable_column();

        for layer in &mut self.layers {
            for x in start_column..=end_column {
                ((start_line + 1)..=end_line).rev().for_each(|y| {
                    let ch = layer.get_char(Position::new(x, y - 1));
                    layer.set_char(Position::new(x, y), ch);
                });
                layer.set_char(
                    Position::new(x, start_line),
                    Some(AttributedChar::default()),
                );
            }
        }
    }

    fn scroll_left(&mut self) {
        let start_line: i32 = self.get_first_editable_line();
        let end_line = self.get_last_editable_line();

        let start_column = self.get_first_editable_column() as usize;
        let end_column = self.get_last_editable_column() as usize + 1;

        for layer in &mut self.layers {
            for i in start_line..=end_line {
                let line = &mut layer.lines[i as usize];
                if line.chars.len() > start_column {
                    line.chars
                        .insert(end_column, Some(AttributedChar::default()));
                    line.chars.remove(start_column);
                }
            }
        }
    }

    fn scroll_right(&mut self) {
        let start_line = self.get_first_editable_line();
        let end_line = self.get_last_editable_line();

        let start_column = self.get_first_editable_column() as usize;
        let end_column = self.get_last_editable_column() as usize;

        for layer in &mut self.layers {
            for i in start_line..=end_line {
                let line = &mut layer.lines[i as usize];
                if line.chars.len() > start_column {
                    line.chars
                        .insert(start_column, Some(AttributedChar::default()));
                    line.chars.remove(end_column + 1);
                }
            }
        }
    }

    fn clear_screen(&mut self, caret: &mut Caret) {
        caret.pos = Position::default();
        self.clear();
        self.clear_buffer_down(caret);
    }

    fn clear_buffer_down(&mut self, caret: &Caret) {
        let pos = caret.get_position();
        let mut ch = AttributedChar::default();
        ch.attribute = caret.attr;

        for y in pos.y..self.get_last_visible_line() {
            for x in 0..self.get_buffer_width() {
                self.set_char(0, Position::new(x, y), Some(ch));
            }
        }
    }

    fn clear_buffer_up(&mut self, caret: &Caret) {
        let pos = caret.get_position();
        let mut ch = AttributedChar::default();
        ch.attribute = caret.attr;

        for y in self.get_first_visible_line()..pos.y {
            for x in 0..self.get_buffer_width() {
                self.set_char(0, Position::new(x, y), Some(ch));
            }
        }
    }

    fn clear_line(&mut self, caret: &Caret) {
        let mut pos = caret.get_position();
        let mut ch = AttributedChar::default();
        ch.attribute = caret.attr;
        for x in 0..self.get_buffer_width() {
            pos.x = x;
            self.set_char(0, pos, Some(ch));
        }
    }

    fn clear_line_end(&mut self, caret: &Caret) {
        let mut pos = caret.get_position();
        let mut ch = AttributedChar::default();
        ch.attribute = caret.attr;
        for x in pos.x..self.get_buffer_width() {
            pos.x = x;
            self.set_char(0, pos, Some(ch));
        }
    }

    fn clear_line_start(&mut self, caret: &Caret) {
        let mut pos = caret.get_position();
        let mut ch = AttributedChar::default();
        ch.attribute = caret.attr;
        for x in 0..pos.x {
            pos.x = x;
            self.set_char(0, pos, Some(ch));
        }
    }

    fn remove_terminal_line(&mut self, line: i32) {
        if line >= self.layers[0].lines.len() as i32 {
            return;
        }
        self.layers[0].remove_line(line);
        if let Some((_, end)) = self.terminal_state.margins_up_down {
            self.layers[0].insert_line(end, Line::new());
        }
    }

    fn insert_terminal_line(&mut self, line: i32) {
        if let Some((_, end)) = self.terminal_state.margins_up_down {
            if end < self.layers[0].lines.len() as i32 {
                self.layers[0].lines.remove(end as usize);
            }
        }
        self.layers[0].insert_line(line, Line::new());
    }
}

fn _get_string_from_buffer(buf: &Buffer) -> String {
    let converted = crate::convert_to_asc(buf, &crate::SaveOptions::new()).unwrap(); // test code
    let b: Vec<u8> = converted
        .iter()
        .map(|&x| if x == 27 { b'x' } else { x })
        .collect();
    let converted = String::from_utf8_lossy(b.as_slice());

    converted.to_string()
}

#[cfg(test)]
fn create_buffer<T: BufferParser>(parser: &mut T, input: &[u8]) -> (Buffer, Caret) {
    let mut buf = Buffer::create(80, 25);
    let mut caret = Caret::default();
    // remove editing layer
    buf.is_terminal_buffer = true;
    buf.layers.remove(0);
    buf.layers[0].is_locked = false;
    buf.layers[0].is_transparent = false;
    buf.layers.first_mut().unwrap().lines.clear();

    update_buffer(&mut buf, &mut caret, parser, input);

    (buf, caret)
}

#[cfg(test)]
fn update_buffer<T: BufferParser>(
    buf: &mut Buffer,
    caret: &mut Caret,
    parser: &mut T,
    input: &[u8],
) {
    for b in input {
        if let Some(ch) = char::from_u32(*b as u32) {
            parser.print_char(buf, caret, ch).unwrap(); // test code
        }
    }
}

#[cfg(test)]
fn get_simple_action<T: BufferParser>(parser: &mut T, input: &[u8]) -> CallbackAction {
    let mut buf = Buffer::create(80, 25);
    let mut caret = Caret::default();
    buf.is_terminal_buffer = true;
    buf.layers.remove(0);
    buf.layers[0].is_locked = false;
    buf.layers[0].is_transparent = false;

    get_action(&mut buf, &mut caret, parser, input)
}

#[cfg(test)]
fn get_action<T: BufferParser>(
    buf: &mut Buffer,
    caret: &mut Caret,
    parser: &mut T,
    input: &[u8],
) -> CallbackAction {
    // remove editing layer

    let mut action = CallbackAction::None;
    for b in input {
        if let Some(ch) = char::from_u32(*b as u32) {
            action = parser.print_char(buf, caret, ch).unwrap(); // test code
        }
    }

    action
}
