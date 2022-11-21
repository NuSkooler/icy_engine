use crate::{ViewdataParser, Position, BufferParser, Buffer, Caret};

use super::update_buffer;

fn create_viewdata_buffer<T: BufferParser>(parser: &mut T, input: &[u8]) -> (Buffer, Caret) 
{
    let mut buf = Buffer::create(40, 24);
    let mut caret  = Caret::new();
    // remove editing layer
    buf.is_terminal_buffer = true;
    buf.layers.remove(0);
    buf.layers[0].is_locked = false;
    buf.layers[0].is_transparent = false;
    
    update_buffer(&mut buf, &mut caret, parser, input);
    
    (buf, caret)
}


#[test]
fn test_bs() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"ab\x08");
    assert_eq!(Position::new(1, 0), caret.pos);

    let (buf, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x08");
    assert_eq!(Position::new(buf.get_buffer_width() - 1, 0), caret.pos);
}

#[test]
fn test_ht() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x09");
    assert_eq!(Position::new(1, 0), caret.pos);

    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x08\x09");
    assert_eq!(Position::new(0, 1), caret.pos);
}

#[test]
fn test_lf() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"test\x0A");
    assert_eq!(Position::new(4, 1), caret.pos);

    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x0B\x0A");
    assert_eq!(Position::new(0, 0), caret.pos);
}

#[test]
fn test_vt() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\n\n\x0B");
    assert_eq!(Position::new(0, 1), caret.pos);

    let (buf, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x0B");
    assert_eq!(Position::new(0, buf.get_buffer_height() - 1), caret.pos);
}

#[test]
fn test_ff() {
    let (buf, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"test\x0C");
    assert_eq!(Position::new(0, 0), caret.pos);
    assert_eq!(' ', buf.get_char(Position::new(0, 0)).unwrap().ch);
}

#[test]
fn test_set_fg_color() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BA");
    assert_eq!(1, caret.attr.get_foreground());
}

#[test]
fn test_set_bg_color() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BA\x1B]");
    assert_eq!(1, caret.attr.get_background());
}

#[test]
fn test_set_black_bg_color() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BA\x1B]\x1B\\");
    assert_eq!(0, caret.attr.get_background());
}

#[test]
fn test_set_flash() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BH");
    assert!(caret.attr.is_blinking());
}

#[test]
fn test_reset_flash() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BH\x1BI");
    assert!(!caret.attr.is_blinking());
}

#[test]
fn test_set_double_height() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BM");
    assert!(caret.attr.is_double_height());
}

#[test]
fn test_reset_double_height() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BM\x1BL");
    assert!(!caret.attr.is_double_height());
}

#[test]
fn test_conceal() {
    let (_, caret) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BX");
    assert!(caret.attr.is_concealed());
}

#[test]
fn test_line_lose_color_bug() {
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BAfoo\x1BBbar\x1E\x1E");
    assert_eq!(1, buf.get_char(Position::new(1, 0)).unwrap().attribute.get_foreground());
}

#[test]
fn testpage_bug_1() {
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BT\x1BZ\x1B^s\x1BQ\x1BY\x1BU\x1B@\x1BU\x1BA\x1BM");
    assert_eq!(' ', buf.get_char(Position::new(10, 0)).unwrap().ch);
}

#[test]
fn testpage_bug_2() {
    // bg color changes immediately
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BM \x1BE\x1B]\x1BBT");
    assert_eq!(5, buf.get_char(Position::new(3, 0)).unwrap().attribute.get_background());
}

#[test]
fn testpage_bug_3() {
    // bg reset color changes immediately
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BM \x1BE\x1B]\x1BBT\x1B\\");
    assert_eq!(0, buf.get_char(Position::new(6, 0)).unwrap().attribute.get_background());
}

#[test]
fn testpage_bug_4() {
    // conceal has no effect in graphics mode 
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1B^\x1BRs\x1BV\x1BX\x1BS\x1B@\x1BW\x1BX\x1BA05");
    for i in 0..10 {
        assert!(!buf.get_char(Position::new(i, 0)).unwrap().attribute.is_concealed());
    }
}

#[test]
fn test_cr_at_eol() {
    // conceal has no effect in graphics mode 
    let (buf, _) = create_viewdata_buffer(&mut ViewdataParser::new(), b"\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA\x1BA01\x08\r");
    for x in 1..buf.get_buffer_width() {
        assert_eq!(1, buf.get_char(Position::new(x, 0)).unwrap().attribute.get_foreground(), "wrong color at {}", x);
    }
}
