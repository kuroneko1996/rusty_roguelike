extern crate tcod;
extern crate rand;
extern crate rustc_serialize;

use std::cell::RefCell;
use tcod::console::*;
use tcod::colors::{self};
use tcod::map::{Map as FovMap};
use tcod::input::{self, Event, Key};

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;
use rustc_serialize::json;

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

fn initialise_fov(map: &Map, tcod: &mut Tcod) {
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            tcod.fov.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked)
        }
    }
    tcod.con.clear();  // unexplored areas start black (which is the default background color)
}

fn new_game(tcod: &mut Tcod) -> (ObjectsManager, Game) {
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

    let object_manager = ObjectsManager { objects: objects };

    initialise_fov(&game.map, tcod);

    // greeting
    game.log.add("Welcome stranger! Prepare to die in these catacombs. Hahaha.", colors::RED);

    (object_manager, game)
}

fn play_game(object_manager: &mut ObjectsManager, game: &mut Game, tcod: &mut Tcod) {
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

        render_all(tcod, object_manager, game, fov_recompute);

        tcod.root.flush();
      
        object_manager.draw_clear(&mut tcod.con);

        previous_player_position = (player_x, player_y);

        // player's turn
        let player_action = handle_keys(key, tcod, game, object_manager);
        if player_action == PlayerAction::Exit {
            msg("\nSaving game...\n", 24, &mut tcod.root);
            match save_game(object_manager, game) {
                Ok(_) => {
                    break
                },
                Err(_e) => {
                    msgbox("\nError saving file.\n", 24, &mut tcod.root);
                },
            }
            break
        }

        // monsters turn
        if object_manager.objects[PLAYER].borrow().alive && player_action == PlayerAction::TookTurn {
            object_manager.ai_turn(game, &tcod.fov);
        }
    }
}

fn main_menu(tcod: &mut Tcod) {
    while !tcod.root.window_closed() {
        let choices = &["Play a new game", "Continue last game", "Quit"];

        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(SCREEN_WIDTH/2, SCREEN_HEIGHT/2 - 4,
                   BackgroundFlag::None, TextAlignment::Center,
                   "RUSTY ROGUELIKE");
        tcod.root.print_ex(SCREEN_WIDTH/2, SCREEN_HEIGHT - 2,
                   BackgroundFlag::None, TextAlignment::Center,
                   "By name");

        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => { // new game
                let (mut object_manager, mut game) = new_game(tcod);
                play_game(&mut object_manager, &mut game, tcod);
                tcod.root.clear();
            },
            Some(1) => { // load game
                match load_game() {
                    Ok((objects, mut game)) => {
                        let mut cells: Vec<RefCell<Object>> = vec![];
                        for object in objects {
                            cells.push(RefCell::new(object));
                        }

                        let mut object_manager = ObjectsManager {objects: cells};
                        initialise_fov(&game.map, tcod);
                        play_game(&mut object_manager, &mut game, tcod);
                        tcod.root.clear();
                    },
                    Err(_e) => {
                        msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                        tcod.root.clear();
                        continue;
                    }
                }
            },
            Some(2) => { // quit
                break;
            },
            _ => {},
        }
    }
}

fn save_game(object_manager: &ObjectsManager, game: &Game) -> Result<(), Box<Error>> {
    let save_data = try! { json::encode(&(&object_manager.objects, game)) };
    let mut file = try! { File::create("savegame") };
    try! { file.write_all(save_data.as_bytes()) };
    Ok(())
}

fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = try! { File::open("savegame") };
    try! { file.read_to_string(&mut json_save_state) };
    let result = try! { json::decode::<(Vec<Object>, Game)>(&json_save_state) };
    Ok(result)
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

   main_menu(&mut tcod);
}
