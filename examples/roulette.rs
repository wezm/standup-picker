#![feature(const_fn)]
#![feature(used)]
#![no_std]

// version = "0.2.2", default-features = false
extern crate cast;

#[macro_use]
extern crate cortex_m;

// version = "0.2.0"
extern crate cortex_m_rt;

// version = "0.1.0"
#[macro_use]
extern crate cortex_m_rtfm as rtfm;

extern crate f3;

use cast::{u8, usize};
use f3::led::{self, LEDS};
use f3::stm32f30x::interrupt::{Tim7, Exti0};
use f3::stm32f30x;
use f3::timer::Timer;
use rtfm::{Local, P0, P1, P2, T0, T1, T2, TMax};

// CONFIGURATION
const FREQUENCY: u32 = 4; // Hz

// RESOURCES
peripherals!(stm32f30x, {
    GPIOE: Peripheral {
        register_block: Gpioe,
        ceiling: C0,
    },
    GPIOA: Peripheral {
        register_block: Gpioa,
        ceiling: C0,
    },
    RCC: Peripheral {
        register_block: Rcc,
        ceiling: C0,
    },
    TIM7: Peripheral {
        register_block: Tim7,
        ceiling: C1,
    },
    EXTI: Peripheral {
        register_block: Exti,
        ceiling: C2,
    },
    // SYSCFG: Peripheral {
    //     register_block: Syscfg,
    //     ceiling: C0,
    // },
});

// INITIALIZATION PHASE
fn init(ref priority: P0, threshold: &TMax) {
    let gpioe = GPIOE.access(priority, threshold);
    let rcc = RCC.access(priority, threshold);
    let tim7 = TIM7.access(priority, threshold);
    let timer = Timer(&tim7);

    // Power up PORTA
    rcc.ahbenr.modify(|_, w| w.iopaen().enabled());

    // Configure pins 8-15 as outputs
    let gpioa = GPIOA.access(priority, threshold);
    gpioa
        .moder
        .modify(
            |_, w| {
                w.moder0().input()
            },
        );

    // Configure Exti0 to be falling edge triggered on PA0

    unsafe {
        let exti = EXTI.access(priority, threshold);
        exti.imr1.modify( |_, w| w.mr0().bits(1));
        exti.ftsr1.modify( |_, w| w.tr0().bits(1));

        // let syscfg = SYSCFG.access(priority, threshold);
        // syscfg.exticr1.modify(|_, w| w.exti0().bits(
    }

    led::init(&gpioe, &rcc);
    timer.init(&rcc, FREQUENCY);
    timer.resume();
}

// IDLE LOOP
fn idle(priority: P0, threshold: T0) -> ! {
    // Sleep
    loop {
        // rtfm::wfi();

        // Sample button
        let gpioa = GPIOA.access(&priority, &threshold);
        let button = gpioa.idr.read().idr0().bits();
        if button != 0 {
            // Button is pressed
            hprintln!("button pressed");
        }
    }
}

// TASKS
tasks!(stm32f30x, {
    roulette: Task {
        interrupt: Tim7,
        priority: P1,
        enabled: true,
    },
    button: Task {
        interrupt: Exti0,
        priority: P2,
        enabled: true,
    },
});

fn roulette(mut task: Tim7, ref priority: P1, ref threshold: T1) {
    static STATE: Local<u8, Tim7> = Local::new(0);

    let tim7 = TIM7.access(priority, threshold);
    let timer = Timer(&tim7);

    if timer.clear_update_flag().is_ok() {
        let state = STATE.borrow_mut(&mut task);

        let curr = *state;
        let next = (curr + 1) % u8(LEDS.len()).unwrap();

        LEDS[usize(curr)].off();
        LEDS[usize(next)].on();

        *state = next;
    } else {
        // Only reachable through `rtfm::request(roulette)`
        #[cfg(debug_assertion)]
        unreachable!()
    }
}

fn button(task: Exti0, ref priority: P2, ref threshold: T2) {
    // Clear pending interrupt flag
    unsafe {
        let exti = EXTI.access(priority, threshold);
        exti.pr1.modify( |_, w| w.pr0().bits(0));
    }

    hprintln!("button pressed in exti0");
}
