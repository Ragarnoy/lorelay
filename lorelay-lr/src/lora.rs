use defmt::{error, info};
use lora_phy::mod_params::{Bandwidth, CodingRate, SpreadingFactor};
use embassy_time::{Duration, Timer};
use heapless::String;
use lora_phy::LoRa;
use lora_phy::sx1261_2::SX1261_2;
use crate::led_handling::{LED_BLUE_BLINK_SIGNAL, LED_GREEN_BLINK_SIGNAL, LED_RED_BLINK_SIGNAL};
use crate::{SpiLora, Stm32wlIv};

type LoraRadio = LoRa<SX1261_2<SpiLora, Stm32wlIv>>;

const LORA_FREQUENCY_IN_HZ: u32 = 433_220_000;


#[embassy_executor::task]
pub async fn rxtx_lora_messages(mut lora: LoraRadio) {
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
