#![no_std]
#![no_main]

use core::convert::TryInto;

use panic_halt as _;

use stm32f3::stm32f303::{Peripherals, SPI1};
use stm32f3xx_hal::{
    delay,
    gpio::{gpioa, gpiob, Alternate, Edge, Input, Output, PushPull},
    prelude::*,
    spi::Spi,
};

use smart_leds::{brightness, gamma, SmartLedsWrite};
use ws2812_spi::Ws2812;

use mmxlviii::{
    board::{Direction, IntoBoard},
    game_board::GameBoard,
    score_board::ScoreBoard,
};

#[rtic::app(device = stm32f3xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        board: GameBoard,

        status_led: gpioa::PA3<Output<PushPull>>,
        up_pin: gpioa::PA11<Input>,
        down_pin: gpioa::PA10<Input>,
        left_pin: gpioa::PA8<Input>,
        right_pin: gpioa::PA9<Input>,
        a_pin: gpiob::PB6<Input>,
        b_pin: gpiob::PB7<Input>,

        board_leds: Ws2812<
            Spi<
                SPI1,
                (
                    gpioa::PA5<Alternate<PushPull, 5>>,
                    gpioa::PA6<Alternate<PushPull, 5>>,
                    gpiob::PB5<Alternate<PushPull, 5>>,
                ),
            >,
        >,

        delay: delay::Delay,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        // Prepare our core and device peripherals
        let cp: rtic::export::Peripherals = cx.core;
        let dp: Peripherals = cx.device;

        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc.apb2);
        let mut exti = dp.EXTI;
        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
        let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);

        let clocks = rcc
            .cfgr
            .sysclk(48.MHz())
            .pclk1(24.MHz())
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
        let board_leds = Ws2812::new(spi);

        // Prepare other useful bits
        let status_led = gpioa
            .pa3
            .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
        let delay = delay::Delay::new(cp.SYST, clocks);

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
        let mut b_pin = gpiob
            .pb7
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);
        b_pin.make_interrupt_source(&mut syscfg);
        b_pin.trigger_on_edge(&mut exti, Edge::RisingFalling);
        b_pin.enable_interrupt(&mut exti);

        // Create the 2048 board
        let mut board = GameBoard::empty();
        board.set_random();

        init::LateResources {
            board,
            status_led,
            up_pin,
            down_pin,
            left_pin,
            right_pin,
            a_pin,
            b_pin,
            board_leds,
            delay,
        }
    }

    #[task(binds = EXTI9_5, resources = [status_led, b_pin])]
    fn exti9_5(cx: exti9_5::Context) {
        cx.resources.b_pin.clear_interrupt_pending_bit();
        cx.resources.status_led.toggle().unwrap();
    }

    #[idle(resources=[board, up_pin, down_pin, left_pin, right_pin, a_pin, board_leds, delay])]
    fn idle(cx: idle::Context) -> ! {
        let brightness_level = 31;
        let mut debouncer = false;

        loop {
            let mut direction: Option<Direction> = None;
            if cx.resources.up_pin.is_high().unwrap() {
                direction = Some(Direction::Up);
            } else if cx.resources.down_pin.is_high().unwrap() {
                direction = Some(Direction::Down);
            } else if cx.resources.left_pin.is_high().unwrap() {
                direction = Some(Direction::Left);
            } else if cx.resources.right_pin.is_high().unwrap() {
                direction = Some(Direction::Right);
            }

            debouncer = match direction {
                Some(chosen_direction) => {
                    if !debouncer && cx.resources.board.make_move(chosen_direction) {
                        cx.resources.board.set_random();
                    }
                    true
                }
                None => false,
            };

            let leds = match cx.resources.a_pin.is_low() {
                Ok(true) => ScoreBoard::from_score(cx.resources.board.get_score()).into_board(),
                Ok(false) | Err(_) => cx.resources.board.into_board(),
            };

            // TODO: Figure out the typing so the below line is cleaner
            cx.resources
                .board_leds
                .write(brightness(
                    gamma(leds.into_iter().cloned()),
                    brightness_level,
                ))
                .unwrap();

            cx.resources.delay.delay_ms(10u16);
        }
    }
};
