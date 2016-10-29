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

fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &mut Map, fov_map: &mut FovMap, fov_recompute: bool) {
    // draw map
    if fov_recompute {
        let player = &objects[0];
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
    for object in objects {
        object.draw(con);
    }

    // copy buffer
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn handle_keys(root: &mut Root, map: &Map, all_objects: &mut [Object]) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let (player_slice, objects) = all_objects.split_at_mut(1);
    let player = &mut player_slice[PLAYER];

    let key = root.wait_for_keypress(true);
    match (key, player.alive) {
        // Alt Enter fullscreen
        (Key { code: Enter, alt: true, ..}, _) => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, ..}, _) => Exit, // Exit game
        // Movement
        (Key { code: Up, .. }, true) => { player.move_by(0, -1, map, objects); TookTurn } ,
        (Key { code: Down, .. }, true) => { player.move_by(0, 1, map, objects); TookTurn },
        (Key { code: Left, .. }, true) => { player.move_by(-1, 0, map, objects); TookTurn },
        (Key { code: Right, .. }, true) => { player.move_by(1, 0, map, objects); TookTurn },

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
    let mut objects = vec![player];
    let (mut map, (player_start_x, player_start_y)) = make_map(&mut objects);
    objects[PLAYER].x = player_start_x;
    objects[PLAYER].y = player_start_y;


    // FOV
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            fov_map.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked)
        }
    }
    let mut previous_player_position = (-1, -1);

    while !root.window_closed() {
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut root, &mut con, &objects, &mut map, &mut fov_map, fov_recompute);

        root.flush();
      
        for object in &objects {
            object.clear(&mut con);
        }

        previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);

        // player's turn
        let player_action = handle_keys(&mut root, &map, &mut objects);
        if player_action == PlayerAction::Exit {
            break
        }

        // monsters turn
        if objects[PLAYER].alive && player_action == PlayerAction::TookTurn {
            for object in &objects {
                // check if not player
                if (object as *const _) != (&objects[PLAYER] as *const _) { // TODO replace this weird thing
                    println!("The {} growls", object.name);
                }
            }
        }
    }
}
