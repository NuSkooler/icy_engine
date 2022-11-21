mod buffer_operation_tests;
mod ansi_parser_tests;
mod ascii_parser_tests;
mod viewdata_parser_tests;

use crate::{Buffer, Caret, BufferParser};

fn get_string_from_buffer(buf: &Buffer) -> String
{
    let converted = crate::convert_to_asc(&buf, &crate::SaveOptions::new()).unwrap(); // test code
    let b : Vec<u8> = converted.iter().map(|&x| if x == 27 { b'x' } else { x }).collect();
    let converted  = String::from_utf8_lossy(b.as_slice());

    converted.to_string()
}

fn create_buffer<T: BufferParser>(parser: &mut T, input: &[u8]) -> (Buffer, Caret) 
{
    let mut buf = Buffer::create(80, 25);
    let mut caret  = Caret::new();
    // remove editing layer
    buf.is_terminal_buffer = true;
    buf.layers.remove(0);
    buf.layers[0].is_locked = false;
    buf.layers[0].is_transparent = false;
    
    update_buffer(&mut buf, &mut caret, parser, input);
    
    (buf, caret)
}

fn update_buffer<T: BufferParser>(buf: &mut Buffer, caret: &mut Caret, parser: &mut T, input: &[u8])
{
    for b in input {
        if let Some(ch) = char::from_u32(*b as u32) {
            parser.print_char(buf,caret, ch).unwrap(); // test code
        }
    }
}