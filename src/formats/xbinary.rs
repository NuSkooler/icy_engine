use std::{cmp::min, io};

use crate::{AttributedChar, BitFont, Buffer, BufferType, Palette, SauceString};

use super::{CompressionLevel, Position, SaveOptions, TextAttribute};

const XBIN_HEADER_SIZE: usize = 11;

const FLAG_PALETTE: u8 = 0b_0000_0001;
const FLAG_FONT: u8 = 0b_0000_0010;
const FLAG_COMPRESS: u8 = 0b_0000_0100;
const FLAG_NON_BLINK_MODE: u8 = 0b_0000_1000;
const FLAG_512CHAR_MODE: u8 = 0b_0001_0000;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum Compression {
    Off = 0b0000_0000,
    Char = 0b0100_0000,
    Attr = 0b1000_0000,
    Full = 0b1100_0000,
}

/// .
///
/// # Errors
///
/// This function will return an error if .
pub fn read_xb(result: &mut Buffer, bytes: &[u8], file_size: usize) -> io::Result<bool> {
    if file_size < XBIN_HEADER_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid XBin.\nFile too short.",
        ));
    }
    if b"XBIN" != &bytes[0..4] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid XBin.\nID doesn't match.",
        ));
    }

    let mut o = 4;

    // let eof_char = bytes[o];
    o += 1;
    result.set_buffer_width(bytes[o] as i32 + ((bytes[o + 1] as i32) << 8));
    o += 2;
    result.set_buffer_height(bytes[o] as i32 + ((bytes[o + 1] as i32) << 8));
    o += 2;

    let font_size = bytes[o];
    o += 1;
    let flags = bytes[o];
    o += 1;

    let has_custom_palette = (flags & FLAG_PALETTE) == FLAG_PALETTE;
    let has_custom_font = (flags & FLAG_FONT) == FLAG_FONT;
    let is_compressed = (flags & FLAG_COMPRESS) == FLAG_COMPRESS;
    let use_ice = (flags & FLAG_NON_BLINK_MODE) == FLAG_NON_BLINK_MODE;
    let extended_char_mode = (flags & FLAG_512CHAR_MODE) == FLAG_512CHAR_MODE;

    if extended_char_mode {
        result.buffer_type = if use_ice {
            BufferType::ExtFontIce
        } else {
            BufferType::ExtFont
        };
    } else {
        result.buffer_type = if use_ice {
            BufferType::LegacyIce
        } else {
            BufferType::LegacyDos
        };
    }

    if has_custom_palette {
        result.palette = Palette::from(&bytes[o..(o + 48)]);
        o += 48;
    }
    if has_custom_font {
        let font_length = font_size as usize * 256;
        result.clear_font_table();
        result.set_font(
            0,
            BitFont::create_8(
                SauceString::new(),
                8,
                font_size,
                &bytes[o..(o + font_length)],
            ),
        );
        o += font_length;
        if extended_char_mode {
            result.set_font(
                1,
                BitFont::create_8(
                    SauceString::new(),
                    8,
                    font_size,
                    &bytes[o..(o + font_length)],
                ),
            );
            o += font_length;
        }
    }

    if is_compressed {
        read_data_compressed(result, &bytes[o..], file_size - o)
    } else {
        read_data_uncompressed(result, &bytes[o..], file_size - o)
    }
}

fn advance_pos(result: &Buffer, pos: &mut Position) -> bool {
    pos.x += 1;
    if pos.x >= result.get_buffer_width() {
        pos.x = 0;
        pos.y += 1;
    }
    true
}

fn read_data_compressed(result: &mut Buffer, bytes: &[u8], file_size: usize) -> io::Result<bool> {
    let mut pos = Position::default();
    let mut o = 0;
    while o < file_size {
        let xbin_compression = bytes[o];
        if o > file_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Invalid XBin.\nRead block start at EOF.",
            ));
        }

        o += 1;
        let compression = unsafe { std::mem::transmute(xbin_compression & 0b_1100_0000) };
        let repeat_counter = (xbin_compression & 0b_0011_1111) + 1;

        match compression {
            Compression::Off => {
                for _ in 0..repeat_counter {
                    if o + 2 > bytes.len() {
                        eprintln!("Invalid XBin. Read char block beyond EOF.");
                        break;
                    }
                    let char_code = bytes[o];
                    let attribute = bytes[o + 1];
                    o += 2;
                    result.set_char(
                        0,
                        pos,
                        Some(decode_char(char_code, attribute, result.buffer_type)),
                    );

                    if !advance_pos(result, &mut pos) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "data out of bounds",
                        ));
                    }
                }
            }
            Compression::Char => {
                let char_code = bytes[o];
                o += 1;
                for _ in 0..repeat_counter {
                    if o + 1 > bytes.len() {
                        eprintln!("Invalid XBin. Read char compression block beyond EOF.");
                        break;
                    }

                    result.set_char(
                        0,
                        pos,
                        Some(decode_char(char_code, bytes[o], result.buffer_type)),
                    );
                    o += 1;
                    if !advance_pos(result, &mut pos) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "data out of bounds",
                        ));
                    }
                }
            }
            Compression::Attr => {
                let attribute = bytes[o];
                o += 1;
                for _ in 0..repeat_counter {
                    if o + 1 > bytes.len() {
                        eprintln!("Invalid XBin. Read attribute compression block beyond EOF.");
                        break;
                    }
                    result.set_char(
                        0,
                        pos,
                        Some(decode_char(bytes[o], attribute, result.buffer_type)),
                    );
                    o += 1;
                    if !advance_pos(result, &mut pos) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "data out of bounds",
                        ));
                    }
                }
            }
            Compression::Full => {
                let char_code = bytes[o];
                o += 1;
                if o + 1 > bytes.len() {
                    eprintln!("Invalid XBin. nRead compression block beyond EOF.");
                    break;
                }
                let attr = bytes[o];
                o += 1;
                let rep_ch = Some(decode_char(char_code, attr, result.buffer_type));

                for _ in 0..repeat_counter {
                    result.set_char(0, pos, rep_ch);
                    if !advance_pos(result, &mut pos) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "data out of bounds",
                        ));
                    }
                }
            }
        }
    }

    Ok(true)
}

fn decode_char(char_code: u8, attr: u8, buffer_type: BufferType) -> AttributedChar {
    let mut attribute = TextAttribute::from_u8(attr, buffer_type);
    if buffer_type.use_extended_font() && (attr & 0b_1000) != 0 {
        attribute.set_foreground(attribute.get_foreground());
        AttributedChar::new(
            char::from_u32(char_code as u32 | 1 << 9).unwrap(),
            attribute,
        )
    } else {
        AttributedChar::new(char::from_u32(char_code as u32).unwrap(), attribute)
    }
}

fn encode_attr(char_code: u16, attr: TextAttribute, buffer_type: BufferType) -> u8 {
    if buffer_type.use_extended_font() {
        attr.as_u8(buffer_type) | if char_code > 255 { 0b1000 } else { 0 }
    } else {
        attr.as_u8(buffer_type)
    }
}

fn read_data_uncompressed(result: &mut Buffer, bytes: &[u8], file_size: usize) -> io::Result<bool> {
    let mut pos = Position::default();
    let mut o = 0;
    while o < file_size {
        if o + 1 > file_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Invalid XBin.\n Uncompressed data length needs to be % 2 == 0",
            ));
        }
        result.set_char(
            0,
            pos,
            Some(decode_char(bytes[o], bytes[o + 1], result.buffer_type)),
        );
        o += 2;
        if !advance_pos(result, &mut pos) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "data out of bounds",
            ));
        }
    }

    Ok(true)
}

/// .
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub fn convert_to_xb(buf: &Buffer, options: &SaveOptions) -> io::Result<Vec<u8>> {
    let mut result = Vec::new();

    result.extend_from_slice(b"XBIN");
    result.push(0x1A); // CP/M EOF char (^Z) - used by DOS as well

    result.push(buf.get_buffer_width() as u8);
    result.push((buf.get_buffer_width() >> 8) as u8);
    result.push(buf.get_real_buffer_height() as u8);
    result.push((buf.get_real_buffer_height() >> 8) as u8);

    let mut flags = 0;
    let font = buf.get_font(0).unwrap();
    if font.size.width != 8 || font.size.height < 1 || font.size.height > 32 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "font not supported by the .xb format only fonts with 8px width and a height from 1 to 32 are supported."));
    }

    result.push(font.size.height);
    if !font.is_default() || buf.has_fonts() {
        flags |= FLAG_FONT;
    }

    if !buf.palette.is_default() {
        flags |= FLAG_PALETTE;
    }
    if options.compression_level != CompressionLevel::Off {
        flags |= FLAG_COMPRESS;
    }

    if buf.buffer_type.use_ice_colors() {
        flags |= FLAG_NON_BLINK_MODE;
    }

    if buf.buffer_type.use_extended_font() {
        flags |= FLAG_512CHAR_MODE;
    }

    result.push(flags);

    if (flags & FLAG_PALETTE) == FLAG_PALETTE {
        result.extend(buf.palette.to_16color_vec());
    }

    if flags & FLAG_FONT == FLAG_FONT {
        font.convert_to_u8_data(&mut result);
        if flags & FLAG_512CHAR_MODE == FLAG_512CHAR_MODE {
            if let Some(font) = buf.get_font(0) {
                font.convert_to_u8_data(&mut result);
            }
        }
    }
    match options.compression_level {
        CompressionLevel::Medium => compress_greedy(&mut result, buf, buf.buffer_type),
        CompressionLevel::High => compress_backtrack(&mut result, buf, buf.buffer_type),
        CompressionLevel::Off => {
            for y in 0..buf.get_real_buffer_height() {
                for x in 0..buf.get_buffer_width() {
                    let ch = buf.get_char(Position::new(x, y)).unwrap_or_default();

                    result.push(ch.ch as u8);
                    result.push(encode_attr(ch.ch as u16, ch.attribute, buf.buffer_type));
                }
            }
        }
    }

    if options.save_sauce {
        buf.write_sauce_info(&crate::SauceFileType::XBin, &mut result)?;
    }
    Ok(result)
}

fn compress_greedy(outputdata: &mut Vec<u8>, buffer: &Buffer, buffer_type: BufferType) {
    let mut run_mode = Compression::Off;
    let mut run_count = 0;
    let mut run_buf = Vec::new();
    let mut run_ch = AttributedChar::default();
    let len = buffer.get_real_buffer_height() * buffer.get_buffer_width();
    for x in 0..len {
        let cur = buffer
            .get_char(Position::from_index(buffer, x))
            .unwrap_or_default();

        let next = if x < len - 1 {
            buffer
                .get_char(Position::from_index(buffer, x + 1))
                .unwrap_or_default()
        } else {
            AttributedChar::default()
        };

        if run_count > 0 {
            let mut end_run = false;
            if run_count >= 64 {
                end_run = true;
            } else if run_count > 0 {
                match run_mode {
                    Compression::Off => {
                        if x < len - 2 && cur == next {
                            end_run = true;
                        } else if x < len - 2 {
                            let next2 = buffer
                                .get_char(Position::from_index(buffer, x + 2))
                                .unwrap_or_default();
                            end_run = cur.ch == next.ch && cur.ch == next2.ch
                                || cur.attribute == next.attribute
                                    && cur.attribute == next2.attribute;
                        }
                    }
                    Compression::Char => {
                        if cur.ch != run_ch.ch {
                            end_run = true;
                        } else if x < len - 3 {
                            let next2 = buffer
                                .get_char(Position::from_index(buffer, x + 2))
                                .unwrap_or_default();
                            let next3 = buffer
                                .get_char(Position::from_index(buffer, x + 3))
                                .unwrap_or_default();
                            end_run = cur == next && cur == next2 && cur == next3;
                        }
                    }
                    Compression::Attr => {
                        if cur.attribute != run_ch.attribute {
                            end_run = true;
                        } else if x < len - 3 {
                            let next2 = buffer
                                .get_char(Position::from_index(buffer, x + 2))
                                .unwrap_or_default();
                            let next3 = buffer
                                .get_char(Position::from_index(buffer, x + 3))
                                .unwrap_or_default();
                            end_run = cur == next && cur == next2 && cur == next3;
                        }
                    }
                    Compression::Full => {
                        end_run = cur != run_ch;
                    }
                }
            }

            if end_run {
                outputdata.push((run_mode as u8) | (run_count - 1));
                outputdata.extend(&run_buf);
                run_count = 0;
            }
        }

        if run_count > 0 {
            match run_mode {
                Compression::Off => {
                    run_buf.push(cur.ch as u8);
                    run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                }
                Compression::Char => {
                    run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                }
                Compression::Attr => {
                    run_buf.push(cur.ch as u8);
                }
                Compression::Full => {
                    // nothing
                }
            }
        } else {
            run_buf.clear();
            if x < len - 1 {
                if cur == next {
                    run_mode = Compression::Full;
                } else if cur.ch == next.ch {
                    run_mode = Compression::Char;
                } else if cur.attribute == next.attribute {
                    run_mode = Compression::Attr;
                } else {
                    run_mode = Compression::Off;
                }
            } else {
                run_mode = Compression::Off;
            }

            if let Compression::Attr = run_mode {
                run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                run_buf.push(cur.ch as u8);
            } else {
                run_buf.push(cur.ch as u8);
                run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
            }

            run_ch = cur;
        }
        run_count += 1;
    }

    if run_count > 0 {
        outputdata.push((run_mode as u8) | (run_count - 1));
        outputdata.extend(run_buf);
    }
}

fn count_length(
    mut run_mode: Compression,
    mut run_ch: AttributedChar,
    mut end_run: Option<bool>,
    mut run_count: u8,
    buffer: &Buffer,
    mut x: i32,
) -> i32 {
    let len = min(
        x + 256,
        (buffer.get_real_buffer_height() * buffer.get_buffer_width()) - 1,
    );
    let mut count = 0;
    while x < len {
        let cur = buffer
            .get_char(Position::from_index(buffer, x))
            .unwrap_or_default();
        let next = buffer
            .get_char(Position::from_index(buffer, x + 1))
            .unwrap_or_default();

        if run_count > 0 {
            if end_run.is_none() {
                if run_count >= 64 {
                    end_run = Some(true);
                } else if run_count > 0 {
                    match run_mode {
                        Compression::Off => {
                            if x < len - 2 && cur == next {
                                end_run = Some(true);
                            } else if x < len - 2 {
                                let next2 = buffer
                                    .get_char(Position::from_index(buffer, x + 2))
                                    .unwrap_or_default();
                                end_run = Some(
                                    cur.ch == next.ch && cur.ch == next2.ch
                                        || cur.attribute == next.attribute
                                            && cur.attribute == next2.attribute,
                                );
                            }
                        }
                        Compression::Char => {
                            if cur.ch != run_ch.ch {
                                end_run = Some(true);
                            } else if x < len - 3 {
                                let next2 = buffer
                                    .get_char(Position::from_index(buffer, x + 2))
                                    .unwrap_or_default();
                                let next3 = buffer
                                    .get_char(Position::from_index(buffer, x + 3))
                                    .unwrap_or_default();
                                end_run = Some(cur == next && cur == next2 && cur == next3);
                            }
                        }
                        Compression::Attr => {
                            if cur.attribute != run_ch.attribute {
                                end_run = Some(true);
                            } else if x < len - 3 {
                                let next2 = buffer
                                    .get_char(Position::from_index(buffer, x + 2))
                                    .unwrap_or_default();
                                let next3 = buffer
                                    .get_char(Position::from_index(buffer, x + 3))
                                    .unwrap_or_default();
                                end_run = Some(cur == next && cur == next2 && cur == next3);
                            }
                        }
                        Compression::Full => {
                            end_run = Some(cur != run_ch);
                        }
                    }
                }
            }

            if let Some(true) = end_run {
                count += 1;
                run_count = 0;
            }
        }
        end_run = None;

        if run_count > 0 {
            match run_mode {
                Compression::Off => {
                    count += 2;
                }
                Compression::Char | Compression::Attr => {
                    count += 1;
                }
                Compression::Full => {
                    // nothing
                }
            }
        } else {
            if x < len - 1 {
                if cur == next {
                    run_mode = Compression::Full;
                } else if cur.ch == next.ch {
                    run_mode = Compression::Char;
                } else if cur.attribute == next.attribute {
                    run_mode = Compression::Attr;
                } else {
                    run_mode = Compression::Off;
                }
            } else {
                run_mode = Compression::Off;
            }
            count += 2;
            run_ch = cur;
            end_run = None;
        }
        run_count += 1;
        x += 1;
    }
    count
}

fn compress_backtrack(outputdata: &mut Vec<u8>, buffer: &Buffer, buffer_type: BufferType) {
    let mut run_mode = Compression::Off;
    let mut run_count = 0;
    let mut run_buf = Vec::new();
    let mut run_ch = AttributedChar::default();
    let len = buffer.get_real_buffer_height() * buffer.get_buffer_width();
    for x in 0..len {
        let cur = buffer
            .get_char(Position::from_index(buffer, x))
            .unwrap_or_default();

        let next = if x < len - 1 {
            buffer
                .get_char(Position::from_index(buffer, x + 1))
                .unwrap_or_default()
        } else {
            AttributedChar::default()
        };

        if run_count > 0 {
            let mut end_run = false;
            if run_count >= 64 {
                end_run = true;
            } else if run_count > 0 {
                match run_mode {
                    Compression::Off => {
                        if x < len - 2 && (cur.ch == next.ch || cur.attribute == next.attribute) {
                            let l1 =
                                count_length(run_mode, run_ch, Some(true), run_count, buffer, x);
                            let l2 =
                                count_length(run_mode, run_ch, Some(false), run_count, buffer, x);
                            end_run = l1 < l2;
                        }
                    }
                    Compression::Char => {
                        if cur.ch != run_ch.ch {
                            end_run = true;
                        } else if x < len - 4 {
                            let next2 = buffer
                                .get_char(Position::from_index(buffer, x + 2))
                                .unwrap_or_default();
                            if cur.attribute == next.attribute && cur.attribute == next2.attribute {
                                let l1 = count_length(
                                    run_mode,
                                    run_ch,
                                    Some(true),
                                    run_count,
                                    buffer,
                                    x,
                                );
                                let l2 = count_length(
                                    run_mode,
                                    run_ch,
                                    Some(false),
                                    run_count,
                                    buffer,
                                    x,
                                );
                                end_run = l1 < l2;
                            }
                        }
                    }
                    Compression::Attr => {
                        if cur.attribute != run_ch.attribute {
                            end_run = true;
                        } else if x < len - 3 {
                            let next2 = buffer
                                .get_char(Position::from_index(buffer, x + 2))
                                .unwrap_or_default();
                            if cur.ch == next.ch && cur.ch == next2.ch {
                                let l1 = count_length(
                                    run_mode,
                                    run_ch,
                                    Some(true),
                                    run_count,
                                    buffer,
                                    x,
                                );
                                let l2 = count_length(
                                    run_mode,
                                    run_ch,
                                    Some(false),
                                    run_count,
                                    buffer,
                                    x,
                                );
                                end_run = l1 < l2;
                            }
                        }
                    }
                    Compression::Full => {
                        end_run = cur != run_ch;
                    }
                }
            }

            if end_run {
                outputdata.push((run_mode as u8) | (run_count - 1));
                outputdata.extend(&run_buf);
                run_count = 0;
            }
        }

        if run_count > 0 {
            match run_mode {
                Compression::Off => {
                    run_buf.push(cur.ch as u8);
                    run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                }
                Compression::Char => {
                    run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                }
                Compression::Attr => {
                    run_buf.push(cur.ch as u8);
                }
                Compression::Full => {
                    // nothing
                }
            }
        } else {
            run_buf.clear();
            if x < len - 1 {
                if cur == next {
                    run_mode = Compression::Full;
                } else if cur.ch == next.ch {
                    run_mode = Compression::Char;
                } else if cur.attribute == next.attribute {
                    run_mode = Compression::Attr;
                } else {
                    run_mode = Compression::Off;
                }
            } else {
                run_mode = Compression::Off;
            }

            if let Compression::Attr = run_mode {
                run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
                run_buf.push(cur.ch as u8);
            } else {
                run_buf.push(cur.ch as u8);
                run_buf.push(encode_attr(cur.ch as u16, cur.attribute, buffer_type));
            }

            run_ch = cur;
        }
        run_count += 1;
    }

    if run_count > 0 {
        outputdata.push((run_mode as u8) | (run_count - 1));
        outputdata.extend(run_buf);
    }
}

pub fn get_save_sauce_default_xb(buf: &Buffer) -> (bool, String) {
    if buf.has_sauce_relevant_data() {
        return (true, String::new());
    }

    (false, String::new())
}
