use defmt::info;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Input;
use embassy_stm32::peripherals::{PA0, PA1, PC6};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub type Button1 = Input<'static, PA0>;
pub type Button2 = Input<'static, PA1>;
pub type Button3 = Input<'static, PC6>;

type ExtiButton1 = ExtiInput<'static, PA0>;
type ExtiButton2 = ExtiInput<'static, PA1>;
type ExtiButton3 = ExtiInput<'static, PC6>;

pub static BUTTON_PRESS_SIGNAL: Signal<CriticalSectionRawMutex, ButtonPress> = Signal::new();

pub enum ButtonPress {
    Button1,
    Button2,
    Button3,
}

#[embassy_executor::task]
pub async fn button_1_press(mut button_exti: ExtiButton1) {
    loop {
        button_exti.wait_for_rising_edge().await;
        info!("Button 1 pressed");
        BUTTON_PRESS_SIGNAL.signal(ButtonPress::Button1);
    }
}

#[embassy_executor::task]
pub async fn button_2_press(mut button_exti: ExtiButton2) {
    loop {
        button_exti.wait_for_rising_edge().await;
        info!("Button 2 pressed");
        BUTTON_PRESS_SIGNAL.signal(ButtonPress::Button2);
    }
}

#[embassy_executor::task]
pub async fn button_3_press(mut button_exti: ExtiButton3) {
    loop {
        button_exti.wait_for_rising_edge().await;
        info!("Button 3 pressed");
        BUTTON_PRESS_SIGNAL.signal(ButtonPress::Button3);
    }
}
