use std::io;

use super::{Position, SaveOptions, TextAttribute};
use crate::{AttributedChar, BitFont, Buffer, BufferType, Palette, Size};

// http://fileformats.archiveteam.org/wiki/ICEDraw

const HEADER_SIZE: usize = 4 + 4 * 2;

const IDF_V1_3_HEADER: &[u8] = b"\x041.3";
const IDF_V1_4_HEADER: &[u8] = b"\x041.4";

const FONT_SIZE: usize = 4096;
const PALETTE_SIZE: usize = 3 * 16;

/// .
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub fn read_idf(result: &mut Buffer, bytes: &[u8], file_size: usize) -> io::Result<bool> {
    if file_size < HEADER_SIZE + FONT_SIZE + PALETTE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid IDF - file too short",
        ));
    }
    let version = &bytes[0..4];

    if version != IDF_V1_3_HEADER && version != IDF_V1_4_HEADER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid IDF or no supported idf version",
        ));
    }

    let mut o = 4;
    let x1 = (bytes[o] as u16 + ((bytes[o + 1] as u16) << 8)) as i32;
    o += 2;
    let y1 = (bytes[o] as u16 + ((bytes[o + 1] as u16) << 8)) as i32;
    o += 2;
    let x2 = (bytes[o] as u16 + ((bytes[o + 1] as u16) << 8)) as i32;
    o += 2;
    // skip y2
    o += 2;

    if x2 < x1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid bounds for idf width needs to be >=0.",
        ));
    }

    result.set_buffer_width(x2 + 1);
    result.buffer_type = BufferType::LegacyIce;
    let data_size = file_size - FONT_SIZE - PALETTE_SIZE;
    let mut pos = Position::new(x1, y1);

    while o + 1 < data_size {
        let mut rle_count = 1;
        let mut char_code = bytes[o];
        o += 1;
        let mut attr = bytes[o];
        o += 1;

        if char_code == 1 && attr == 0 {
            rle_count = bytes[o] as i32 + ((bytes[o + 1] as i32) << 8);

            if o + 3 >= data_size {
                break;
            }
            o += 2;
            char_code = bytes[o];
            o += 1;
            attr = bytes[o];
            o += 1;
        }
        while rle_count > 0 {
            result.set_char(
                0,
                pos,
                Some(AttributedChar::new(
                    char::from_u32(char_code as u32).unwrap(),
                    TextAttribute::from_u8(attr, result.buffer_type),
                )),
            );
            advance_pos(x1, x2, &mut pos);
            rle_count -= 1;
        }
    }
    result.clear();
    result.set_font(0, BitFont::from_basic(8, 16, &bytes[o..(o + FONT_SIZE)]));
    o += FONT_SIZE;

    result.palette = Palette::from(&bytes[o..(o + PALETTE_SIZE)]);

    result.set_height_for_pos(pos);

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
pub fn convert_to_idf(buf: &Buffer, options: &SaveOptions) -> io::Result<Vec<u8>> {
    let mut result = IDF_V1_4_HEADER.to_vec();

    // x1
    result.push(0);
    result.push(0);

    // y1
    result.push(0);
    result.push(0);

    let w = buf.get_buffer_width() - 1;
    result.push(w as u8);
    result.push((w >> 8) as u8);

    let h = buf.get_real_buffer_height() - 1;
    result.push(h as u8);
    result.push((h >> 8) as u8);

    let len = buf.get_real_buffer_height() * buf.get_buffer_width();
    let mut x = 0;
    while x < len {
        let ch = buf
            .get_char(Position::from_index(buf, x))
            .unwrap_or_default();
        let mut rle_count = 1;
        while x + rle_count < len && rle_count < (u16::MAX) as i32 {
            if ch
                != buf
                    .get_char(Position::from_index(buf, x + rle_count))
                    .unwrap_or_default()
            {
                break;
            }
            rle_count += 1;
        }
        if rle_count > 3 || ch.ch == '\x01' {
            result.push(1);
            result.push(0);

            result.push(rle_count as u8);
            result.push((rle_count >> 8) as u8);
        } else {
            rle_count = 1;
        }
        result.push(ch.ch as u8);
        result.push(ch.attribute.as_u8(BufferType::LegacyIce));

        x += rle_count;
    }

    // font
    if buf.get_font_dimensions() != Size::new(8, 16) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Only 8x16 fonts are supported by adf.",
        ));
    }
    buf.get_font(0).unwrap().convert_to_u8_data(&mut result);

    // palette
    result.extend(buf.palette.to_16color_vec());
    if options.save_sauce {
        buf.write_sauce_info(&crate::SauceFileType::Bin, &mut result)?;
    }
    Ok(result)
}

pub fn get_save_sauce_default_idf(buf: &Buffer) -> (bool, String) {
    if buf.get_buffer_width() != 80 {
        return (true, "width != 80".to_string());
    }

    if buf.has_sauce_relevant_data() {
        return (true, String::new());
    }

    (false, String::new())
}

fn advance_pos(x1: i32, x2: i32, pos: &mut Position) -> bool {
    pos.x += 1;
    if pos.x > x2 {
        pos.x = x1;
        pos.y += 1;
    }
    true
}
