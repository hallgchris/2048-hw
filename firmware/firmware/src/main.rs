#![no_std]
#![no_main]

// logs messages to the host stderr; requires a debugger
use panic_semihosting as _;

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

use mmxlviii::board::Board;

#[entry]
fn main() -> ! {
    hprintln!("Hello, world!").unwrap();

    let mut board = Board::empty();
    for _ in 0..3 {
        board.set_random();
    }
    hprintln!("The current board is {:?}", board.get_board()).unwrap();

    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}
