#![no_std]
#![no_main]

use core::convert::TryInto;

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f3xx_hal::{
    delay,
    gpio::{
        gpioa::{PA10, PA11, PA8, PA9},
        gpiob::{PB6, PB7},
        Input,
    },
    pac,
    prelude::*,
    spi::Spi,
};

use smart_leds::{
    colors::{BLACK, BLUE, GREEN, RED, WHITE, YELLOW},
    SmartLedsWrite,
};
use ws2812_spi::Ws2812;

use mmxlviii::board::{Board, Coord, IntoBoard, SIZE};

struct JoystickDemoBoard {
    up_pin: PA11<Input>,
    down_pin: PA10<Input>,
    left_pin: PA8<Input>,
    right_pin: PA9<Input>,

    a_pin: PB6<Input>,
    b_pin: PB7<Input>,
}

impl IntoBoard for JoystickDemoBoard {
    fn into_board(&self) -> Board {
        let mut board = Board::new();

        // TODO: Use interrupts instead of polling
        if self.up_pin.is_high().unwrap() {
            (0..SIZE).for_each(|x| board.set_led(Coord::new(x, SIZE - 1).unwrap(), RED));
        } else if self.down_pin.is_high().unwrap() {
            (0..SIZE).for_each(|x| board.set_led(Coord::new(x, 0).unwrap(), YELLOW));
        } else if self.left_pin.is_high().unwrap() {
            (0..SIZE).for_each(|y| board.set_led(Coord::new(0, y).unwrap(), GREEN));
        } else if self.right_pin.is_high().unwrap() {
            (0..SIZE).for_each(|y| board.set_led(Coord::new(SIZE - 1, y).unwrap(), BLUE));
        }

        let a_colour = match self.a_pin.is_low().unwrap() {
            true => WHITE,
            false => BLACK,
        };
        let b_colour = match self.b_pin.is_low().unwrap() {
            true => WHITE,
            false => BLACK,
        };

        board.set_led(Coord::new(1, 2).unwrap(), a_colour);
        board.set_led(Coord::new(2, 1).unwrap(), b_colour);

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
        .sysclk(24.MHz())
        .pclk1(12.MHz())
        .freeze(&mut flash.acr);

    // Set up SPI for WS2812b LEDs
    let (sck, miso, mosi) = (
        gpioa
            .pa5
            .into_af5_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl),
        gpioa
            .pa6
            .into_af5_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl),
        gpiob
            .pb5
            .into_af5_push_pull(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl),
    );
    let spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        ws2812_spi::MODE,
        3.MHz().try_into().unwrap(),
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
        a_pin: gpiob
            .pb6
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr),
        b_pin: gpiob
            .pb7
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr),
    };

    loop {
        board_leds
            .write(board.into_board().into_iter().cloned())
            .unwrap();

        status_led.toggle().unwrap();
        delay.delay_ms(20u16);
    }
}
