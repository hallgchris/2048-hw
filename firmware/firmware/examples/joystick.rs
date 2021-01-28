#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f3xx_hal::{
    delay,
    gpio::{
        gpioa::{PA10, PA11, PA8, PA9},
        Input, PullUp,
    },
    pac,
    prelude::*,
    spi::Spi,
};

use smart_leds::{
    colors::{BLUE, GREEN, RED, YELLOW},
    SmartLedsWrite,
};
use ws2812_spi::Ws2812;

use mmxlviii::board::{Board, Coord, IntoBoard, SIZE};

struct JoystickDemoBoard {
    up_pin: PA11<Input<PullUp>>,
    down_pin: PA10<Input<PullUp>>,
    left_pin: PA8<Input<PullUp>>,
    right_pin: PA9<Input<PullUp>>,
}

impl IntoBoard for JoystickDemoBoard {
    fn into_board(&self) -> Board {
        // TODO: Use interrupts instead of polling
        let mut board = Board::new();
        if self.up_pin.is_high().unwrap() {
            (0..SIZE).for_each(|x| board.set_led(Coord::new(x, SIZE - 1).unwrap(), RED));
        } else if self.down_pin.is_high().unwrap() {
            (0..SIZE).for_each(|x| board.set_led(Coord::new(x, 0).unwrap(), YELLOW));
        } else if self.left_pin.is_high().unwrap() {
            (0..SIZE).for_each(|y| board.set_led(Coord::new(0, y).unwrap(), GREEN));
        } else if self.right_pin.is_high().unwrap() {
            (0..SIZE).for_each(|y| board.set_led(Coord::new(SIZE - 1, y).unwrap(), BLUE));
        }
        return board;
    }
}

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

    // Set up joystick demo
    let board = JoystickDemoBoard {
        left_pin: gpioa
            .pa8
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr),
        right_pin: gpioa
            .pa9
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr),
        down_pin: gpioa
            .pa10
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr),
        up_pin: gpioa
            .pa11
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr),
    };

    loop {
        board_leds
            .write(board.into_board().into_iter().cloned())
            .unwrap();

        status_led.toggle().unwrap();
        delay.delay_ms(20u16);
    }
}
