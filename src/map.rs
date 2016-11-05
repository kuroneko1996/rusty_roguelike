extern crate rand;

use std::cmp;
use std::cell::RefCell;

use rand::Rng;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};
use tcod::colors::{self};

use config::*;
use tile::*;
use rect::*;
use object::*;

pub type Map = Vec<Vec<Tile>>;

pub fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

pub fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

pub fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }    
}

pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<RefCell<Object>>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            // monsters random table
            let monster_chances = &mut [
                Weighted {weight: 80, item: MonsterType::Orc},
                Weighted {weight: 20, item: MonsterType::Troll},
            ];
            let monster_choice = WeightedChoice::new(monster_chances);

            let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
                MonsterType::Orc => { 
                    let mut orc = Object::new(x, y, 'o', "orc", colors::DESATURATED_GREEN, true);
                    orc.fighter = Some(Fighter{
                        max_hp: 10, hp: 10, defense: 0, power: 3, xp: 35,
                        on_death: DeathCallback::Monster,
                    });
                    orc.ai = Some(Ai::Basic);
                    orc
                },
                MonsterType::Troll => {
                    let mut troll = Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true);
                    troll.fighter = Some(Fighter{
                        max_hp: 10, hp: 10, defense: 0, power: 3, xp: 100,
                        on_death: DeathCallback::Monster,
                    });
                    troll.ai = Some(Ai::Basic);
                    troll
                },
            };
            monster.alive = true;
            objects.push(RefCell::new(monster));
        }
    }

    // items
    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);
    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            // item random table
            let item_chances = &mut [
                Weighted {weight: 70, item: Item::Heal},
                Weighted {weight: 10, item: Item::Lightning},
                Weighted {weight: 10, item: Item::Fireball},
                Weighted {weight: 10, item: Item::Confuse},
                Weighted {weight: 100, item: Item::Equipment},
            ];
            let item_choice = WeightedChoice::new(item_chances);

            let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                Item::Heal => {
                    let mut object = Object::new(x, y, '!', "healing potion", colors::VIOLET, false);
                    object.item = Some(Item::Heal);
                    object
                },
                Item::Lightning => {
                    let mut object = Object::new(x, y, '#', "scroll of lightning bolt", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Lightning);
                    object
                },
                Item::Fireball => {
                    let mut object = Object::new(x, y, '#', "scroll of fireball", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Fireball);
                    object
                },
                Item::Confuse => {
                    let mut object = Object::new(x, y, '#', "scroll of confusion", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Confuse);
                    object
                },
                Item::Equipment => {
                    let mut object = Object::new(x, y, '/', "sword", colors::SKY, false);
                    object.item = Some(Item::Equipment);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand});
                    object
                },
            };
            item.always_visible = true;
            objects.push(RefCell::new(item));
        }
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[RefCell<Object>]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }

    objects.iter().map(|c| c.borrow()).any(|object| {
        object.blocks && object.pos() == (x, y)
    })
}

pub fn make_map(objects: &mut Vec<RefCell<Object>>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);
        // check for overlapping with existing ones
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            create_room(new_room, &mut map);
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() { // first room
                objects[PLAYER].borrow_mut().set_pos(new_x, new_y);
            } else {
                // connect the room to the previous room with a tunnel
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // random order
                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            place_objects(new_room, &map, objects);
            rooms.push(new_room);
        }
    }

    // add stairs to the center of last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', "stairs", colors::WHITE, false);
    stairs.always_visible = true;
    objects.push(RefCell::new(stairs));

    map
}
