pub mod message;
pub mod neighbour;

use crate::led_handling::{LED_BLUE_BLINK_SIGNAL, LED_GREEN_BLINK_SIGNAL, LED_RED_BLINK_SIGNAL};
use crate::{SpiLora, Stm32wlIv};
use core::fmt::Write;
use defmt::{debug, error, info};
use embassy_time::{Duration, Timer};
use heapless::String;
use lora_phy::mod_params::{
    Bandwidth, CodingRate, ModulationParams, PacketParams, RadioError, SpreadingFactor,
};
use lora_phy::sx1261_2::SX1261_2;
use lora_phy::LoRa;

type Lora = LoRa<SX1261_2<SpiLora, Stm32wlIv>>;

const LORA_FREQUENCY_IN_HZ: u32 = 433_220_000;

const RX_BUF_SIZE: usize = 100;

const OUTPUT_POWER: i32 = 20;

const FIRST_MESSAGE: [u8; 8] = [b'h', b'e', b'l', b'l', b'o', b' ', b'0', b'\0'];

pub struct LoraRadio {
    lora: Lora,
    mdltn_params: ModulationParams,
    rx_pkt_params: PacketParams,
    tx_pkt_params: PacketParams,
}

impl LoraRadio {
    pub async fn new(mut lora: Lora) -> Self {
        let mdltn_params = modulation_params(&mut lora).expect("Failed to create modulation params");

        let mut tx_pkt_params =
            create_tx_packet(&mut lora, &mdltn_params).expect("Failed to create TX packet params");

        let rx_pkt_params =
            create_rx_packet(&mut lora, &mdltn_params).expect("Failed to create RX packet params");

        LoraRadio {
            lora,
            mdltn_params,
            tx_pkt_params,
            rx_pkt_params
        }
    }

    pub async fn send(&mut self, message: &[u8]) -> Result<(), RadioError> {
        match self.lora.prepare_for_tx(&self.mdltn_params, OUTPUT_POWER, false).await {
            Ok(()) => {
                debug!("Radio prepared for TX");
            }
            Err(err) => {
                error!("Radio error = {}", err);
                return Err(err);
            }
        };

        Timer::after(Duration::from_secs(1)).await;

        match self.lora.tx(&self.mdltn_params, &mut self.tx_pkt_params, message, 0xffffff).await {
            Ok(()) => {
                info!("Sending message: {}", core::str::from_utf8(message).unwrap());
                LED_BLUE_BLINK_SIGNAL.signal(());
            }
            Err(err) => {
                error!("Radio error = {}", err);
                return Err(err);
            }
        };
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<[u8; RX_BUF_SIZE], RadioError>
    {
        let mut rx_buffer: [u8; RX_BUF_SIZE] = [0; RX_BUF_SIZE];

        match self.lora.rx(&self.rx_pkt_params, &mut rx_buffer).await {
            Err(err) => {
                info!("rx unsuccessful = {}", err);
                Err(err)
            }
            Ok((received_len, _rx_pkt_status)) => {
                Ok(rx_buffer)
            }
        }
    }
}

#[embassy_executor::task]
pub async fn idle_task(mut lora: Lora) {
    let mut rx_buffer = [0u8; RX_BUF_SIZE];

    info!("Starting RX/TX");
    let mdltn_params = modulation_params(&mut lora).expect("Failed to create modulation params");

    let mut tx_pkt_params =
        create_tx_packet(&mut lora, &mdltn_params).expect("Failed to create TX packet params");

    let rx_pkt_params =
        create_rx_packet(&mut lora, &mdltn_params).expect("Failed to create RX packet params");

    LED_RED_BLINK_SIGNAL.signal(());
    Timer::after(Duration::from_secs(5)).await;

    if let Err(e) = prepare_tx(&mut lora, &mdltn_params).await {
        error!("Failed to prepare TX: {}", e);
        return;
    }

    info!("Starting First TX");
    if let Err(e) = tx_buffer(&mut lora, &mdltn_params, &mut tx_pkt_params, &FIRST_MESSAGE).await {
        error!("Failed to TX: {}", e);
        return;
    }

    info!("Preparing RX");

    if let Err(e) = prepare_rx(&mut lora, &mdltn_params, &rx_pkt_params).await {
        error!("Failed to prepare RX: {}", e);
        return;
    }

    info!("Starting RXTX loop");

    loop {
        rx_buffer.fill(0);

        info!("Starting RXTX loop cycle");
        match lora.rx(&rx_pkt_params, &mut rx_buffer).await {
            Err(err) => info!("rx unsuccessful = {}", err),
            Ok((received_len, _rx_pkt_status)) => {
                if received_len <= 12 && rx_buffer.starts_with("hello".as_bytes()) {
                    info!(
                        "Received message: {}",
                        core::str::from_utf8(&rx_buffer).unwrap()
                    );
                    // Green led for message reception
                    LED_GREEN_BLINK_SIGNAL.signal(());
                    Timer::after(Duration::from_secs(1)).await;

                    if let Err(e) = prepare_tx(&mut lora, &mdltn_params).await {
                        error!("Failed to prepare TX: {}", e);
                        return;
                    }

                    let new_message = create_message(rx_buffer);
                    Timer::after(Duration::from_secs(1)).await;

                    if let Err(e) = tx_buffer(
                        &mut lora,
                        &mdltn_params,
                        &mut tx_pkt_params,
                        new_message.as_bytes(),
                    )
                    .await
                    {
                        error!("Failed to TX: {}", e);
                        return;
                    }
                } else {
                    info!("rx unknown packet");
                }
            }
        }
        if let Err(e) = prepare_rx(&mut lora, &mdltn_params, &rx_pkt_params).await {
            error!("Failed to prepare RX: {}", e);
            return;
        }
    }
}

fn create_message(rx_buffer: [u8; 100]) -> String<20> {
    let msg = core::str::from_utf8(&rx_buffer).unwrap();
    let (hello, number_str) = core::ffi::CStr::from_bytes_until_nul(msg.as_bytes())
        .unwrap()
        .to_str()
        .unwrap()
        .split_at(msg.find(' ').unwrap());
    let number: u32 = number_str.trim().parse().unwrap();
    let mut new_message: String<20> = String::new();
    write!(&mut new_message, "{} {}", hello, number + 1).expect("Failed to write to string");
    new_message
}

async fn prepare_rx(
    lora: &mut Lora,
    mdltn_params: &ModulationParams,
    rx_pkt_params: &PacketParams,
) -> Result<(), RadioError> {
    match lora
        .prepare_for_rx(
            mdltn_params,
            rx_pkt_params,
            None,
            true,
            false,
            0,
            0x00ffffffu32,
        )
        .await
    {
        Ok(()) => {}
        Err(err) => {
            error!("Radio error = {}", err);
            return Err(err);
        }
    };
    Ok(())
}

async fn tx_buffer(
    lora: &mut Lora,
    mdltn_params: &ModulationParams,
    tx_pkt_params: &mut PacketParams,
    buff: &[u8],
) -> Result<(), RadioError> {
    match lora.tx(mdltn_params, tx_pkt_params, buff, 0xffffff).await {
        Ok(()) => {
            info!("Sending message: {}", core::str::from_utf8(buff).unwrap());
            LED_BLUE_BLINK_SIGNAL.signal(());
        }
        Err(err) => {
            error!("Radio error = {}", err);
            return Err(err);
        }
    };
    Ok(())
}

async fn prepare_tx(
    lora: &mut Lora,
    mdltn_params: &ModulationParams,
) -> Result<(), RadioError> {
    match lora.prepare_for_tx(mdltn_params, OUTPUT_POWER, false).await {
        Ok(()) => {
            debug!("Radio prepared for TX");
        }
        Err(err) => {
            error!("Radio error = {}", err);
            return Err(err);
        }
    };
    Ok(())
}

fn create_rx_packet(lora: &mut Lora, mdltn_params: &ModulationParams) -> Option<PacketParams> {
    Some(
        match lora.create_rx_packet_params(4, false, RX_BUF_SIZE as u8, true, false, mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                return None;
            }
        },
    )
}

fn create_tx_packet(lora: &mut Lora, mdltn_params: &ModulationParams) -> Option<PacketParams> {
    Some(
        match lora.create_tx_packet_params(4, false, true, false, mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                error!("Radio error = {}", err);
                return None;
            }
        },
    )
}

fn modulation_params(lora: &mut Lora) -> Option<ModulationParams> {
    Some(
        match lora.create_modulation_params(
            SpreadingFactor::_10,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                error!("Radio error = {}", err);
                return None;
            }
        },
    )
}
