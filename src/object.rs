use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::ops::Deref;

use config::*;
use map::*;
use messages::*;
use game::*;

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
    pub item: Option<Item>,
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
            item: None,
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

    pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, messages);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, messages: &mut Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            message(messages, format!("{} attacks {} for {} hit points.", self.name, target.name, damage),
                    colors::WHITE);
            target.take_damage(damage, messages);
        } else {
            message(messages, format!("{} attacks {} but it has no effect!", self.name, target.name),
                    colors::WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeathCallback {
    Player,
    Monster,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
    Lightning,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, messages: &mut Messages) {
        let callback: fn(&mut Object, &mut Messages) = match self {
            DeathCallback::Player => player_death,
            DeathCallback::Monster => monster_death,
        };
        callback(object, messages);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ai;

pub struct ObjectsManager {
    pub objects: Vec<RefCell<Object>>,
}

impl ObjectsManager {

    pub fn draw_clear(&self, con: &mut Offscreen) {
        for object in &self.objects {
            object.borrow().clear(con);
        }
    }

    pub fn draw(&self, con: &mut Offscreen, fov_map: &FovMap) {
        let mut to_draw: Vec<_> = self.objects.iter().map(|c| c.borrow()).filter(|o| fov_map.is_in_fov(o.x, o.y)).collect();
        // sort so that non-blocking objects come first
        to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
        for object in &to_draw {
            object.draw(con);
        }
    }

    pub fn move_by(&mut self, id: usize, dx: i32, dy: i32, map: &Map) {
        let (x, y) = self.objects[id].borrow().pos();
        let new_x: i32 = x + dx;
        let new_y: i32 = y + dy;
        if new_x < 0 || new_y < 0 || new_x >= MAP_WIDTH || new_y >= MAP_HEIGHT {
            return
        }

        if !is_blocked(x + dx, y + dy, map, &self.objects) {
            let mut object = self.objects[id].borrow_mut();
            object.x += dx;
            object.y += dy;
        }
    }

    pub fn move_towards(&mut self, id: usize, target_x: i32, target_y: i32, map: &Map) {
        let (x, y) = self.objects[id].borrow().pos();
        // make a vector
        let dx = target_x - x;
        let dy = target_y - y;
        let distance = ((dx.pow(2) + dx.pow(2)) as f32).sqrt();

        // normalize to 1
        let dx = (dx as f32 / distance).round() as i32;
        let dy = (dy as f32 / distance).round() as i32;

        self.move_by(id, dx, dy, map);
    }

    pub fn player_move_or_attack(&mut self, dx: i32, dy: i32, game: &mut Game) {
        let (mut x, mut y) = self.objects[PLAYER].borrow().pos();
        x += dx;
        y += dy;

        let target_id = self.objects.iter_mut().map(|c| c.borrow()).position(|object| {
            object.fighter.is_some() && object.pos() == (x, y)
        });

        match target_id {
            Some(target_id) => {
                let (mut player, mut target) = (self.objects[PLAYER].borrow_mut(), self.objects[target_id].borrow_mut());
                player.attack(target.deref_mut(), &mut game.log);
            },
            None => {
                self.move_by(PLAYER, dx, dy, &game.map);
            }
        }
    }

    pub fn ai_take_turn(&mut self, monster_id: usize, map: &Map, fov_map: &FovMap, messages: &mut Messages) {
        let (monster_x, monster_y) = self.objects[monster_id].borrow().pos();

        if fov_map.is_in_fov(monster_x, monster_y) {
            let distance = self.objects[monster_id].borrow().distance_to(self.objects[PLAYER].borrow().deref());
            if distance >= 2.0 {
                // move towards player if far away
                let (player_x, player_y) = self.objects[PLAYER].borrow().pos();
                self.move_towards(monster_id, player_x, player_y, map);
            } else {
                let (mut player, mut monster) = (self.objects[PLAYER].borrow_mut(), self.objects[monster_id].borrow_mut());
                monster.attack(player.deref_mut(), messages);
            }
        }
    }

    pub fn ai_turn(&mut self, map: &Map, fov_map: &FovMap, messages: &mut Messages) {
        for id in 0..self.objects.len() {
            if self.objects[id].borrow().ai.is_some() {
                self.ai_take_turn(id, &map, &fov_map, messages);
            }
        }
    }
}

fn player_death(player: &mut Object, messages: &mut Messages) {
    message(messages, "YOU DIED", colors::RED);

    player.char = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, messages: &mut Messages) {
    // transform to a corpse
    message(messages, format!("{} is dead", monster.name), colors::ORANGE);
    monster.char = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn pick_item_up(object_id: usize, object_manager: &mut ObjectsManager, game: &mut Game) {
    if game.inventory.len() >= MAX_INVENTORY_SIZE as usize {
        message(&mut game.log, format!("Your inventory is full, cannot pick up {}.", object_manager.objects[object_id].borrow().deref().name), colors::RED);
    } else {
        let item = object_manager.objects.swap_remove(object_id);
        message(&mut game.log, format!("You picked up a {}!", item.borrow().deref().name), colors::GREEN);
        game.inventory.push(item);
    }
}

pub enum UseResult {
    UsedUp,
    Cancelled,
}

pub fn use_item(inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) 
{
    let item = game.inventory[inventory_id].borrow().item;
    
    let on_use = match item {
        Some(Item::Heal) => cast_heal,
        Some(Item::Lightning) => cast_lightning,
        None => {
            message(&mut game.log, format!("The {} cannot be used.", game.inventory[inventory_id].borrow().name), colors::WHITE);
            return
        },
    };

    match on_use(inventory_id, object_manager, game, tcod) {
        UseResult::UsedUp => {
            // destroy after use
            game.inventory.remove(inventory_id);
        },
        UseResult::Cancelled => {
            message(&mut game.log, "Cancelled", colors::WHITE);
        }
    }
}

pub fn drop_item(inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game) 
{
    let item_cell = game.inventory.remove(inventory_id);
    {
        let mut item = item_cell.borrow_mut();
        let player = object_manager.objects[PLAYER].borrow();
        item.set_pos(player.x, player.y);
        message(&mut game.log, format!("You dropped a {}.", item.name), colors::YELLOW);
    }
    object_manager.objects.push(item_cell);
}

fn cast_heal(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    let mut is_fighter = false;
    if let Some(fighter) = object_manager.objects[PLAYER].borrow().fighter {
        if fighter.hp == fighter.max_hp {
            message(&mut game.log, "You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        is_fighter = true;
    }

    if is_fighter {
        message(&mut game.log, "Your wounds start to feel better!", colors::LIGHT_VIOLET);
        object_manager.objects[PLAYER].borrow_mut().heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }

    UseResult::Cancelled
}

fn cast_lightning(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    // find the closest enemy
    let monster_id = closest_monster(LIGHTNING_RANGE, object_manager, tcod);
    if let Some(monster_id) = monster_id {
        let mut monster = object_manager.objects[monster_id].borrow_mut();
        message(&mut game.log, format!("A lightning bolt strikes the {} with a loud thunder! \
                                        The damage is {} hit points.", 
                                monster.name, LIGHTNING_DAMAGE),
                colors::LIGHT_BLUE);
        monster.take_damage(LIGHTNING_DAMAGE, &mut game.log);
        UseResult::UsedUp
    } else {
        message(&mut game.log, "No enemy is close enough to strike", colors::RED);
        UseResult::Cancelled
    }
}

fn closest_monster(max_range: i32, object_manager: &mut ObjectsManager, tcod: &mut Tcod) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // starts with slighty more than max range

    for (id, cell) in object_manager.objects.iter().enumerate() {
        let object = cell.borrow();

        if (id != PLAYER) && object.fighter.is_some() && object.ai.is_some() 
            && tcod.fov.is_in_fov(object.x, object.y) 
        {
            let dist = object_manager.objects[PLAYER].borrow().distance_to(object.deref());
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}