use defmt::debug;
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::{PB11, PB15, PB9};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub type BlueLed = Output<'static, PB15>;
pub type GreenLed = Output<'static, PB9>;
pub type RedLed = Output<'static, PB11>;

pub static LED_BLUE_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static LED_RED_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static LED_GREEN_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn blue_led_handler(mut led: BlueLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_BLUE_BLINK_SIGNAL.wait().await;

        debug!("Blinking blue LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}

#[embassy_executor::task]
pub async fn green_led_handler(mut led: GreenLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_GREEN_BLINK_SIGNAL.wait().await;

        debug!("Blinking green LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}

#[embassy_executor::task]
pub async fn red_led_handler(mut led: RedLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_RED_BLINK_SIGNAL.wait().await;

        debug!("Blinking red LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}
