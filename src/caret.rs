use super::{Position, TextAttribute};

#[derive(Clone)]
pub struct Caret {
    pub(super) pos: Position,
    pub(super) attr: TextAttribute,
    pub insert_mode: bool,
    pub is_visible: bool,
    pub is_blinking: bool,
}

impl Caret {
    pub fn new(pos: Position) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }

    pub fn new_xy(x: i32, y: i32) -> Self {
        Self {
            pos: Position { x, y },
            ..Default::default()
        }
    }

    pub fn get_attribute(&self) -> TextAttribute {
        self.attr
    }

    pub fn get_position(&self) -> Position {
        self.pos
    }

    pub fn set_position(&mut self, pos: Position) {
        self.pos = pos;
    }

    pub fn set_position_xy(&mut self, x: i32, y: i32) {
        self.pos = Position::new(x, y);
    }

    pub fn set_x_position(&mut self, x: i32) {
        self.pos.x = x;
    }

    pub fn set_y_position(&mut self, y: i32) {
        self.pos.y = y;
    }

    pub fn set_attr(&mut self, attr: TextAttribute) {
        self.attr = attr;
    }

    pub fn set_foreground(&mut self, color: u32) {
        self.attr.set_foreground(color);
    }

    pub fn set_background(&mut self, color: u32) {
        self.attr.set_background(color);
    }

    pub(crate) fn reset(&mut self) {
        self.pos = Position::default();
        self.attr = TextAttribute::default();
        self.insert_mode = false;
        self.is_visible = true;
        self.is_blinking = true;
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Caret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor")
            .field("pos", &self.pos)
            .field("attr", &self.attr)
            .field("insert_mode", &self.insert_mode)
            .finish()
    }
}

impl Default for Caret {
    fn default() -> Self {
        Self {
            pos: Position::default(),
            attr: TextAttribute::default(),
            insert_mode: false,
            is_visible: true,
            is_blinking: true,
        }
    }
}

impl PartialEq for Caret {
    fn eq(&self, other: &Caret) -> bool {
        self.pos == other.pos && self.attr == other.attr
    }
}
