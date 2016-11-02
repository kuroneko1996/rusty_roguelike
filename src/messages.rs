use config::*;
use tcod::colors::{Color};

pub type Messages = Vec<(String, Color)>;

pub fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
    if messages.len() == MSG_HEIGHT {
        messages.remove(0);
    }
    messages.push((message.into(), color));
}
