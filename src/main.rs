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


fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &Map, fov_map: &mut FovMap, fov_recompute: bool) {
    // draw map
    if fov_recompute {
        println!("Fov recomputed");
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
                con.set_char_background(x, y, color, BackgroundFlag::Set);
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

fn handle_keys(root: &mut Root, player: &mut Object, map: &Map) -> bool {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = root.wait_for_keypress(true);
    match key {
        // Alt Enter fullscreen
        Key { code: Enter, alt: true, ..} => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
        },
        Key { code: Escape, ..} => return true, // Exit game
        // Movement
        Key { code: Up, .. } => player.move_by(0, -1, map),
        Key { code: Down, .. } => player.move_by(0, 1, map),
        Key { code: Left, .. } => player.move_by(-1, 0, map),
        Key { code: Right, .. } => player.move_by(1, 0, map),

        _ => {},
    }

    false
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

    let (map, (player_start_x, player_start_y)) = make_map();

    let player = Object::new(player_start_x, player_start_y, '@', colors::WHITE);
    let mut objects = [player];


    // FOV
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            fov_map.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked)
        }
    }
    let mut previous_player_position = (-1, -1);

    while !root.window_closed() {
        let fov_recompute = previous_player_position != (objects[0].x, objects[0].y);
        render_all(&mut root, &mut con, &objects, &map, &mut fov_map, fov_recompute);

        root.flush();
      
        for object in &objects {
            object.clear(&mut con);
        }

        let player = &mut objects[0];
        previous_player_position = (player.x, player.y);
        let exit = handle_keys(&mut root, player, &map);
        if exit {
            break;
        }
    }
}
