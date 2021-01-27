#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f3xx_hal::{delay, pac, prelude::*, spi::Spi};

use smart_leds::SmartLedsWrite;
use ws2812_spi::Ws2812;

use mmxlviii::{board::IntoBoard, game_board::GameBoard};

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
        .sysclk(24.mhz())
        .pclk1(12.mhz())
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

    // Create the 2048 board
    let mut board = GameBoard::empty();

    // Each loop, add a 2 or 4 to an empty tile.
    // If the board is full, clear it instead.
    loop {
        if board.is_full() {
            board.clear();
        } else {
            board.set_random();
        }
        // TODO: Figure out the typing so the below line is cleaner
        board_leds
            .write(board.into_board().into_iter().cloned())
            .unwrap();

        status_led.toggle().unwrap();
        delay.delay_ms(200u16);
        delay.delay_ms(200u16);
    }
}
