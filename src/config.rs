use tcod::colors::Color;
use tcod::map::FovAlgorithm;

pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;

// sizes and coordinates for the GUI
pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;
pub const INVENTORY_WIDTH: i32 = 50;
pub const HELP_WIDTH: i32 = 50;

// map size
pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

pub const LIMIT_FPS: i32 = 20;

pub const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
pub const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
pub const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
pub const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIUS: i32 = 10;

pub const MAX_ROOM_MONSTERS: i32 = 3;
pub const MAX_ROOM_ITEMS: i32 = 2;

pub const MAX_INVENTORY_SIZE: i32 = 26;

// player's index in objects vector
pub const PLAYER: usize = 0;

pub const HEAL_AMOUNT: i32 = 4;

// Spells
pub const LIGHTNING_RANGE: i32 = 5;
pub const LIGHTNING_DAMAGE: i32 = 20;
pub const CONFUSE_RANGE: i32 = 8;
pub const CONFUSE_NUM_TURNS: i32 = 10;
