use smart_leds::{colors::GRAY, RGB8};

use crate::board::{Board, Coord, IntoBoard, SIZE};
use core::fmt::Debug;

const BASE: u32 = 10;
const SCORE_COLOUR: RGB8 = GRAY;

/// Compute base 10 exponent of an integer.
fn compute_exponent(n: u32) -> u32 {
    let mut exponent = 0;
    let mut remaining = n;
    while remaining >= BASE {
        exponent += 1;
        remaining /= BASE;
    }
    exponent
}

/// Compute 2 digit base 10 mantissa of an integer.
/// The most significant digit is returned first.
fn compute_mantissa(n: u32) -> (u32, u32) {
    let mut remaining = n;
    while remaining >= BASE * BASE {
        remaining /= BASE;
    }
    if remaining < BASE {
        remaining *= BASE;
    }
    let d0 = remaining / BASE;
    let d1 = remaining - BASE * d0;
    (d0, d1)
}

/// Transform number into 4-bit (SIZE-bit) binary representation.
/// The most significant bit is returned first.
fn int_to_bin4(n: u32) -> [bool; SIZE] {
    let mut result = [false; SIZE];
    let mut remaining = n;
    for i in 0..SIZE {
        result[SIZE - i - 1] = remaining % 2 == 1;
        remaining /= 2;
    }
    result
}

pub struct ScoreBoard {
    score: u32,
    board: Board,
}

impl ScoreBoard {
    /// Create a board with a score
    pub fn from_score(score: u32) -> ScoreBoard {
        let mut board = Board::new();

        let exp_bits = int_to_bin4(compute_exponent(score));

        let (d0, d1) = compute_mantissa(score);
        let d0_bits = int_to_bin4(d0);
        let d1_bits = int_to_bin4(d1);

        for i in 0..SIZE {
            if exp_bits[i] {
                board.set_led(Coord::new(i, 0).unwrap(), SCORE_COLOUR);
            }
            if d0_bits[i] {
                board.set_led(Coord::new(i, SIZE - 1).unwrap(), SCORE_COLOUR)
            }
            if d1_bits[i] {
                board.set_led(Coord::new(i, SIZE - 2).unwrap(), SCORE_COLOUR)
            }
        }

        ScoreBoard { score, board }
    }
}

impl Debug for ScoreBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ScoreBoard")
            .field("score", &self.score)
            .finish()
    }
}

impl IntoBoard for ScoreBoard {
    fn into_board(&self) -> Board {
        self.board
    }
}

#[cfg(test)]
mod tests {
    use smart_leds::colors::BLACK;

    use super::*;

    #[test]
    fn test_compute_exponent() {
        assert_eq!(compute_exponent(0), 0);
        assert_eq!(compute_exponent(1), 0);
        assert_eq!(compute_exponent(9), 0);
        assert_eq!(compute_exponent(10), 1);
        assert_eq!(compute_exponent(50_097), 4);
        assert_eq!(compute_exponent(999_999_999), 8);
        assert_eq!(compute_exponent(1_000_000_000), 9);
    }

    #[test]
    fn test_compute_mantissa() {
        assert_eq!(compute_mantissa(0), (0, 0));
        assert_eq!(compute_mantissa(1), (1, 0));
        assert_eq!(compute_mantissa(10), (1, 0));
        assert_eq!(compute_mantissa(11), (1, 1));
        assert_eq!(compute_mantissa(473), (4, 7));
        assert_eq!(compute_mantissa(999_999_999), (9, 9));
        assert_eq!(compute_mantissa(1_010_000_000), (1, 0));
    }

    #[test]
    fn test_int_to_bin4() {
        assert_eq!(int_to_bin4(0), [false, false, false, false]);
        assert_eq!(int_to_bin4(1), [false, false, false, true]);
        assert_eq!(int_to_bin4(10), [true, false, true, false]);
        assert_eq!(int_to_bin4(15), [true, true, true, true]);
        assert_eq!(int_to_bin4(17), [false, false, false, true]);
    }

    #[test]
    fn test_from_score() {
        let scoreboard = ScoreBoard::from_score(0);
        assert!(scoreboard.board.into_iter().all(|&led| led == BLACK));
    }
}
