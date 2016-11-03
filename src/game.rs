use std::cell::RefCell;

use map::Map;
use messages::Messages;
use object::Object;

pub struct Game {
    pub map: Map,
    pub log: Messages,
    pub inventory: Vec<RefCell<Object>>,
}
