const SIZE: usize = 4;

pub struct Board {
    tiles: [u32; SIZE * SIZE],
}

impl Board {
    pub fn empty() -> Board {
        Board {
            tiles: [0; SIZE * SIZE],
        }
    }

    pub fn max_tile(&self) -> u32 {
        *self
            .tiles
            .iter()
            .max()
            .expect("there were no tiles on the board")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
