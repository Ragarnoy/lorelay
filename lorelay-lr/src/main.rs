//! This example runs on a STM32WL board, which has a builtin Semtech Sx1262 radio.
//! It demonstrates LORA P2P send functionality.
#![no_std]
#![no_main]
#![macro_use]
#![feature(type_alias_impl_trait, async_fn_in_trait)]
#![allow(incomplete_features)]

mod button_handling;
mod led_handling;
mod lora;

use crate::button_handling::{Button1, Button3, BUTTON_PRESS_SIGNAL, ButtonPress};
use button_handling::Button2;
use defmt::info;
use embassy_executor::Spawner;
use embassy_lora::iv::InterruptHandler;
use embassy_lora::iv::Stm32wlInterfaceVariant;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::{DMA1_CH1, DMA1_CH2};
use embassy_stm32::spi::Spi;
use embassy_time::Delay;
use led_handling::{BlueLed, GreenLed, RedLed};
use lora_phy::mod_params::*;
use lora_phy::sx1261_2::SX1261_2;
use lora_phy::LoRa;
use {defmt_rtt as _, panic_probe as _};
use crate::lora::LoraRadio;
use crate::lora::neighbour::Neighbour;

type SpiLora = Spi<'static, embassy_stm32::peripherals::SUBGHZSPI, DMA1_CH1, DMA1_CH2>;
type Stm32wlIv = Stm32wlInterfaceVariant<Output<'static, AnyPin>>;

bind_interrupts!(struct Irqs{
    SUBGHZ_RADIO => InterruptHandler;
});

pub struct Device {
    uuid: u16,
    pub lora: LoraRadio,
    neighbours: heapless::Vec<Neighbour, 16>,
}


#[embassy_executor::task]
pub async fn state_machine(mut device: Device) {

    loop {
        match BUTTON_PRESS_SIGNAL.wait().await {
            ButtonPress::Button1 => {
                todo!("Mode 1")
            }
            ButtonPress::Button2 => {
                todo!("Mode 2")
            }
            ButtonPress::Button3 => {
                todo!("Mode 3")
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.mux = embassy_stm32::rcc::ClockSrc::HSE32;
    let p = embassy_stm32::init(config);

    let spi = Spi::new_subghz(p.SUBGHZSPI, p.DMA1_CH1, p.DMA1_CH2);

    // Set CTRL1 and CTRL3 for high-power transmission, while CTRL2 acts as an RF switch between tx and rx
    let _ctrl1 = Output::new(p.PC4.degrade(), Level::Low, Speed::High);
    let ctrl2 = Output::new(p.PC5.degrade(), Level::High, Speed::High);
    let _ctrl3 = Output::new(p.PC3.degrade(), Level::High, Speed::High);
    let iv = Stm32wlInterfaceVariant::new(Irqs, None, Some(ctrl2)).unwrap();

    let mut delay = Delay;

    let lora = {
        match LoRa::new(
            SX1261_2::new(BoardType::Stm32wlSx1262, spi, iv),
            false,
            &mut delay,
        )
        .await
        {
            Ok(l) => l,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };
    let device = Device {
        uuid: 1,
        lora: LoraRadio::new(lora).await,
        neighbours: heapless::Vec::new(),
    };

    let blue_led: BlueLed = Output::new(p.PB15, Level::Low, Speed::Low);
    let green_led: GreenLed = Output::new(p.PB9, Level::Low, Speed::Low);
    let red_led: RedLed = Output::new(p.PB11, Level::Low, Speed::Low);

    let button_1: Button1 = Input::new(p.PA0, Pull::Up);
    let button_2: Button2 = Input::new(p.PA1, Pull::Up);
    let button_3: Button3 = Input::new(p.PC6, Pull::Up);

    let exti_1 = ExtiInput::new(button_1, p.EXTI0);
    let exti_2 = ExtiInput::new(button_2, p.EXTI1);
    let exti_3 = ExtiInput::new(button_3, p.EXTI6);

    spawner
        .spawn(led_handling::blue_led_handler(blue_led))
        .expect("spawner failed");
    spawner
        .spawn(led_handling::green_led_handler(green_led))
        .expect("spawner failed");
    spawner
        .spawn(led_handling::red_led_handler(red_led))
        .expect("spawner failed");
    spawner
        .spawn(button_handling::button_1_press(exti_1))
        .expect("spawner failed");
    spawner
        .spawn(button_handling::button_2_press(exti_2))
        .expect("spawner failed");
    spawner
        .spawn(button_handling::button_3_press(exti_3))
        .expect("spawner failed");
    spawner
        .spawn(lora::idle_task(lora))
        .expect("spawner failed");
}
