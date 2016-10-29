extern crate tcod;
extern crate rand;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};
use rand::Rng;

mod config;
mod tile;
mod map;
mod object;
mod rect;

use config::*;
use tile::*;
use map::*;
use object::*;
use rect::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn render_all(root: &mut Root, con: &mut Offscreen, object_manager: &mut ObjectsManager, 
                    map: &mut Map, fov_map: &mut FovMap, fov_recompute: bool) 
{
    // draw map
    if fov_recompute {
        let player = object_manager.get(PLAYER);
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
    object_manager.draw(con);

    // show the player's stats
    if let Some(fighter) = object_manager.get(PLAYER).fighter {
        root.print_ex(1, SCREEN_HEIGHT - 2, BackgroundFlag::None, TextAlignment::Left,
                        format!("HP: {}/{} ", fighter.hp, fighter.max_hp));
    }

    // copy buffer
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn handle_keys(root: &mut Root, map: &Map, object_manager: &mut ObjectsManager) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let is_alive = object_manager.get(PLAYER).alive;

    let key = root.wait_for_keypress(true);
    match (key, is_alive) {
        // Alt Enter fullscreen
        (Key { code: Enter, alt: true, ..}, _) => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, ..}, _) => Exit, // Exit game
        // Movement
        (Key { code: Up, .. }, true) => { object_manager.player_move_or_attack(0, -1, map); TookTurn } ,
        (Key { code: Down, .. }, true) => { object_manager.player_move_or_attack(0, 1, map); TookTurn },
        (Key { code: Left, .. }, true) => { object_manager.player_move_or_attack(-1, 0, map); TookTurn },
        (Key { code: Right, .. }, true) => { object_manager.player_move_or_attack(1, 0, map); TookTurn },

        _ => DidntTakeTurn,
    }
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

    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5,
    });

    let mut objects = vec![player];
    let (mut map, (player_start_x, player_start_y)) = make_map(&mut objects);

    let mut object_manager = ObjectsManager { objects: objects };
    object_manager.get(PLAYER).set_pos(player_start_x, player_start_y);

    // FOV
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            fov_map.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked)
        }
    }
    let mut previous_player_position = (-1, -1);

    while !root.window_closed() {
        let (player_x, player_y) = object_manager.get(PLAYER).pos();
        let fov_recompute = previous_player_position != (player_x, player_y);
        render_all(&mut root, &mut con, &mut object_manager, &mut map, &mut fov_map, fov_recompute);

        root.flush();
      
        object_manager.draw_clear(&mut con);

        previous_player_position = (player_x, player_y);

        // player's turn
        let player_action = handle_keys(&mut root, &map, &mut object_manager);
        if player_action == PlayerAction::Exit {
            break
        }

        // monsters turn
        if object_manager.get(PLAYER).alive && player_action == PlayerAction::TookTurn {
            object_manager.ai_turn(&map, &fov_map);
        }
    }
}
