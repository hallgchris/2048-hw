use heapless::{consts, Vec};
use rand::RngCore;
use wyhash::WyRng;

const SIZE: usize = 4;

#[derive(Clone, Copy, Debug, Eq)]
pub struct Coord {
    x: usize,
    y: usize,
}

impl Coord {
    /// Create a new Coord from x and y coordinates
    pub fn new(x: usize, y: usize) -> Option<Coord> {
        if x < SIZE && y < SIZE {
            Some(Coord { x, y })
        } else {
            None
        }
    }

    /// Create a new Coord from an index on the board
    pub fn from_index(index: usize) -> Option<Coord> {
        if index < SIZE * SIZE {
            Some(Coord {
                x: index % SIZE,
                y: index / SIZE,
            })
        } else {
            None
        }
    }

    /// Get the board index for this Coord
    fn board_index(&self) -> usize {
        self.x + SIZE * self.y
    }
}

impl PartialEq for Coord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

pub struct Board {
    tiles: [u32; SIZE * SIZE],
    rng: WyRng,
}

impl Board {
    /// Create an empty board.
    pub fn empty() -> Board {
        Board::full_of(0)
    }

    /// Create a board entirely filled with some tile.
    fn full_of(value: u32) -> Board {
        Board {
            tiles: [value; SIZE * SIZE],
            rng: WyRng::default(),
        }
    }

    /// Get the maximum value of any tile on the board.
    pub fn max_tile(&self) -> u32 {
        *self
            .tiles
            .iter()
            .max()
            .expect("there were no tiles on the board")
    }

    /// Set a tile on the board to some value.
    fn set_tile(&mut self, coord: Coord, value: u32) {
        self.tiles[coord.board_index()] = value;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_coord() {
        let (x, y) = (0, 3);
        let coord = Coord::new(x, y).unwrap();
        assert_eq!((coord.x, coord.y), (x, y))
    }

    #[test]
    fn test_new_invalid_coord() {
        assert!(Coord::new(0, SIZE).is_none())
    }

    #[test]
    fn test_from_valid_index() {
        let coord1 = Coord::from_index(0).unwrap();
        assert_eq!((coord1.x, coord1.y), (0, 0));
        let coord2 = Coord::from_index(7).unwrap();
        assert_eq!((coord2.x, coord2.y), (3, 1));
        let coord3 = Coord::from_index(15).unwrap();
        assert_eq!((coord3.x, coord3.y), (3, 3));
    }

    #[test]
    fn test_equality() {
        let coord1 = Coord::new(0, 1).unwrap();
        let coord2 = Coord::new(1, 0).unwrap();
        let coord3 = Coord::new(1, 0).unwrap();

        assert_eq!(coord1, coord1);
        assert_eq!(coord2, coord3);
        assert_ne!(coord1, coord2);
        assert_ne!(coord1, coord3);
    }

    #[test]
    fn test_from_invalid_index() {
        assert!(Coord::from_index(SIZE * SIZE).is_none())
    }

    #[test]
    fn test_get_board_index() {
        let index = 7;
        let coord = Coord::from_index(index).unwrap();
        assert_eq!(coord.board_index(), index)
    }

    #[test]
    fn test_empty_instantiation() {
        let board = Board::empty();
        assert!(board.tiles.iter().all(|&tile| tile == 0))
    }

    #[test]
    fn test_max_tile() {
        let mut board = Board::empty();
        board.tiles[7] = 11;
        assert_eq!(board.max_tile(), 11)
    }

    #[test]
    fn test_set_tile() {
        let coord = Coord::new(2, 3).unwrap();
        let mut board = Board::empty();
        board.set_tile(coord, 5);
        assert_eq!(board.tiles[coord.board_index()], 5)
    }

    #[test]
    fn test_vacant_tiles_all() {
        let board = Board::empty();
        let ans = board.vacant_tiles();
        assert_eq!(ans.count(), SIZE * SIZE);
    }

    #[test]
    fn test_vacant_tiles_some() {
        let mut board = Board::empty();
        board.set_tile(Coord::new(2, 0).unwrap(), 3);
        board.set_tile(Coord::new(1, 1).unwrap(), 1);
        board.set_tile(Coord::new(1, 3).unwrap(), 8);
        assert_eq!(board.vacant_tiles().count(), SIZE * SIZE - 3);
    }

    #[test]
    fn test_vacant_tiles_all_but_one() {
        let mut board = Board::full_of(1);
        let vacant_tile = Coord::new(3, 0).unwrap();
        board.set_tile(vacant_tile, 0);
        assert_eq!(board.vacant_tiles().nth(0).unwrap(), vacant_tile);
    }

    #[test]
    fn test_vacant_tiles_none() {
        let board = Board::full_of(1);
        assert_eq!(board.vacant_tiles().count(), 0);
    }

    #[test]
    fn test_random_vacant_tile() {
        let mut board = Board::full_of(1);
        let vacant_tile = Coord::new(3, 0).unwrap();
        board.set_tile(vacant_tile, 0);
        assert_eq!(board.random_vacant_tile().unwrap(), vacant_tile);
    }

    #[test]
    fn test_random_vacant_tile_none() {
        let mut board = Board::full_of(1);
        assert!(!board.set_random())
    }

    #[test]
    fn test_set_random() {
        let mut board = Board::empty();
        board.set_random();
        assert!(board.max_tile() != 0)
    }
}
