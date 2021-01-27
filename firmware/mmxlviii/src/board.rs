use smart_leds::RGB8;

pub const SIZE: usize = 4;

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
    pub fn board_index(&self) -> usize {
        self.x + SIZE * self.y
    }

    /// Get the corresponding LED's index as wired on the PCB
    fn led_index(&self) -> usize {
        // Odd rows are reversed.
        match self.y {
            0 | 2 => SIZE * self.y + self.x,
            1 | 3 => SIZE * (self.y + 1) - self.x - 1,
            _ => 0,
        }
    }
}

impl PartialEq for Coord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

pub struct Board {
    leds: [RGB8; SIZE * SIZE],
}

impl Board {
    pub fn new() -> Board {
        Board {
            leds: [RGB8 { r: 0, g: 0, b: 0 }; SIZE * SIZE],
        }
    }

    /// Set the LED at some location to the provided colour
    pub fn set_led(&mut self, coord: Coord, colour: RGB8) {
        self.leds[coord.led_index()] = colour;
    }

    /// Get an iterator to the board's LEDs in the order they are on the PCB
    pub fn into_iter(&self) -> impl Iterator<Item = &RGB8> {
        self.leds.iter()
    }
}

pub trait IntoBoard {
    fn into_board(&self) -> Board;
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
    fn test_from_invalid_index() {
        assert!(Coord::from_index(SIZE * SIZE).is_none())
    }

    #[test]
    fn test_led_index() {
        let expected = [0, 1, 2, 3, 7, 6, 5, 4, 8, 9, 10, 11, 15, 14, 13, 12];
        for i in 0..expected.len() {
            assert_eq!(Coord::from_index(i).unwrap().led_index(), expected[i])
        }
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
}
