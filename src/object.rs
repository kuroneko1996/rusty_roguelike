use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};
use std::cmp;
use config::*;
use map::*;

#[derive(Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            println!("{} attacks {} for {} hit points.", self.name, target.name, damage);
            target.take_damage(damage);
        } else {
            println!("{} attacks {} but it has no effect!", self.name, target.name);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ai;

pub struct ObjectsManager {
    pub objects: Vec<Object>,
}

impl ObjectsManager {

    pub fn add(&mut self, obj: Object) {
        self.objects.push(obj);
    }

    pub fn get(&mut self, id: usize) -> &mut Object {
        &mut self.objects[id]
    }

    pub fn draw_clear(&self, con: &mut Offscreen) {
        for object in &self.objects {
            object.clear(con);
        }
    }

    pub fn draw(&self, con: &mut Offscreen) {
        for object in &self.objects {
            object.draw(con);
        }
    }

    pub fn move_by(&mut self, id: usize, dx: i32, dy: i32, map: &Map) {
        let (x, y) = self.objects[id].pos();
        let new_x: i32 = x + dx;
        let new_y: i32 = y + dy;
        if new_x < 0 || new_y < 0 || new_x >= MAP_WIDTH || new_y >= MAP_HEIGHT {
            return
        }

        if !is_blocked(x + dx, y + dy, map, &self.objects) {
            self.objects[id].x += dx;
            self.objects[id].y += dy;
        }
    }

    pub fn move_towards(&mut self, id: usize, target_x: i32, target_y: i32, map: &Map) {
        let (x, y) = self.objects[id].pos();
        // make a vector
        let dx = target_x - x;
        let dy = target_y - y;
        let distance = ((dx.pow(2) + dx.pow(2)) as f32).sqrt();

        // normalize to 1
        let dx = (dx as f32 / distance).round() as i32;
        let dy = (dy as f32 / distance).round() as i32;

        self.move_by(id, dx, dy, map);
    }

    pub fn player_move_or_attack(&mut self, dx: i32, dy: i32, map: &Map) {
        let x = self.get(PLAYER).x + dx;
        let y = self.get(PLAYER).y + dy;

        let target_id = self.objects.iter().position(|object| {
            object.pos() == (x, y)
        });

        match target_id {
            Some(target_id) => {
                let (player, target) = mut_two(PLAYER, target_id, &mut self.objects);
                player.attack(target);
            },
            None => {
                self.move_by(PLAYER, dx, dy, map);
            }
        }
    }

    pub fn ai_take_turn(&mut self, monster_id: usize, map: &Map, fov_map: &FovMap) {
        let (monster_x, monster_y) = self.get(monster_id).pos();

        if fov_map.is_in_fov(monster_x, monster_y) {
            if self.objects[monster_id].distance_to(&self.objects[PLAYER]) >= 2.0 {
                // move towards player if far away
                let (player_x, player_y) = self.get(PLAYER).pos();
                self.move_towards(monster_id, player_x, player_y, map);
            } else {
                let (monster, player) = mut_two(monster_id, PLAYER, &mut self.objects);
                monster.attack(player);
            }
        }
    }

    pub fn ai_turn(&mut self, map: &Map, fov_map: &FovMap) {
        for id in 0..self.objects.len() {
            if self.objects[id].ai.is_some() {
                self.ai_take_turn(id, &map, &fov_map);
            }
        }
    }
}

/// Mutably borrow two *separate* elements from the given slice.
/// Panics when the indexes are equal or out of bounds.
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}
