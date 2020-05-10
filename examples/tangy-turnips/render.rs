use crate::tangy::{EntityKey, World, GRID_HEIGHT, GRID_WIDTH};
use sweet_turnips::sprites::{Color, SpriteGrid};

const MAX_X: u8 = GRID_WIDTH - 1;

pub fn sprite_grid_from_world(
    world: &World,
) -> SpriteGrid {
    let mut sprite_grid = SpriteGrid::new();

    let y_transit = (world.ticks % GRID_HEIGHT as usize) as u8;

    for y in 0..GRID_HEIGHT {
        if y == y_transit {
            sprite_grid.small_circle_at(0, y);
            sprite_grid.small_circle_at(MAX_X, y);
        } else {
            sprite_grid.big_circle_at(0, y);
            sprite_grid.big_circle_at(MAX_X, y);
        }
    }

    sprite_grid
}
