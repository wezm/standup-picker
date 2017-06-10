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
use f3::stm32f30x::interrupt::{Tim7, Exti0, Exti1510};
use f3::stm32f30x;
use f3::timer::Timer;
use rtfm::{Local, Resource, C1, P0, P1, P2, T0, T1, T2, TMax};

use core::cell::Cell;

// CONFIGURATION
const FREQUENCY: u32 = 5 * 4; // Hz

// STATE
enum Mode {
    Stopped,
    Running(i32),
}

struct State {
    mode: Cell<Mode>
}

impl State {
    const fn new() -> Self {
        State {
            mode: Cell::new(Mode::Stopped)
        }
    }
}

static SHARED: Resource<State, C1> = Resource::new(State::new());

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
    GPIOC: Peripheral {
        register_block: Gpioc,
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
        ceiling: C1,
    },
    DWT: Peripheral {
        register_block: Dwt,
        ceiling: C0,
    },
    SYSCFG: Peripheral {
        register_block: Syscfg,
        ceiling: C0,
    },
});

// INITIALIZATION PHASE
fn init(ref priority: P0, threshold: &TMax) {
    let dwt = DWT.access(priority, threshold);
    dwt.enable_cycle_counter();

    let gpioe = GPIOE.access(priority, threshold);
    let rcc = RCC.access(priority, threshold);
    let tim7 = TIM7.access(priority, threshold);
    let timer = Timer(&tim7);

    // Power up PORTA and PORTC
    rcc.ahbenr.modify(|_, w| w.iopaen().enabled().iopcen().enabled());
    rcc.apb2enr.modify(|_, w| w.syscfgen().enabled());

    // Configure PA0 as input
    let gpioa = GPIOA.access(priority, threshold);
    gpioa
        .moder
        .modify(
            |_, w| {
                w.moder0().input()
            },
        );

    // Configure PC13
    let gpioc = GPIOC.access(priority, threshold);
    gpioc
        .moder
        .modify(
            |_, w| {
                w.moder13().input()
            },
        );

    // Configure Exti0 and Exti13 to be falling edge triggered on PA0, PC13 respectively
    unsafe {
        let exti = EXTI.access(priority, threshold);
        exti.imr1.modify( |_, w| w.mr0().bits(1));
        exti.ftsr1.modify( |_, w| w.tr0().bits(1));

        // Exti13
        // TODO: Combine with above
        exti.imr1.modify( |_, w| w.mr13().bits(1));
        exti.ftsr1.modify( |_, w| w.tr13().bits(1));

        // Set EXTI13 to be triggered by PC13
        let syscfg = SYSCFG.access(priority, threshold);
        syscfg.exticr4.modify(|_, w| w.exti13().bits(0b010));
    }

    let cycle_count = dwt.cyccnt.read();
    hprintln!("cycle count {}", cycle_count);

    led::init(&gpioe, &rcc);
    timer.init(&rcc, FREQUENCY);
}

// IDLE LOOP
fn idle(priority: P0, threshold: T0) -> ! {
    // Sleep
    loop {
        rtfm::wfi();
    }
}

// TASKS
tasks!(stm32f30x, {
    button: Task {
        interrupt: Exti0,
        priority: P1,
        enabled: true,
    },
    button2: Task {
        interrupt: Exti1510,
        priority: P1,
        enabled: true,
    },
    roulette: Task {
        interrupt: Tim7,
        priority: P1,
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

fn toggle_timer(timer: &stm32f30x::Tim7) {
    timer.cr1.modify(|r, w| {
        if r.cen().is_enabled() {
            hprintln!("disable timer");
            w.cen().disabled()
        }
        else {
            hprintln!("enable timer");
            w.cen().enabled()
        }
    });
}

fn button2(task: Exti1510, ref priority: P1, ref threshold: T1) {
    // Clear pending interrupt flag (by writing 1 to it)
    unsafe {
        let exti = EXTI.access(priority, threshold);
        exti.pr1.modify( |_, w| w.pr13().bits(1));
    }

    hprintln!("button pressed in exti13");

    let shared = SHARED.access(priority, threshold);
    let mut mode = shared.mode.get();

    let tim7 = TIM7.access(priority, threshold);
    // let timer = Timer(&tim7);
    // timer.resume();
    toggle_timer(&tim7);
}

fn button(task: Exti0, ref priority: P1, ref threshold: T1) {
    // Clear pending interrupt flag (by writing 1 to it)
    unsafe {
        let exti = EXTI.access(priority, threshold);
        exti.pr1.modify( |_, w| w.pr0().bits(1));
    }

    hprintln!("button pressed in exti0");

    let tim7 = TIM7.access(priority, threshold);
    // let timer = Timer(&tim7);
    // timer.resume();
    toggle_timer(&tim7);
}
