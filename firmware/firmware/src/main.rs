#![no_std]
#![no_main]

use core::convert::TryInto;

use panic_halt as _;

use cortex_m::interrupt;
use rtic::cyccnt::U32Ext;
use stm32f3::stm32f303::{Peripherals, EXTI, SPI1};
use stm32f3xx_hal::{
    gpio::{gpioa, gpiob, Alternate, Edge, Input, Output, PushPull},
    prelude::*,
    spi::Spi,
};

use smart_leds::{brightness, SmartLedsWrite};
use ws2812_spi::Ws2812;

use mmxlviii::{
    board::{Direction, IntoBoard},
    game_board::GameBoard,
    score_board::ScoreBoard,
};

const SYSCLK_FREQ: u32 = 48_000_000; // Hz
const UPDATE_PERIOD: u32 = SYSCLK_FREQ / 60; // Cycles
const MOVE_RATE_LIMIT: u32 = SYSCLK_FREQ / 3; // Cycles
const BRIGHTNESS: u8 = 31; // Out of 255

#[rtic::app(
    device = stm32f3xx_hal::pac,
    peripherals = true,
    monotonic = rtic::cyccnt::CYCCNT
)]
const APP: () = {
    struct Resources {
        board: GameBoard,

        exti: EXTI,

        status_led: gpioa::PA3<Output<PushPull>>,

        up_pin: gpioa::PA8<Input>,
        down_pin: gpioa::PA9<Input>,
        left_pin: gpiob::PB1<Input>,
        right_pin: gpiob::PB0<Input>,

        a_pin: gpioa::PA12<Input>,
        b_pin: gpioa::PA11<Input>,

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

        #[init(true)]
        is_move_allowed: bool,
    }

    #[init(spawn = [update])]
    fn init(cx: init::Context) -> init::LateResources {
        // Prepare our core and device peripherals
        let cp: rtic::Peripherals = cx.core;
        let dp: Peripherals = cx.device;

        let mut dcb = cp.DCB;
        let mut dwt = cp.DWT;
        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc.apb2);
        let mut exti = dp.EXTI;
        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
        let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);

        // Initialise monotonic timer for periodic interrupts
        dcb.enable_trace();
        dwt.enable_cycle_counter();

        let clocks = rcc
            .cfgr
            .sysclk(SYSCLK_FREQ.Hz().into())
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

        let mut up_pin = gpioa
            .pa8
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        up_pin.make_interrupt_source(&mut syscfg);
        up_pin.trigger_on_edge(&mut exti, Edge::Rising);
        up_pin.enable_interrupt(&mut exti);
        let mut down_pin = gpioa
            .pa9
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        down_pin.make_interrupt_source(&mut syscfg);
        down_pin.trigger_on_edge(&mut exti, Edge::Rising);
        down_pin.enable_interrupt(&mut exti);
        let mut left_pin = gpiob
            .pb1
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);
        left_pin.make_interrupt_source(&mut syscfg);
        left_pin.trigger_on_edge(&mut exti, Edge::Rising);
        left_pin.enable_interrupt(&mut exti);
        let mut right_pin = gpiob
            .pb0
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);
        right_pin.make_interrupt_source(&mut syscfg);
        right_pin.trigger_on_edge(&mut exti, Edge::Rising);
        right_pin.enable_interrupt(&mut exti);

        let a_pin = gpioa
            .pa12
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        let mut b_pin = gpioa
            .pa11
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        b_pin.make_interrupt_source(&mut syscfg);
        b_pin.trigger_on_edge(&mut exti, Edge::RisingFalling);
        b_pin.enable_interrupt(&mut exti);

        // Create the 2048 board
        let mut board = GameBoard::empty();
        board.set_random();
        board.set_random();

        cx.spawn.update().unwrap();

        init::LateResources {
            board,
            exti,
            status_led,
            up_pin,
            down_pin,
            left_pin,
            right_pin,
            a_pin,
            b_pin,
            board_leds,
        }
    }

    #[task(
        priority = 3,
        binds = EXTI0,
        resources = [exti, right_pin],
        spawn = [make_move]
    )]
    fn exti0(cx: exti0::Context) {
        let pr = cx.resources.exti.pr1.read();
        if pr.pr0().is_pending() {
            cx.resources.right_pin.clear_interrupt_pending_bit();
            let _ = cx.spawn.make_move(Direction::Right);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI1,
        resources = [exti, left_pin],
        spawn = [make_move]
    )]
    fn exti1(cx: exti1::Context) {
        let pr = cx.resources.exti.pr1.read();
        if pr.pr1().is_pending() {
            cx.resources.left_pin.clear_interrupt_pending_bit();
            let _ = cx.spawn.make_move(Direction::Left);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI9_5,
        resources = [exti, down_pin, up_pin],
        spawn = [make_move]
    )]
    fn exti9_5(cx: exti9_5::Context) {
        let pr = cx.resources.exti.pr1.read();
        if pr.pr9().is_pending() {
            cx.resources.down_pin.clear_interrupt_pending_bit();
            let _ = cx.spawn.make_move(Direction::Down);
        } else if pr.pr8().is_pending() {
            cx.resources.up_pin.clear_interrupt_pending_bit();
            let _ = cx.spawn.make_move(Direction::Up);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI15_10,
        resources = [exti, b_pin, status_led],
        spawn = [make_move]
    )]
    fn exti15_10(cx: exti15_10::Context) {
        let pr = cx.resources.exti.pr1.read();
        if pr.pr11().is_pending() {
            cx.resources.b_pin.clear_interrupt_pending_bit();
            cx.resources.status_led.toggle().unwrap();
        }
    }

    #[task(
        priority = 2,
        resources = [board, is_move_allowed],
        schedule = [allow_moves]
    )]
    fn make_move(cx: make_move::Context, direction: Direction) {
        if *cx.resources.is_move_allowed && cx.resources.board.make_move(direction) {
            cx.resources.board.set_random();
            *cx.resources.is_move_allowed = false;
            cx.schedule
                .allow_moves(cx.scheduled + MOVE_RATE_LIMIT.cycles())
                .unwrap();
        }
    }

    #[task(priority = 2, resources = [is_move_allowed])]
    fn allow_moves(cx: allow_moves::Context) {
        *cx.resources.is_move_allowed = true;
    }

    #[task(
        priority = 1,
        resources = [board, a_pin, board_leds],
        schedule = [update]
    )]
    fn update(mut cx: update::Context) {
        let show_score = cx.resources.a_pin.is_low();

        let leds = cx.resources.board.lock(|board| match show_score {
            Ok(true) => ScoreBoard::from_score(board.get_score()).into_board(),
            Ok(false) | Err(_) => board.into_board(),
        });

        // Prevent interrupts occurring during LED write.
        // If this were to occur, the LEDs would display incorrect data
        // manifesting as a momentary flicker.
        interrupt::free(|_| {
            cx.resources
                .board_leds
                .write(brightness(leds.into_iter().cloned(), BRIGHTNESS))
                .unwrap()
        });

        cx.schedule
            .update(cx.scheduled + UPDATE_PERIOD.cycles())
            .unwrap();
    }

    extern "C" {
        fn USB_WKUP();
        fn USB_LP();
        fn USB_HP();
    }
};
