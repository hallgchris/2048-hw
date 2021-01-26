#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;

use stm32f3xx_hal::{delay, pac, prelude::*};

use mmxlviii::board::Board;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut status_led = gpioa
        .pa3
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    let mut delay = delay::Delay::new(cp.SYST, clocks);

    let mut board = Board::empty();
    for _ in 0..3 {
        board.set_random();
    }

    loop {
        status_led.toggle().unwrap();
        delay.delay_ms(500u16);
    }
}
