extern crate tcod;
extern crate rand;

use std::cell::RefCell;
use tcod::console::*;
use tcod::colors::{self};
use tcod::map::{Map as FovMap};
use tcod::input::{self, Event, Key};

mod config;
mod tile;
mod map;
mod object;
mod rect;
mod messages;
mod game;

use config::*;
use map::*;
use object::*;
use messages::*;
use game::*;

fn handle_keys(key: Key, tcod: &mut Tcod, game: &mut Game, object_manager: &mut ObjectsManager) -> PlayerAction 
{
    use tcod::input::KeyCode::*;
    use game::PlayerAction::*;

    let is_alive = object_manager.objects[PLAYER].borrow().alive;

    match (key, is_alive) {
        // Alt Enter fullscreen
        (Key { code: Enter, alt: true, ..}, _) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, ..}, _) => Exit, // Exit game
        // Movement
        (Key { code: Up, .. }, true) | (Key { code: NumPad8, .. }, true) => {
            object_manager.player_move_or_attack(0, -1, game);
            TookTurn 
        },
        (Key { code: Down, .. }, true) | (Key { code: NumPad2, .. }, true) => {
            object_manager.player_move_or_attack(0, 1, game);
            TookTurn 
        },
        (Key { code: Left, .. }, true) | (Key { code: NumPad4, .. }, true) => {
            object_manager.player_move_or_attack(-1, 0, game);
            TookTurn 
        },
        (Key { code: Right, .. }, true) | (Key { code: NumPad6, .. }, true) => {
            object_manager.player_move_or_attack(1, 0, game);
            TookTurn 
        },
        (Key { code: Home, .. }, true) | (Key { code: NumPad7, .. }, true) => {
            object_manager.player_move_or_attack(-1, -1, game);
            TookTurn 
        },
        (Key { code: PageUp, .. }, true) | (Key { code: NumPad9, .. }, true) => {
            object_manager.player_move_or_attack(1, -1, game);
            TookTurn 
        },
        (Key { code: End, .. }, true) | (Key { code: NumPad1, .. }, true) => {
            object_manager.player_move_or_attack(-1, 1, game);
            TookTurn 
        },
        (Key { code: PageDown, .. }, true) | (Key { code: NumPad3, .. }, true) => {
            object_manager.player_move_or_attack(1, 1, game);
            TookTurn 
        },
        (Key { code: NumPad5, .. }, true) => { // wait for turn
            TookTurn 
        },
        // Help screen
        (Key { printable: '?', .. }, true) | (Key { printable: '/', .. }, true) => { 
            show_help(&mut tcod.root);
            DidntTakeTurn
        }
        // Inventory
        (Key { printable: 'g', .. }, true) => {
            let player_pos = object_manager.objects[PLAYER].borrow().pos();
            // pick up an item
            let item_id = object_manager.objects.iter().map(|c| c.borrow()).position(|object| {
                object.pos() == player_pos && object.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, object_manager, game);
            }
            DidntTakeTurn
        },
        (Key {printable: 'd', .. }, true) => {
            let inventory_index = inventory_menu(&game.inventory, "Press the key next to an item to DROP it, or any other to cancel.\n",
                &mut tcod.root);

            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, object_manager, game);
            }
            DidntTakeTurn
        },
        (Key {printable: 'i', .. }, true) => {
            let inventory_index = inventory_menu(&game.inventory, "Press the key next to an item to USE it, or any other to cancel.\n",
                &mut tcod.root);

            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, object_manager, game, tcod);
            }
            DidntTakeTurn
        },
        _ => DidntTakeTurn,
    }
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rusty Roguelike")
        .init();
    tcod::system::set_fps(LIMIT_FPS);

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        mouse: Default::default(),
    };

    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![RefCell::new(player)];

    let mut game = Game {
        map: make_map(&mut objects),
        log: vec![], // messages here
        inventory: vec![],
    };

    let mut object_manager = ObjectsManager { objects: objects };

    // greeting
    game.log.add("Welcome stranger! Prepare to die in these catacombs. Hahaha.", colors::RED);

    // FOV
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            tcod.fov.set(x, y, !game.map[x as usize][y as usize].block_sight, !game.map[x as usize][y as usize].blocked)
        }
    }
    let mut previous_player_position = (-1, -1);

    let mut key = Default::default();

    while !tcod.root.window_closed() {
        let (player_x, player_y) = object_manager.objects[PLAYER].borrow().pos();
        let fov_recompute = previous_player_position != (player_x, player_y);

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }


        render_all(&mut tcod, &mut object_manager, &mut game, fov_recompute);

        tcod.root.flush();
      
        object_manager.draw_clear(&mut tcod.con);

        previous_player_position = (player_x, player_y);

        // player's turn
        let player_action = handle_keys(key, &mut tcod, &mut game, &mut object_manager);
        if player_action == PlayerAction::Exit {
            break
        }

        // monsters turn
        if object_manager.objects[PLAYER].borrow().alive && player_action == PlayerAction::TookTurn {
            object_manager.ai_turn(&mut game, &tcod.fov);
        }
    }
}
