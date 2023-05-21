use defmt::debug;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::peripherals::{PA0, PA1, PC6};
use embassy_stm32::gpio::Input;

pub type Button1 = Input<'static, PA0>;
pub type Button2 = Input<'static, PA1>;
pub type Button3 = Input<'static, PC6>;

type ExtiButton1 = ExtiInput<'static, PA0>;
type ExtiButton2 = ExtiInput<'static, PA1>;
type ExtiButton3 = ExtiInput<'static, PC6>;

static BUTTON_PRESS_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn button_1_press(mut button_exti: ExtiButton1) {
    loop {
        button_exti.wait_for_rising_edge().await;
        debug!("Button 1 pressed");
        BUTTON_PRESS_SIGNAL.signal(());
    }
}

#[embassy_executor::task]
pub async fn button_2_press(mut button_exti: ExtiButton2) {
    loop {
        button_exti.wait_for_rising_edge().await;
        debug!("Button 2 pressed");
        BUTTON_PRESS_SIGNAL.signal(());
    }
}

#[embassy_executor::task]
pub async fn button_3_press(mut button_exti: ExtiButton3) {
    loop {
        button_exti.wait_for_rising_edge().await;
        debug!("Button 3 pressed");
        BUTTON_PRESS_SIGNAL.signal(());
    }
}
