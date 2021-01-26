#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f3xx_hal::{delay, pac, prelude::*, spi::Spi};

use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

use mmxlviii::board::Board;

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

    // Confirm that our 2048 library compiles okay
    let mut board = Board::empty();
    for _ in 0..3 {
        board.set_random();
    }

    // Some sample colours to display
    let red = RGB8 {
        r: 0x3f,
        g: 0x0,
        b: 0x0,
    };
    let blue = RGB8 {
        r: 0x0,
        g: 0x0,
        b: 0x3f,
    };

    loop {
        status_led.toggle().unwrap();

        board_leds.write([red].iter().cloned()).unwrap();
        delay.delay_ms(200u16);
        board_leds.write([blue].iter().cloned()).unwrap();
        delay.delay_ms(200u16);
    }
}
