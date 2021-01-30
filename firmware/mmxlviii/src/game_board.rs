use core::fmt::Debug;

use heapless::{consts, Vec};
use rand::RngCore;
use smart_leds::{
    colors::{BLACK, WHITE},
    hsv::{hsv2rgb, Hsv},
    RGB8,
};
use wyhash::WyRng;

use crate::board::{Board, Coord, Direction, IntoBoard, SIZE};

#[derive(Debug, PartialEq)]
enum TileMoveResult {
    NoMove,
    Free(Coord),
    Merge(Coord),
}

pub struct GameBoard {
    tiles: [u32; SIZE * SIZE],
    rng: WyRng,
}

impl GameBoard {
    /// Create an empty board.
    pub fn empty() -> GameBoard {
        GameBoard::full_of(0)
    }

    /// Create a board entirely filled with some tile.
    fn full_of(value: u32) -> GameBoard {
        GameBoard {
            tiles: [value; SIZE * SIZE],
            rng: WyRng::default(),
        }
    }

    /// Clears all tiles from the board.
    pub fn clear(&mut self) {
        self.tiles = [0; SIZE * SIZE];
    }

    /// Get the maximum value of any tile on the board.
    pub fn max_tile(&self) -> u32 {
        *self
            .tiles
            .iter()
            .max()
            .expect("there were no tiles on the board")
    }

    /// Returns true only if all tiles are filled (non-zero)
    pub fn is_full(&self) -> bool {
        self.tiles.iter().all(|&tile| tile != 0)
    }

    /// Get the value of a tile on the board.
    fn get_tile(&self, coord: Coord) -> u32 {
        self.tiles[coord.board_index()]
    }

    /// Set a tile on the board to some value.
    fn set_tile(&mut self, coord: Coord, value: u32) {
        self.tiles[coord.board_index()] = value;
    }

    /// Set a tile on the board to empty.
    fn clear_tile(&mut self, coord: Coord) {
        self.set_tile(coord, 0)
    }

    /// Get the locations of all empty tiles.
    fn vacant_tiles(&self) -> impl Iterator<Item = Coord> + '_ {
        self.tiles
            .iter()
            .enumerate()
            .filter(|&(_index, &value)| value == 0)
            .map(|(index, _value)| {
                Coord::from_index(index).expect("index was invalid for creating Coord")
            })
    }

    /// Get the location of a random empty tile.
    /// Returns `None` if no empty tiles are present.
    fn random_vacant_tile(&mut self) -> Option<Coord> {
        let mut vacant_tiles = Vec::<Coord, consts::U16>::new();
        let num_vacant = self.vacant_tiles().fold(0, |count, coord| {
            vacant_tiles
                .push(coord)
                .expect("more than 16 tiles were vacant");
            count + 1
        });
        if num_vacant > 0 {
            let index = (self.rng.next_u32() as usize) % num_vacant;
            Some(vacant_tiles[index])
        } else {
            None
        }
    }

    /// Set a random empty tile to a 2 or a 4.
    /// If no empty tile is found, then no changes are made and `false` is returned.
    pub fn set_random(&mut self) -> bool {
        if let Some(tile) = self.random_vacant_tile() {
            let value = if self.rng.next_u32() % 10 == 0 { 2 } else { 1 };
            self.set_tile(tile, value);
            true
        } else {
            false
        }
    }

    /// Get the board tiles.
    /// FIXME: This is temporary, make some nice pretty print instead
    pub fn get_board(&self) -> [u32; SIZE * SIZE] {
        self.tiles
    }

    /// Return two arrays specifying the order to attempt to move tiles.
    fn get_traversal_order(&self, direction: Direction) -> ([usize; SIZE], [usize; SIZE]) {
        let x_traversal_order = match direction {
            Direction::Right => [3, 2, 1, 0],
            _ => [0, 1, 2, 3],
        };
        let y_traversal_order = match direction {
            Direction::Up => [3, 2, 1, 0],
            _ => [0, 1, 2, 3],
        };
        (x_traversal_order, y_traversal_order)
    }

    /// Find the farthest position in the specified direction that the tile can move to
    fn find_tile_move(&self, tile_coord: Coord, direction: Direction) -> TileMoveResult {
        let mut prev = tile_coord;
        loop {
            match prev.neighbour(direction) {
                None => break, // Edge of board has been reached
                Some(next) => {
                    if self.get_tile(next) == self.get_tile(tile_coord) {
                        // Next tile is same as tile that we're moving, so merge
                        return TileMoveResult::Merge(next);
                    } else if self.get_tile(next) != 0 {
                        // Next tile is occupied but not mergable.
                        break;
                    }
                    prev = next;
                }
            };
        }
        // Prev is the furthest we can move and it's not a merge.
        // Now check if we've moved at all.
        if tile_coord == prev {
            TileMoveResult::NoMove
        } else {
            TileMoveResult::Free(prev)
        }
    }

    /// Moves all tiles as far as possible in the specified direction.
    /// Returns true if any tiles were moved.
    pub fn make_move(&mut self, direction: Direction) -> bool {
        let (x_traversals, y_traversals) = self.get_traversal_order(direction);
        let mut moved = false;

        for &x in x_traversals.iter() {
            for &y in y_traversals.iter() {
                let coord = Coord::new(x, y).unwrap();
                let value = self.get_tile(coord);

                if value == 0 {
                    continue;
                }

                match self.find_tile_move(coord, direction) {
                    TileMoveResult::NoMove => {}
                    TileMoveResult::Free(new_coord) => {
                        self.set_tile(new_coord, value);
                        self.clear_tile(coord);
                        moved = true;
                    }
                    TileMoveResult::Merge(new_coord) => {
                        self.set_tile(new_coord, value + 1);
                        self.clear_tile(coord);
                        moved = true;
                    }
                }
            }
        }

        return moved;
    }
}

/// Map blank tiles to be off
/// Map 2 to 1024 tiles to rainbow colours
/// Map 2048 to 8192 tiles to shades of white
/// Map tiles greater than 8192 to the same white as 8192
fn get_tile_colour(value: u32) -> RGB8 {
    match value {
        0 => BLACK,
        1..=10 => hsv2rgb(Hsv {
            hue: (value as u8 - 1) * (255 / 10),
            sat: 0xff,
            val: 0xbf,
        }),
        11..=13 => hsv2rgb(Hsv {
            hue: 0,
            sat: 0,
            val: (value as u8 - 11) * (128 / 3) + 127,
        }),
        _ => WHITE,
    }
}

impl PartialEq for GameBoard {
    fn eq(&self, other: &Self) -> bool {
        self.tiles == other.tiles
    }
}

impl Eq for GameBoard {}

impl Debug for GameBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GameBoard")
            .field("tiles", &self.tiles)
            .finish()
    }
}

impl IntoBoard for GameBoard {
    /// Return a board where 2s are red and 4s are blue.
    fn into_board(&self) -> Board {
        let mut board = Board::new();
        for index in 0..(SIZE * SIZE) {
            let coord = Coord::from_index(index).unwrap();
            let colour = get_tile_colour(self.tiles[index]);
            board.set_led(coord, colour);
        }
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_board_index() {
        let index = 7;
        let coord = Coord::from_index(index).unwrap();
        assert_eq!(coord.board_index(), index)
    }

    #[test]
    fn test_empty_instantiation() {
        let board = GameBoard::empty();
        assert!(board.tiles.iter().all(|&tile| tile == 0))
    }

    #[test]
    fn test_clear() {
        let mut board = GameBoard::full_of(1);
        board.clear();
        assert!(board.tiles.iter().all(|&tile| tile == 0));
    }

    #[test]
    fn test_max_tile() {
        let mut board = GameBoard::empty();
        board.tiles[7] = 11;
        assert_eq!(board.max_tile(), 11)
    }

    #[test]
    fn test_is_full() {
        let mut board = GameBoard::full_of(1);
        assert!(board.is_full());
        board.set_tile(Coord::new(0, 0).unwrap(), 0);
        assert!(!board.is_full());
    }

    #[test]
    fn test_get_tile() {
        let coord = Coord::new(2, 3).unwrap();
        let mut board = GameBoard::empty();
        board.set_tile(coord, 5);
        assert_eq!(board.get_tile(coord), 5)
    }

    #[test]
    fn test_set_tile() {
        let coord = Coord::new(2, 3).unwrap();
        let mut board = GameBoard::empty();
        board.set_tile(coord, 5);
        assert_eq!(board.tiles[coord.board_index()], 5)
    }

    #[test]
    fn test_clear_tile() {
        let coord = Coord::new(2, 3).unwrap();
        let mut board = GameBoard::full_of(1);
        board.clear_tile(coord);
        assert_eq!(board.tiles[coord.board_index()], 0)
    }

    #[test]
    fn test_vacant_tiles_all() {
        let board = GameBoard::empty();
        let ans = board.vacant_tiles();
        assert_eq!(ans.count(), SIZE * SIZE);
    }

    #[test]
    fn test_vacant_tiles_some() {
        let mut board = GameBoard::empty();
        board.set_tile(Coord::new(2, 0).unwrap(), 3);
        board.set_tile(Coord::new(1, 1).unwrap(), 1);
        board.set_tile(Coord::new(1, 3).unwrap(), 8);
        assert_eq!(board.vacant_tiles().count(), SIZE * SIZE - 3);
    }

    #[test]
    fn test_vacant_tiles_all_but_one() {
        let mut board = GameBoard::full_of(1);
        let vacant_tile = Coord::new(3, 0).unwrap();
        board.set_tile(vacant_tile, 0);
        assert_eq!(board.vacant_tiles().nth(0).unwrap(), vacant_tile);
    }

    #[test]
    fn test_vacant_tiles_none() {
        let board = GameBoard::full_of(1);
        assert_eq!(board.vacant_tiles().count(), 0);
    }

    #[test]
    fn test_random_vacant_tile() {
        let mut board = GameBoard::full_of(1);
        let vacant_tile = Coord::new(3, 0).unwrap();
        board.set_tile(vacant_tile, 0);
        assert_eq!(board.random_vacant_tile().unwrap(), vacant_tile);
    }

    #[test]
    fn test_random_vacant_tile_none() {
        let mut board = GameBoard::full_of(1);
        assert!(!board.set_random())
    }

    #[test]
    fn test_set_random() {
        let mut board = GameBoard::empty();
        board.set_random();
        assert!(board.max_tile() != 0)
    }

    #[test]
    fn test_find_tile_move() {
        let mut board = GameBoard::empty();
        let start_coord = Coord::new(1, 0).unwrap();
        board.set_tile(start_coord, 1);
        board.set_tile(Coord::new(3, 0).unwrap(), 1);
        board.set_tile(Coord::new(0, 0).unwrap(), 2);

        // Board looks like
        // |         |
        // |         |
        // |         |
        // | 2 1   1 |

        assert_eq!(
            board.find_tile_move(start_coord, Direction::Up),
            TileMoveResult::Free(Coord::new(1, 3).unwrap())
        );
        assert_eq!(
            board.find_tile_move(start_coord, Direction::Down),
            TileMoveResult::NoMove
        );
        assert_eq!(
            board.find_tile_move(start_coord, Direction::Left),
            TileMoveResult::NoMove
        );
        assert_eq!(
            board.find_tile_move(start_coord, Direction::Right),
            TileMoveResult::Merge(Coord::new(3, 0).unwrap())
        );
    }

    #[test]
    fn test_make_move() {
        let mut board = GameBoard::empty();
        board.set_tile(Coord::new(0, 0).unwrap(), 1);
        assert!(board.make_move(Direction::Up));

        let mut expected_board = GameBoard::empty();
        expected_board.set_tile(Coord::new(0, 3).unwrap(), 1);

        assert_eq!(board, expected_board);

        board.set_tile(Coord::new(2, 3).unwrap(), 1);
        assert!(board.make_move(Direction::Right));

        expected_board.clear();
        expected_board.set_tile(Coord::new(3, 3).unwrap(), 2);

        assert_eq!(board, expected_board);

        assert!(!board.make_move(Direction::Right));

        assert_eq!(board, expected_board);
    }

    #[test]
    fn test_make_move_full_board() {
        let mut board = GameBoard::full_of(1);

        assert!(board.make_move(Direction::Down));
        assert_eq!(
            board.tiles,
            [2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        assert!(board.make_move(Direction::Up));
        assert_eq!(
            board.tiles,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3]
        );

        assert!(board.make_move(Direction::Left));
        assert_eq!(
            board.tiles,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 0, 0]
        );

        assert!(board.make_move(Direction::Right));
        assert_eq!(
            board.tiles,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5]
        );

        assert!(!board.make_move(Direction::Up));
    }

    #[test]
    fn test_get_colour() {
        for i in 0..(SIZE * SIZE) {
            get_tile_colour(i as u32);
        }
    }

    #[test]
    fn test_eq() {
        let coords = [
            Coord::new(3, 1).unwrap(),
            Coord::new(0, 2).unwrap(),
            Coord::new(1, 0).unwrap(),
        ];
        let mut board1 = GameBoard::empty();
        let mut board2 = GameBoard::empty();
        for &coord in coords.iter() {
            board1.set_tile(coord, 1);
            board2.set_tile(coord, 1);
        }
        assert_eq!(board1, board2);

        let board3 = GameBoard::empty();
        assert_ne!(board1, board3);
    }
}
