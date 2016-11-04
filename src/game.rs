use std::cell::RefCell;
use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::Mouse;

use map::Map;
use messages::Messages;
use object::Object;

pub struct Game {
    pub map: Map,
    pub log: Messages,
    pub inventory: Vec<RefCell<Object>>,
}

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}
