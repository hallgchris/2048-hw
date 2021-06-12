#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f3xx_hal::{delay, pac, prelude::*, spi::Spi};

use smart_leds::{brightness, gamma, SmartLedsWrite};
use ws2812_spi::Ws2812;

use mmxlviii::{
    board::{Direction, IntoBoard},
    game_board::GameBoard,
    score_board::ScoreBoard,
};

#[entry]
fn main() -> ! {
    // Prepare our peripherals
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);

    let clocks = rcc
        .cfgr
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    // Set up SPI for WS2812b LEDs
    let (sck, miso, mosi) = (
        gpioa.pa5.into_af5(&mut gpioa.moder, &mut gpioa.afrl),
        gpioa.pa6.into_af5(&mut gpioa.moder, &mut gpioa.afrl),
        gpiob.pb5.into_af5(&mut gpiob.moder, &mut gpiob.afrl),
    );
    let spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        ws2812_spi::MODE,
        3.mhz(),
        clocks,
        &mut rcc.apb2,
    );
    let mut board_leds = Ws2812::new(spi);

    // Prepare other useful bits
    let mut status_led = gpioa
        .pa3
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    let mut delay = delay::Delay::new(cp.SYST, clocks);

    let up_pin = gpioa
        .pa11
        .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
    let down_pin = gpioa
        .pa10
        .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
    let left_pin = gpioa
        .pa8
        .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
    let right_pin = gpioa
        .pa9
        .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);

    let a_pin = gpiob
        .pb6
        .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);

    // Create the 2048 board
    let mut board = GameBoard::empty();
    board.set_random();

    let brightness_level = 127;

    let mut debouncer = false;

    loop {
        let mut direction: Option<Direction> = None;
        if up_pin.is_high().unwrap() {
            direction = Some(Direction::Up);
        } else if down_pin.is_high().unwrap() {
            direction = Some(Direction::Down);
        } else if left_pin.is_high().unwrap() {
            direction = Some(Direction::Left);
        } else if right_pin.is_high().unwrap() {
            direction = Some(Direction::Right);
        }

        debouncer = match direction {
            Some(chosen_direction) => {
                if !debouncer && board.make_move(chosen_direction) {
                    board.set_random();
                }
                true
            }
            None => false,
        };

        let leds = match a_pin.is_low() {
            Ok(true) => ScoreBoard::from_score(board.get_score()).into_board(),
            Ok(false) | Err(_) => board.into_board(),
        };

        // TODO: Figure out the typing so the below line is cleaner
        board_leds
            .write(brightness(
                gamma(leds.into_iter().cloned()),
                brightness_level,
            ))
            .unwrap();

        status_led.toggle().unwrap();
        delay.delay_ms(10u16);
    }
}
