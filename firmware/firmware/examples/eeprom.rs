//! Stores some data on an AT24C256C EEPROM.
//! Then reads it again and if it matches, blinks LED 0.
//!
//! Introductory blog post here:
//! https://blog.eldruin.com/24x-serial-eeprom-driver-in-rust/
//!
//! This example is runs on the STM32F3 Discovery board using I2C1.
//!
//! ```
//! F3  <-> AT24C256
//! GND <-> GND
//! +5V <-> +5V
//! PB7 <-> SDA
//! PB6 <-> SCL
//! ```
//!
//! Run with:
//! `cargo run --example at24c256-f3 --target thumbv7em-none-eabihf`,

#![deny(unsafe_code)]
#![no_std]
#![no_main]

use core::convert::TryInto;
use cortex_m_rt::entry;
use heapless::Vec;
use mmxlviii::game_board::GameBoard;
use panic_rtt_target as _;
use postcard::{from_bytes, to_vec};
use rtt_target::{rprintln, rtt_init_print};
use stm32f3xx_hal::{self as hal, delay::Delay, pac, prelude::*};

use eeprom24x::{Eeprom24x, SlaveAddr};

const BUFFER_SIZE: usize = 128;
const PAGE_SIZE: usize = 16;
const DATA_SIZE: usize = 2 * PAGE_SIZE;
const MEMORY_BASE: u32 = 0x00;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("AT24C256 example");

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(cp.SYST, clocks);

    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
    let mut led = gpioa
        .pa3
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let mut scl =
        gpiob
            .pb6
            .into_af4_open_drain(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);
    let mut sda =
        gpiob
            .pb7
            .into_af4_open_drain(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);
    scl.internal_pull_up(&mut gpiob.pupdr, true);
    sda.internal_pull_up(&mut gpiob.pupdr, true);

    let i2c = hal::i2c::I2c::new(
        dp.I2C1,
        (scl, sda),
        100.kHz().try_into().unwrap(),
        clocks,
        &mut rcc.apb1,
    );

    let mut board = GameBoard::empty();
    board.set_random();
    board.set_random();
    board.make_move(mmxlviii::board::Direction::Right);
    board.set_random();
    board.make_move(mmxlviii::board::Direction::Up);
    board.set_random();
    let mut bytes: Vec<u8, BUFFER_SIZE> = to_vec(&board).unwrap();

    rprintln!("Board: {:?}", board);
    rprintln!("Bytes: {:?}", bytes);
    rprintln!("Bytes len: {}", bytes.len());

    bytes.resize(DATA_SIZE, 0).unwrap();

    let mut eeprom = Eeprom24x::new_24x08(i2c, SlaveAddr::Alternative(false, true, true));

    bytes
        .chunks(PAGE_SIZE)
        .enumerate()
        .for_each(|(page_num, page)| {
            let page_address = MEMORY_BASE + (page_num * PAGE_SIZE) as u32;

            rprintln!("Writing page {} at address {}", page_num, page_address);
            eeprom.write_page(page_address, page).unwrap();

            // wait maximum time necessary for write
            delay.delay_ms(5_u16);
        });

    loop {
        let mut data = [0; DATA_SIZE];
        eeprom.read_data(MEMORY_BASE, &mut data).unwrap();
        eeprom
            .read_data(MEMORY_BASE + PAGE_SIZE as u32, &mut data[PAGE_SIZE..])
            .unwrap();
        match from_bytes::<GameBoard>(&data) {
            Ok(board) => rprintln!("Parsed a board from eeprom: {:?}", board),
            Err(_) => rprintln!("Error reading board"),
        };

        let mut equal = true;
        for i in 0..PAGE_SIZE {
            if data[i] != bytes[i] {
                equal = false;
            }
        }
        if equal {
            led.set_high().unwrap();
            delay.delay_ms(5000_u16);
            led.set_low().unwrap();
            delay.delay_ms(5000_u16);
        }
    }
}
