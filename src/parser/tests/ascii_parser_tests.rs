use crate::{SaveOptions, AsciiParser, parser::tests::{create_buffer, update_buffer}, convert_to_asc, Position};

fn test_ascii(data: &[u8])
{
    let (buf, _) = create_buffer(&mut AsciiParser::new(), data);
    let converted = convert_to_asc(&buf, &SaveOptions::new()).unwrap();

    // more gentle output.
    let b : Vec<u8> = converted.iter().map(|&x| if x == 27 { b'x' } else { x }).collect();
    let converted  = String::from_utf8_lossy(b.as_slice());

    let b : Vec<u8> = data.iter().map(|&x| if x == 27 { b'x' } else { x }).collect();
    let expected  = String::from_utf8_lossy(b.as_slice());

    assert_eq!(expected, converted);
}

#[test]
fn test_full_line_height() {
    let mut vec = Vec::new();
    vec.resize(80, b'-');
    let (mut buf, mut caret) = create_buffer(&mut AsciiParser::new(), &vec);
    assert_eq!(1, buf.get_real_buffer_height());
    vec.push(b'-');
    update_buffer(&mut buf, &mut caret, &mut AsciiParser::new(), &vec);
    assert_eq!(3, buf.get_real_buffer_height());
}

#[test]
fn test_emptylastline_height() {
    let mut vec = Vec::new();
    vec.resize(80, b'-');
    vec.resize(80 * 2, b' ');
    let (buf, _) = create_buffer(&mut AsciiParser::new(), &vec);
    assert_eq!(2, buf.get_real_buffer_height());
}

/*
#[test]
fn test_emptylastline_roundtrip() {
    let mut vec = Vec::new();
    vec.resize(80, b'-');
    vec.resize(80 * 2, b' ');

    let (buf, _) = create_buffer(&mut AsciiParser::new(), &vec);
    assert_eq!(2, buf.get_real_buffer_height());
    let vec2 = buf.to_bytes("asc", &SaveOptions::new()).unwrap();
    let (buf2, _) = create_buffer(&mut AsciiParser::new(), &vec2);
    assert_eq!(2, buf2.get_real_buffer_height());
}

 */
#[test]
fn test_eol() {
    let data = b"foo\r\n";
    let (buf, _) = create_buffer(&mut AsciiParser::new(), data);
    assert_eq!(2, buf.get_real_buffer_height());
}
/* 
#[test]
fn test_ws_skip() {
    let data = b"123456789012345678901234567890123456789012345678901234567890123456789012345678902ndline";
    test_ascii(data);
}

#[test]
fn test_ws_skip_empty_line() {
    let data = b"12345678901234567890123456789012345678901234567890123456789012345678901234567890\r\n\r\n2ndline";
    test_ascii(data);
}
*/
#[test]
fn test_eol_start() {
    let data = b"\r\n2ndline";
    test_ascii(data);
}

#[test]
fn test_eol_line_break() {
    let (mut buf, mut caret) = create_buffer(&mut AsciiParser::new(), b"################################################################################\r\n");
    assert_eq!(Position::from(0, 1), caret.pos);

    update_buffer(&mut buf, &mut caret, &mut AsciiParser::new(), b"#");
    assert_eq!(Position::from(1, 1), caret.pos);
    assert_eq!(b'#', buf.get_char(Position::from(0, 1)).unwrap().char_code as u8);
}

