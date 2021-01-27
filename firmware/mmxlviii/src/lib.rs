#![no_std]

pub mod board;
pub mod game_board;

pub fn add_one(n: i32) -> i32 {
    n + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(add_one(2), 3);
    }
}
