extern crate rand;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::ops::Deref;
use rand::Rng;

use config::*;
use map::*;
use messages::*;
use game::*;

#[derive(Debug, RustcEncodable, RustcDecodable)]
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
    pub always_visible: bool,
    pub level: i32,
    pub equipment: Option<Equipment>,
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
            always_visible: false,
            level: 1,
            equipment: None,
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

    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            game.log.add(format!("{} attacks {} for {} hit points.", self.name, target.name, damage),
                    colors::WHITE);
            if let Some(xp) = target.take_damage(damage, game) {
                // yield experience to the player
                if let Some(f) = self.fighter.as_mut() {
                    f.xp += xp;
                }
            }
        } else {
            game.log.add(format!("{} attacks {} but it has no effect!", self.name, target.name),
                    colors::WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
            }
        }
    }

    pub fn equip(&mut self, log: &mut Vec<(String, Color)>) {
        if self.item.is_none() {
            log.add(format!("Can't equip {:?} because it's not an Item.", self),
                colors::RED);
            return
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                log.add(format!("Equipped {} on {}.", self.name, equipment.slot),
                                colors::LIGHT_GREEN);
            }
        } else {
            log.add(format!("Can't equip {:?} because it's not an Equipment.", self),
                colors::RED);
        }
    }

    pub fn dequip(&mut self, log: &mut Vec<(String, Color)>) {
        if self.item.is_none() {
            log.add(format!("Can't dequip {:?} because it's not an Item.", self),
                            colors::RED);
            return
        };
        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                equipment.equipped = false;
                log.add(format!("Dequipped {} from {}.", self.name, equipment.slot),
                                colors::LIGHT_YELLOW);
            }
        } else {
            log.add(format!("Can't dequip {:?} because it's not an Equipment.", self),
                            colors::RED);
        }
    }

    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.power_bonus);
        base_power + bonus
    }

    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.defense_bonus);
        base_defense + bonus
    }

    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus = self.get_all_equipped(game).iter().fold(0, |sum, e| sum + e.max_hp_bonus);
        base_max_hp + bonus
    }

    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "player" { // TODO
            game.inventory.iter().filter(|item| item.equipment.map_or(false, |e| e.equipped))
                .map(|item| item.equipment.unwrap()).collect()
        } else {
            vec![]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum DeathCallback {
    Player,
    Monster,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Fighter {
    pub base_max_hp: i32,
    pub hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub xp: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Sword,
    Shield,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcDecodable, RustcEncodable)]
/// An object that can be equipped, yielding bonuses.
pub struct Equipment {
    pub slot: Slot,
    pub equipped: bool,
    pub max_hp_bonus: i32,
    pub power_bonus: i32,
    pub defense_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcDecodable, RustcEncodable)]
pub enum Slot {
    LeftHand,
    RightHand,
    Head,
}

impl ::std::fmt::Display for Slot {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MonsterType {
    Orc,
    Troll,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, game: &mut Game) {
        let callback: fn(&mut Object, &mut Game) = match self {
            DeathCallback::Player => player_death,
            DeathCallback::Monster => monster_death,
        };
        callback(object, game);
    }
}

#[derive(Clone, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum Ai {
    Basic,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}

pub struct ObjectsManager {
    pub objects: Vec<RefCell<Object>>,
}

impl ObjectsManager {

    pub fn draw_clear(&self, con: &mut Offscreen) {
        for object in &self.objects {
            object.borrow().clear(con);
        }
    }

    pub fn draw(&self, tcod: &mut Tcod, game: &mut Game) {
        let mut to_draw: Vec<_> = self.objects.iter().map(|c| c.borrow()).filter(|o| {
            tcod.fov.is_in_fov(o.x, o.y) || (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
        }).collect();
        // sort so that non-blocking objects come first
        to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
        for object in &to_draw {
            object.draw(&mut tcod.con);
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
                player.attack(target.deref_mut(), game);
            },
            None => {
                self.move_by(PLAYER, dx, dy, &game.map);
            }
        }
    }

    pub fn ai_take_turn(&mut self, monster_id: usize, game: &mut Game, fov_map: &FovMap) {
        let ai_option = self.objects[monster_id].borrow_mut().ai.take();
        if let Some(ai) = ai_option {
            let new_ai = match ai {
                Ai::Basic => self.ai_basic(monster_id, game, fov_map),
                Ai::Confused{previous_ai, num_turns} => self.ai_confused(monster_id, game, previous_ai, num_turns),
            };
            self.objects[monster_id].borrow_mut().ai = Some(new_ai);
        }
    }

    pub fn ai_turn(&mut self, game: &mut Game, fov_map: &FovMap) {
        for id in 0..self.objects.len() {
            if self.objects[id].borrow().ai.is_some() {
                self.ai_take_turn(id, game, &fov_map);
            }
        }
    }

    fn ai_basic(&mut self, monster_id: usize, game: &mut Game, fov_map: &FovMap) -> Ai {
        let (monster_x, monster_y) = self.objects[monster_id].borrow().pos();

        if fov_map.is_in_fov(monster_x, monster_y) {
            let distance = self.objects[monster_id].borrow().distance_to(self.objects[PLAYER].borrow().deref());
            if distance >= 2.0 {
                // move towards player if far away
                let (player_x, player_y) = self.objects[PLAYER].borrow().pos();
                self.move_towards(monster_id, player_x, player_y, &game.map);
            } else {
                let (mut player, mut monster) = (self.objects[PLAYER].borrow_mut(), self.objects[monster_id].borrow_mut());
                monster.attack(player.deref_mut(), game);
            }
        }
        Ai::Basic
    }

    fn ai_confused(&mut self, monster_id: usize, game: &mut Game, previous_ai: Box<Ai>, num_turns: i32) -> Ai 
    {
        if num_turns >= 0 {
            self.move_by(monster_id, rand::thread_rng().gen_range(-1, 2), rand::thread_rng().gen_range(-1, 2), &game.map);
            Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
        } else {
            game.log.add(format!("The {} is no longer confused!", self.objects[monster_id].borrow().name), colors::RED);
            *previous_ai
        }
    }
}

fn player_death(player: &mut Object, game: &mut Game) {
    game.log.add("YOU DIED", colors::RED);

    player.char = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    // transform to a corpse
    game.log.add(
    format!("{} is dead! You gain {} experience points.",
            monster.name, monster.fighter.map_or(0, |f| f.xp)), colors::ORANGE);

    monster.char = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn pick_item_up(object_id: usize, object_manager: &mut ObjectsManager, game: &mut Game) {
    if game.inventory.len() >= MAX_INVENTORY_SIZE as usize {
        game.log.add(format!("Your inventory is full, cannot pick up {}.", object_manager.objects[object_id].borrow().deref().name), colors::RED);
    } else {
        let cell = object_manager.objects.swap_remove(object_id);
        let item = cell.into_inner();
        game.log.add(format!("You picked up a {}!", item.name), colors::GREEN);
        game.inventory.push(item);
    }
}

pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

pub fn use_item(inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) 
{
    let item = game.inventory[inventory_id].item;
    
    let on_use = match item {
        Some(Item::Heal) => cast_heal,
        Some(Item::Lightning) => cast_lightning,
        Some(Item::Confuse) => cast_confuse,
        Some(Item::Fireball) => cast_fireball,
        Some(Item::Sword) => toggle_equipment,
        Some(Item::Shield) => toggle_equipment,
        None => {
            game.log.add(format!("The {} cannot be used.", game.inventory[inventory_id].name), colors::WHITE);
            return
        },
    };

    match on_use(inventory_id, object_manager, game, tcod) {
        UseResult::UsedUp => {
            // destroy after use
            game.inventory.remove(inventory_id);
        },
        UseResult::UsedAndKept => {}, // do nothing
        UseResult::Cancelled => {
            game.log.add("Cancelled", colors::WHITE);
        }
    }
}

pub fn drop_item(inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game) 
{
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.dequip(&mut game.log);
    }
    
    {
        let player = object_manager.objects[PLAYER].borrow();
        item.set_pos(player.x, player.y);
    }
    game.log.add(format!("You dropped a {}.", item.name), colors::YELLOW);
    object_manager.objects.push(RefCell::new(item));
}

fn cast_heal(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    let mut player = object_manager.objects[PLAYER].borrow_mut();
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.log.add("You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }

        game.log.add("Your wounds start to feel better!", colors::LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    // find the closest enemy
    let monster_id = closest_monster(LIGHTNING_RANGE, object_manager, tcod);
    if let Some(monster_id) = monster_id {
        let mut monster = object_manager.objects[monster_id].borrow_mut();
        game.log.add(format!("A lightning bolt strikes the {} with a loud thunder! \
                                        The damage is {} hit points.", 
                                monster.name, LIGHTNING_DAMAGE),
                colors::LIGHT_BLUE);
        if let Some(xp) = monster.take_damage(LIGHTNING_DAMAGE, game) {
            // add exp to the player
            if let Some(f) = object_manager.objects[PLAYER].borrow_mut().fighter.as_mut() {
                f.xp += xp;
            }
        }
        UseResult::UsedUp
    } else {
        game.log.add("No enemy is close enough to strike", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    game.log.add("Left-click an enemy to confuse it, or right-click to cancel.",
        colors::LIGHT_CYAN);
    let monster_id = target_monster(tcod, object_manager, game, Some(CONFUSE_RANGE as f32));

    if let Some(monster_id) = monster_id {
        let mut monster = object_manager.objects[monster_id].borrow_mut();
        // replace old ai
        let old_ai = monster.ai.take().unwrap_or(Ai::Basic);
        monster.ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.log.add(format!("The eyes of {} look vacant, as he starts to stumble around!",
                                            monster.name),
                colors::LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.log.add("No enemy is close enough to strike", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(_inventory_id: usize, object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) -> UseResult {
    game.log.add("Left-click a target tile for the fireball, or right-click to cancel.",
            colors::LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, object_manager, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.log.add(format!("The fireball explodes, burning everything within {} tiles!", FIREBALL_RADIUS),
            colors::ORANGE);

    let mut xp_to_gain = 0;
    for (id, cell) in object_manager.objects.iter_mut().enumerate() {
        let mut obj = cell.borrow_mut();
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE),
                    colors::ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }
    object_manager.objects[PLAYER].borrow_mut().fighter.as_mut().unwrap().xp += xp_to_gain;

    UseResult::UsedUp
}

fn toggle_equipment(inventory_id: usize, _object_manager: &mut ObjectsManager, game: &mut Game, _tcod: &mut Tcod) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if equipment.equipped {
        game.inventory[inventory_id].dequip(&mut game.log);
    } else {
        if let Some(old_equipment) = get_equipped_in_slot(equipment.slot, &game.inventory) {
            game.inventory[old_equipment].dequip(&mut game.log);
        }
        game.inventory[inventory_id].equip(&mut game.log);
    }
    UseResult::UsedAndKept
}

fn get_equipped_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
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
