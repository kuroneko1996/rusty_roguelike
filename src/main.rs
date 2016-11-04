extern crate tcod;
extern crate rand;

use std::cell::RefCell;
use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap};
use tcod::input::{self, Event, Mouse, Key};

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

fn render_all(tcod: &mut Tcod, object_manager: &mut ObjectsManager, game: &mut Game,
              fov_recompute: bool) 
{
    // draw map
    if fov_recompute {
        let player = object_manager.objects[PLAYER].borrow();
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let wall = game.map[x as usize][y as usize].block_sight;
                let visible = tcod.fov.is_in_fov(x, y);
                let color = match (visible, wall) {
                    // outside fov
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };

                // render only explored tiles
                let explored = &mut game.map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }
                if *explored {
                    tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }

    // draw objects
    object_manager.draw(&mut tcod.con, &tcod.fov);

    // copy buffer
    blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0, 0), 1.0, 1.0);

    // draw the gui panel
    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // draw the messages
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // draw stats
    {
        let player = object_manager.objects[PLAYER].borrow();
        let hp = player.fighter.map_or(0, |f| f.hp);
        let max_hp = player.fighter.map_or(0, |f| f.max_hp);
        render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);
    }
    

    // display names under mouse
    tcod.panel.set_default_foreground(colors::LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(tcod.mouse, object_manager, &tcod.fov));
    blit(&tcod.panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0, PANEL_Y), 1.0, 1.0);
}

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

fn render_bar(panel: &mut Offscreen, x: i32, y: i32, total_width: i32, name: &str, value: i32, maximum: i32,
                bar_color: Color, back_color: Color) 
{
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;
    // background 
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);
    // bar
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }
    // text on top
    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(x + total_width / 2, y, BackgroundFlag::None, TextAlignment::Center, 
        format!("{}: {}/{}", name, value, maximum));
}

fn get_names_under_mouse(mouse: Mouse, object_manager: &mut ObjectsManager, fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = object_manager.objects
        .iter()
        .map(|c| c.borrow())
        .filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    use std::ascii::AsciiExt;

    assert!(options.len() <= MAX_INVENTORY_SIZE as usize, 
        format!("Cannot have a menu with more than {} options", MAX_INVENTORY_SIZE));

    // calculate height of the window
    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32,  BackgroundFlag::None, TextAlignment::Left, text);
    }

    // "blit" to the center of root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);
    // and show data immediately
    root.flush();
    let key = root.wait_for_keypress(true);

    // converts ASCII key to index (a is 0, b is 1, etc)
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[RefCell<Object>], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty".into()]
    } else {
        inventory.iter().map(|c| { c.borrow().name.clone() }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn show_help(root: &mut Root) {
    let width = HELP_WIDTH;
    let help_text = "Press arrows or numpad buttons to move. Use 'g' to pick up items, \
                    'i' to open an inventory, 'd' to drop item. '?' or '/' for this help. \
                    Press any key to close this window.";
    let height = 8;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, help_text);

    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    root.flush();
    root.wait_for_keypress(true);
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
    message(&mut game.log, "Welcome stranger! Prepare to die in these catacombs. Hahaha.", colors::RED);

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
            object_manager.ai_turn(&game.map, &tcod.fov, &mut game.log);
        }
    }
}
