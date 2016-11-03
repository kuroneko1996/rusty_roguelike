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

use config::*;
use map::*;
use object::*;
use messages::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn render_all(root: &mut Root, con: &mut Offscreen, panel: &mut Offscreen, mouse: Mouse,
                    object_manager: &mut ObjectsManager, map: &mut Map, messages: &Messages,
                    fov_map: &mut FovMap, fov_recompute: bool) 
{
    // draw map
    if fov_recompute {
        let player = object_manager.objects[PLAYER].borrow();
        fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let wall = map[x as usize][y as usize].block_sight;
                let visible = fov_map.is_in_fov(x, y);
                let color = match (visible, wall) {
                    // outside fov
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };

                // render only explored tiles
                let explored = &mut map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }
                if *explored {
                    con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }

    // draw objects
    object_manager.draw(con, fov_map);

    // copy buffer
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);

    // draw the gui panel
    panel.set_default_background(colors::BLACK);
    panel.clear();

    // draw the messages
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in messages.iter().rev() {
        let msg_height = panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        panel.set_default_foreground(color);
        panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // draw stats
    {
        let player = object_manager.objects[PLAYER].borrow();
        let hp = player.fighter.map_or(0, |f| f.hp);
        let max_hp = player.fighter.map_or(0, |f| f.max_hp);
        render_bar(panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);
    }
    

    // display names under mouse
    panel.set_default_foreground(colors::LIGHT_GREY);
    panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(mouse, object_manager, fov_map));
    blit(panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), root, (0, PANEL_Y), 1.0, 1.0);
}

fn handle_keys(key: Key, root: &mut Root, map: &Map, object_manager: &mut ObjectsManager, inventory: &mut Vec<RefCell<Object>>,
    messages: &mut Messages) -> PlayerAction 
{
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let is_alive = object_manager.objects[PLAYER].borrow().alive;

    match (key, is_alive) {
        // Alt Enter fullscreen
        (Key { code: Enter, alt: true, ..}, _) => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, ..}, _) => Exit, // Exit game
        // Movement
        (Key { code: Up, .. }, true) => { object_manager.player_move_or_attack(0, -1, map, messages); TookTurn } ,
        (Key { code: Down, .. }, true) => { object_manager.player_move_or_attack(0, 1, map, messages); TookTurn },
        (Key { code: Left, .. }, true) => { object_manager.player_move_or_attack(-1, 0, map, messages); TookTurn },
        (Key { code: Right, .. }, true) => { object_manager.player_move_or_attack(1, 0, map, messages); TookTurn },
        // Inventory
        (Key { printable: 'g', .. }, true) => {
            let player_pos = object_manager.objects[PLAYER].borrow().pos();
            // pick up an item
            let item_id = object_manager.objects.iter().map(|c| c.borrow()).position(|object| {
                object.pos() == player_pos && object.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, object_manager, inventory, messages);
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

fn main() {
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rusty Roguelike")
        .init();
    tcod::system::set_fps(LIMIT_FPS);
    let mut con = Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut panel = Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT);

    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![RefCell::new(player)];
    let (mut map, (player_start_x, player_start_y)) = make_map(&mut objects);

    let mut object_manager = ObjectsManager { objects: objects };
    object_manager.objects[PLAYER].borrow_mut().set_pos(player_start_x, player_start_y);

    // log messages and their colors
    let mut messages = vec![];

    // items
    let mut inventory: Vec<RefCell<Object>> = vec![];

    // greeting
    message(&mut messages, "Welcome stranger! Prepare to die in these catacombs. Hahaha.", colors::RED);

    // FOV
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            fov_map.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked)
        }
    }
    let mut previous_player_position = (-1, -1);

    let mut mouse = Default::default();
    let mut key = Default::default();

    while !root.window_closed() {
        let (player_x, player_y) = object_manager.objects[PLAYER].borrow().pos();
        let fov_recompute = previous_player_position != (player_x, player_y);

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }



        render_all(&mut root, &mut con, &mut panel, mouse, &mut object_manager, &mut map, &mut messages,
                    &mut fov_map, fov_recompute);

        root.flush();
      
        object_manager.draw_clear(&mut con);

        previous_player_position = (player_x, player_y);

        // player's turn
        let player_action = handle_keys(key, &mut root, &map, &mut object_manager, &mut inventory, &mut messages);
        if player_action == PlayerAction::Exit {
            break
        }

        // monsters turn
        if object_manager.objects[PLAYER].borrow().alive && player_action == PlayerAction::TookTurn {
            object_manager.ai_turn(&map, &fov_map, &mut messages);
        }
    }
}
