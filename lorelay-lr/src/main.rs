//! This example runs on a STM32WL board, which has a builtin Semtech Sx1262 radio.
//! It demonstrates LORA P2P send functionality.
#![no_std]
#![no_main]
#![macro_use]
#![feature(type_alias_impl_trait, async_fn_in_trait)]
#![allow(incomplete_features)]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_lora::iv::Stm32wlInterfaceVariant;
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::spi::Spi;
use embassy_stm32::{interrupt, into_ref, Peripheral};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::peripherals::{DMA1_CH1, DMA1_CH2, PA1, PB11, PB15, PB9};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Duration, Timer};
use heapless::{String};
use lora_phy::mod_params::*;
use lora_phy::sx1261_2::SX1261_2;
use lora_phy::LoRa;
use {defmt_rtt as _, panic_probe as _};

type BlueLed = Output<'static, PB15>;
type GreenLed = Output<'static, PB9>;
type RedLed = Output<'static, PB11>;

type Button1 = Input<'static, PA1>;
// type Button2 = Input<'static, PA1>;
// type Button3 = Input<'static, PC6>;

type SpiLora = Spi<'static, embassy_stm32::peripherals::SUBGHZSPI, DMA1_CH1, DMA1_CH2>;
type Stm32wlIv = Stm32wlInterfaceVariant<'static, Output<'static, AnyPin>>;
type LoraRadio = LoRa<SX1261_2<SpiLora, Stm32wlIv>>;

const LORA_FREQUENCY_IN_HZ: u32 = 433_220_000;

static LED_BLUE_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static LED_RED_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static LED_GREEN_BLINK_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

static BUTTON_PRESS_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
async fn blue_led_handler(mut led: BlueLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_BLUE_BLINK_SIGNAL.wait().await;

        info!("Blinking LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}

#[embassy_executor::task]
async fn green_led_handler(mut led: GreenLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_GREEN_BLINK_SIGNAL.wait().await;

        info!("Blinking LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}

#[embassy_executor::task]
async fn red_led_handler(mut led: RedLed) {
    loop {
        // Wait for the signal to blink the LED
        LED_RED_BLINK_SIGNAL.wait().await;

        info!("Blinking LED");
        // Blink the LED
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
    }
}

#[embassy_executor::task]
async fn button_press(mut button_exti: ExtiInput<'static, PA1>) {
    loop {
        button_exti.wait_for_rising_edge().await;
        info!("Button pressed");
        BUTTON_PRESS_SIGNAL.signal(());
    }
}


#[embassy_executor::task]
async fn rxtx_lora_messages(mut lora: LoraRadio) {
    let mut rx_buffer = [0u8; 100];

    info!("Starting RX/TX");
    let mdltn_params = {
        match lora.create_modulation_params(
            SpreadingFactor::_10,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                error!("Radio error = {}", err);
                return;
            }
        }
    };

    let mut tx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                return;
            }
        }
    };
    Timer::after(Duration::from_secs(5)).await;

    match lora.prepare_for_tx(&mdltn_params, 20, false).await {
        Ok(()) => {
            info!("Radio prepared for TX");
            LED_GREEN_BLINK_SIGNAL.signal(());
            // Timer::after(Duration::from_secs(4)).await;
        }
        Err(err) => {
            error!("Radio error = {}", err);
            return;
        }
    };
    info!("Starting First TX");
    let buff: [u8; 8] = [b'h', b'e', b'l', b'l', b'o', b' ', b'0', b'\0'];
    match lora.tx(&mdltn_params, &mut tx_pkt_params,  &buff, 0xffffff).await {
        Ok(()) => {
            info!("FIRST TX DONE");
            LED_RED_BLINK_SIGNAL.signal(());
        }
        Err(err) => {
            error!("Radio error = {}", err);
            return;
        }
    };

    info!("Preparing RX");

    let rx_pkt_params = {
        match lora.create_rx_packet_params(4, false, rx_buffer.len() as u8, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                return;
            }
        }
    };

    match lora
        .prepare_for_rx(&mdltn_params, &rx_pkt_params, None, true, false, 0, 0x00ffffffu32)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            info!("Radio error = {}", err);
            return;
        }
    };

    info!("Starting RXTX loop");
    loop {
        rx_buffer.fill(0);

        info!("Starting RXTX loop cycle");
        match lora.rx(&rx_pkt_params, &mut rx_buffer).await {
            Err(err) => info!("rx unsuccessful = {}", err),
            Ok((received_len, _rx_pkt_status)) => {
                if received_len <= 20 && rx_buffer.starts_with("hello".as_bytes()) {

                    info!("Received message: {}", core::str::from_utf8(&rx_buffer).unwrap());
                    // Signal the LED blink task to blink the LED
                    LED_GREEN_BLINK_SIGNAL.signal(());
                    Timer::after(Duration::from_secs(1)).await;

                    match lora.prepare_for_tx(&mdltn_params, 20, false).await {
                        Ok(()) => {}
                        Err(err) => {
                            info!("Radio error = {}", err);
                            return;
                        }
                    };

                    info!("Sending message");
                    let (hello, number_str) = rx_buffer.split_at(rx_buffer.iter().position(|&c| c == b' ').unwrap() + 1);

                    // Parse the number, increment it, and convert it back to a string
                    let number = core::ffi::CStr::from_bytes_until_nul(number_str).unwrap().to_str().unwrap().parse::<u32>().unwrap() + 1;
                    let mut new_number_str: String<2> = String::new();
                    if number >= 100 {
                        unreachable!("The number should never be 100 or more");
                    }
                    if number >= 10 {
                        new_number_str.push(char::from_digit(number / 10, 10).unwrap()).unwrap();
                    }
                    new_number_str.push(char::from_digit(number % 10, 10).unwrap()).unwrap();
                    // Create a new array for the new message
                    let mut new_message = [0u8; 100];
                    // Copy the "Hello " part into the new array
                    new_message[..hello.len()].copy_from_slice(hello);
                    // Copy the new number into the new array
                    new_message[hello.len()..hello.len() + new_number_str.len()].copy_from_slice(new_number_str.as_bytes());
                    // The length of the new message
                    let new_message_len = hello.len() + new_number_str.len();

                    Timer::after(Duration::from_secs(1)).await;
                    info!("Sending message: {}", core::str::from_utf8(&new_message[..new_message_len]).unwrap());
                    match lora.tx(&mdltn_params, &mut tx_pkt_params,  &new_message[..new_message_len], 0xffffff).await {
                        Ok(()) => {
                            LED_BLUE_BLINK_SIGNAL.signal(());
                            info!("TX DONE");
                        }
                        Err(err) => {
                            info!("Radio error = {}", err);
                            return;
                        }
                    };
                } else {
                    info!("rx unknown packet");
                }
            }
        }
        match lora
            .prepare_for_rx(&mdltn_params, &rx_pkt_params, None, true, false, 0, 0x00ffffffu32)
            .await
        {
            Ok(()) => {}
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.mux = embassy_stm32::rcc::ClockSrc::HSE32;
    let p = embassy_stm32::init(config);

    let spi = Spi::new_subghz(p.SUBGHZSPI, p.DMA1_CH1, p.DMA1_CH2);

    let irq = interrupt::take!(SUBGHZ_RADIO);
    into_ref!(irq);
    // Set CTRL1 and CTRL3 for high-power transmission, while CTRL2 acts as an RF switch between tx and rx
    let _ctrl1 = Output::new(p.PC4.degrade(), Level::Low, Speed::High);
    let ctrl2 = Output::new(p.PC5.degrade(), Level::High, Speed::High);
    let _ctrl3 = Output::new(p.PC3.degrade(), Level::High, Speed::High);
    let iv = Stm32wlInterfaceVariant::new(irq, None, Some(ctrl2)).unwrap();

    let mut delay = Delay;
    info!("Starting LoRa P3P send example");

    let lora = {
        match LoRa::new(SX1261_2::new(BoardType::Stm32wlSx1262, spi, iv), false, &mut delay).await {
            Ok(l) => l,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };

    let blue_led: BlueLed = Output::new(p.PB15, Level::Low, Speed::Low);
    let green_led: GreenLed = Output::new(p.PB9, Level::Low, Speed::Low);
    let red_led: RedLed = Output::new(p.PB11, Level::Low, Speed::Low);
    let button: Button1 = Input::new(p.PA1, Pull::Up);
    let exti = ExtiInput::new(button, p.EXTI1);

    spawner.spawn(blue_led_handler(blue_led)).expect("spawner failed");
    spawner.spawn(green_led_handler(green_led)).expect("spawner failed");
    spawner.spawn(red_led_handler(red_led)).expect("spawner failed");
    spawner.spawn(button_press(exti)).expect("spawner failed");
    spawner.spawn(rxtx_lora_messages(lora)).expect("spawner failed");
}
