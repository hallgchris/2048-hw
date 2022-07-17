#![no_std]
#![no_main]

use panic_halt as _;

// use rtic::cyccnt::U32Ext;

#[rtic::app(
    device = stm32f3xx_hal::pac,
    peripherals = true,
    // monotonic = rtic::cyccnt::CYCCNT
    dispatchers = [USB_HP, USB_LP, USB_WKUP]
)]
mod app {
    use cortex_m::interrupt;
    use rtt_target::{rprintln, rtt_init_print};

    use stm32f3xx_hal::{
        gpio::{self, gpioa, gpiob, Alternate, Edge, Input, Output, PushPull, AF5},
        pac::{EXTI, SPI1},
        prelude::*,
        spi::{MosiPin, Spi},
    };

    use systick_monotonic::{ExtU64, Systick};

    use smart_leds::{brightness, SmartLedsWrite};
    use ws2812_spi::Ws2812;

    use mmxlviii::{
        board::{Direction, IntoBoard},
        game_board::GameBoard,
        score_board::ScoreBoard,
    };

    const SYSCLK_FREQ: u32 = 48_000_000; // Hz
    const UPDATE_PERIOD: u64 = 1000 / 60; // ms
    const MOVE_RATE_LIMIT: u64 = 1000 / 3; // ms
    const BRIGHTNESS: u8 = 31; // Out of 255

    #[shared]
    struct Shared {
        board: GameBoard,

        #[lock_free]
        exti: EXTI,

        is_move_allowed: bool,
    }

    #[local]
    struct Local {
        status_led: gpioa::PA3<Output<PushPull>>,
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

        up_pin: gpioa::PA8<Input>,
        down_pin: gpioa::PA9<Input>,
        left_pin: gpiob::PB1<Input>,
        right_pin: gpiob::PB0<Input>,

        a_pin: gpioa::PA12<Input>,
        b_pin: gpioa::PA11<Input>,
    }

    #[monotonic(binds = SysTick, default = true)]
    type Tonic = Systick<1000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("2048-hw v0.1.0");

        // Prepare our core and device peripherals
        let cp = cx.core;
        let dp = cx.device;

        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc.apb2);
        let mut exti = dp.EXTI;
        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
        let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);

        // Initialise monotonic timer for periodic interrupts
        let mono = Systick::new(cp.SYST, SYSCLK_FREQ);

        let clocks = rcc
            .cfgr
            .sysclk(SYSCLK_FREQ.Hz().into())
            .freeze(&mut flash.acr);

        // Set up SPI for WS2812b LEDs
        let sck = gpioa
            .pa5
            .into_af_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
        let miso =
            gpioa
                .pa6
                .into_af_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
        let mosi =
            gpiob
                .pb5
                .into_af_push_pull(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);
        let spi = Spi::new(
            dp.SPI1,
            (sck, miso, mosi),
            3.MHz(),
            // ws2812_spi::MODE,
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
        syscfg.select_exti_interrupt_source(&up_pin);
        up_pin.trigger_on_edge(&mut exti, Edge::Rising);
        up_pin.enable_interrupt(&mut exti);
        let mut down_pin = gpioa
            .pa9
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        syscfg.select_exti_interrupt_source(&down_pin);
        down_pin.trigger_on_edge(&mut exti, Edge::Rising);
        down_pin.enable_interrupt(&mut exti);
        let mut left_pin = gpiob
            .pb1
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);
        syscfg.select_exti_interrupt_source(&left_pin);
        left_pin.trigger_on_edge(&mut exti, Edge::Rising);
        left_pin.enable_interrupt(&mut exti);
        let mut right_pin = gpiob
            .pb0
            .into_pull_up_input(&mut gpiob.moder, &mut gpiob.pupdr);
        syscfg.select_exti_interrupt_source(&right_pin);
        right_pin.trigger_on_edge(&mut exti, Edge::Rising);
        right_pin.enable_interrupt(&mut exti);

        let a_pin = gpioa
            .pa12
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        let mut b_pin = gpioa
            .pa11
            .into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        syscfg.select_exti_interrupt_source(&b_pin);
        b_pin.trigger_on_edge(&mut exti, Edge::RisingFalling);
        b_pin.enable_interrupt(&mut exti);

        // Create the 2048 board
        let mut board = GameBoard::empty();
        board.set_random();
        board.set_random();

        update::spawn().unwrap();

        let shared = Shared {
            board,
            exti,
            is_move_allowed: true,
        };
        let local = Local {
            status_led,
            board_leds,
            up_pin,
            down_pin,
            left_pin,
            right_pin,
            a_pin,
            b_pin,
        };

        (shared, local, init::Monotonics(mono))
    }

    #[task(
        priority = 3,
        binds = EXTI0,
        shared = [exti],
        local = [right_pin]
    )]
    fn exti0(cx: exti0::Context) {
        let pr = cx.shared.exti.pr1.read();
        if pr.pr0().is_pending() {
            cx.local.right_pin.clear_interrupt();
            let _ = make_move::spawn(Direction::Right);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI1,
        shared = [exti],
        local=[left_pin]
    )]
    fn exti1(cx: exti1::Context) {
        let pr = cx.shared.exti.pr1.read();
        if pr.pr1().is_pending() {
            cx.local.left_pin.clear_interrupt();
            let _ = make_move::spawn(Direction::Left);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI9_5,
        shared = [exti],
        local = [ down_pin, up_pin]
    )]
    fn exti9_5(cx: exti9_5::Context) {
        let pr = cx.shared.exti.pr1.read();
        if pr.pr9().is_pending() {
            cx.local.down_pin.clear_interrupt();
            let _ = make_move::spawn(Direction::Down);
        } else if pr.pr8().is_pending() {
            cx.local.up_pin.clear_interrupt();
            let _ = make_move::spawn(Direction::Up);
        }
    }

    #[task(
        priority = 3,
        binds = EXTI15_10,
        shared = [exti ],
        local = [b_pin, status_led]
    )]
    fn exti15_10(cx: exti15_10::Context) {
        let pr = cx.shared.exti.pr1.read();
        if pr.pr11().is_pending() {
            cx.local.b_pin.clear_interrupt();
            cx.local.status_led.toggle().unwrap();
        }
    }

    #[task(
        priority = 2,
        shared = [board, is_move_allowed],
    )]
    fn make_move(cx: make_move::Context, direction: Direction) {
        let board = cx.shared.board;
        let is_move_allowed = cx.shared.is_move_allowed;

        (board, is_move_allowed).lock(|board, is_move_allowed| {
            if *is_move_allowed && board.make_move(direction) {
                board.set_random();
                *is_move_allowed = false;
                allow_moves::spawn_after(MOVE_RATE_LIMIT.millis()).unwrap();
            }
        })
    }

    #[task(priority = 2, shared = [is_move_allowed])]
    fn allow_moves(mut cx: allow_moves::Context) {
        cx.shared
            .is_move_allowed
            .lock(|is_move_allowed| *is_move_allowed = true);
    }

    #[task(
        priority = 1,
        shared = [board],
        local = [a_pin, board_leds]
    )]
    fn update(mut cx: update::Context) {
        let show_score = cx.local.a_pin.is_low();

        let leds = cx.shared.board.lock(|board| match show_score {
            Ok(true) => ScoreBoard::from_score(board.get_score()).into_board(),
            Ok(false) | Err(_) => board.into_board(),
        });

        // Prevent interrupts occurring during LED write.
        // If this were to occur, the LEDs would display incorrect data
        // manifesting as a momentary flicker.
        interrupt::free(|_| {
            cx.local
                .board_leds
                .write(brightness(leds.into_iter().cloned(), BRIGHTNESS))
                .unwrap()
        });

        update::spawn_after(UPDATE_PERIOD.millis()).unwrap();
    }
}
