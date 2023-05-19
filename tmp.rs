#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _}; // global logger

// Declare async tasks
#[embassy_executor::task]
async fn blink(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low, OutputDrive::Standard);

    loop {
        // Timekeeping is globally available, no need to mess with hardware timers.
        led.set_high();
        Timer::after(Duration::from_millis(150)).await;
        led.set_low();
        Timer::after(Duration::from_millis(150)).await;
    }
}

#[embassy_executor::task]
async fn button_task(pin: AnyPin) {
    let mut button = Input::new(pin, Pull::Up);

    loop {
        button.wait_for_low().await;
        info!("Button pressed!");
        button.wait_for_high().await;
        info!("Button released!");
    }
}

#[embassy_executor::task]
async fn button_task_2(pin: AnyPin) {
    let mut button = Input::new(pin, Pull::Up);

    loop {
        button.wait_for_low().await;
        info!("Button pressed!");
        button.wait_for_high().await;
        info!("Button released!");
    }
}

// Main is itself an async task as well.
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Spawned tasks run in the background, concurrently.
    spawner.spawn(blink(p.P0_13.degrade())).unwrap();

    // let mut button_1 = Input::new(p.P0_11, Pull::Up);
    let mut button_2 = Input::new(p.P0_12, Pull::Up);
    // let mut button_3 = Input::new(p.P0_24, Pull::Up);
    // let mut button_4 = Input::new(p.P0_25, Pull::Up);
    spawner.spawn(button_task(p.P0_11.degrade())).unwrap();
    spawner.spawn(button_task_2(p.P0_24.degrade())).unwrap();

    loop {
        // Asynchronously wait for GPIO events, allowing other tasks
        // to run, or the core to sleep.
        button_2.wait_for_low().await;
        info!("Button 2 pressed!");
        button_2.wait_for_high().await;
        info!("Button 2 released!");
    }
}
