use std::cell::RefCell;
use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::{self, Event, Mouse};
use tcod::colors::{self, Color};

use config::*;
use map::Map;
use messages::Messages;
use object::{Object, ObjectsManager};

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

pub fn target_tile(tcod: &mut Tcod, object_manager: &mut ObjectsManager, game: &mut Game, max_range: Option<f32>) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;

    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }

        render_all(tcod, object_manager, game, false);

        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let player = object_manager.objects[PLAYER].borrow();
        let in_range = max_range.map_or(true, |range| player.distance(x, y) <= range);

        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        }

        let escape = key.map_or(false, |k| k.code == Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None
        }
    }
}

pub fn render_all(tcod: &mut Tcod, object_manager: &mut ObjectsManager, game: &mut Game,
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

pub fn get_names_under_mouse(mouse: Mouse, object_manager: &mut ObjectsManager, fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = object_manager.objects
        .iter()
        .map(|c| c.borrow())
        .filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

pub fn render_bar(panel: &mut Offscreen, x: i32, y: i32, total_width: i32, name: &str, value: i32, maximum: i32,
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

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
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

pub fn inventory_menu(inventory: &[RefCell<Object>], header: &str, root: &mut Root) -> Option<usize> {
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

pub fn show_help(root: &mut Root) {
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
