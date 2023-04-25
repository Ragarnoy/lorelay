//! This example showcases how to notify a connected client via BLE of new SAADC data.
//! Using, for example, nRF-Connect on iOS/Android we can connect to the device "HelloRust"
//! and see the battery level characteristic getting updated in real-time.
//!
//! The SAADC is initialized in single-ended mode and a single measurement is taken every second.
//! This value is then used to update the battery_level characteristic.
//! We are using embassy-time for time-keeping purposes.
//! Everytime a new value is recorded, it gets sent to the connected clients via a GATT Notification.
//!
//! The ADC doesn't gather data unless a valid connection exists with a client. This is guaranteed
//! by using the "select" crate to wait for either the `gatt_server::run` future or the `adc_fut` future
//! to complete.
//!
//! Only a single BLE connection is supported in this example so that RAM usage remains minimal.
//!
//! The internal RC oscillator is used to generate the LFCLK.
//!

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![macro_use]

use core::ffi::CStr;
use defmt_rtt as _; // global logger
use embassy_nrf as _; // time driver
use panic_probe as _;

use core::mem;

use defmt::{info, *};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive, Pin};
use embassy_nrf::saadc::{ChannelConfig, Saadc};
use embassy_nrf::{bind_interrupts, interrupt, saadc, spim, peripherals};
use embassy_nrf::interrupt::{Interrupt, InterruptExt};
use embassy_sync::blocking_mutex::raw::{ThreadModeRawMutex};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use futures::future::{select, Either};
use futures::pin_mut;
use nrf_softdevice::ble::{gatt_server, peripheral, Connection};
use nrf_softdevice::{raw, Softdevice};

static mut LED_FLAG: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);
bind_interrupts!(struct Irqs {
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    SAADC => saadc::InterruptHandler;
});

/// Reads the current ADC value every second and notifies the connected client.
async fn notify_adc_value<'a>(
    saadc: &'a mut Saadc<'_, 1>,
    server: &'a Server,
    connection: &'a Connection,
) {
    loop {
        let mut buf = [0i16; 1];
        saadc.sample(&mut buf).await;

        // We only sampled one ADC channel.
        let adc_raw_value: i16 = buf[0];

        // Try and notify the connected client of the new ADC value.
        match server.bas.battery_level_notify(connection, &adc_raw_value) {
            Ok(_) => info!("Battery adc_raw_value: {=i16}", &adc_raw_value),
            Err(_) => unwrap!(server.bas.battery_level_set(&adc_raw_value)),
        };

        // Sleep for one second.
        Timer::after(Duration::from_secs(1)).await
    }
}

#[embassy_executor::task]
async fn blink_once(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low, OutputDrive::Standard);
    loop {
        let should_blink = unsafe { LED_FLAG.try_lock() }
            .map(|flag| *flag)
            .unwrap_or(false);

        if should_blink {
            info!("Blinking once");
            led.set_low();
            Timer::after(Duration::from_secs(1)).await;
            led.set_high();
            Timer::after(Duration::from_secs(1)).await;
            let flag: &mut bool = unsafe { LED_FLAG.get_mut() };
            *flag = false;
        } else {
            led.set_high();
            Timer::after(Duration::from_millis(100)).await; // Check the Mutex flag more frequently
        }
    }
}

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[nrf_softdevice::gatt_service(uuid = "150f")]
struct CustomService {
    #[characteristic(uuid = "120f", read, write)]
    custom_value: [u8; 16],
}

#[nrf_softdevice::gatt_service(uuid = "180f")]
struct BatteryService {
    #[characteristic(uuid = "2a19", read, notify)]
    battery_level: i16,
}

#[nrf_softdevice::gatt_server]
struct Server {
    bas: BatteryService,
    custom: CustomService,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    // First we get the peripherals access crate.
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = interrupt::Priority::P2;
    config.time_interrupt_priority = interrupt::Priority::P2;
    let mut p = embassy_nrf::init(config);

    // Then we initialize the ADC. We are only using one channel in this example.
    let channel_config = ChannelConfig::single_ended(&mut p.P0_02);
    unsafe { interrupt::SAADC::steal() }.set_priority(interrupt::Priority::P2);
    let mut saadc = Saadc::new(p.SAADC, Irqs, saadc::Config::default(), [channel_config]);
    // Indicated: wait for ADC calibration.
    saadc.calibrate().await;
    info!("ADC calibrated");

    let config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 1,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: raw::BLE_GAP_ADV_SET_COUNT_DEFAULT as u8,
            periph_role_count: raw::BLE_GAP_ROLE_COUNT_PERIPH_DEFAULT as u8,
            central_role_count: 0,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    info!("Enabling softdevice");
    let sd = Softdevice::enable(&config);
    info!("Softdevice enabled");
    let server = unwrap!(Server::new(sd));

    spawner.spawn(blink_once(p.P0_13.degrade())).unwrap();
    unwrap!(spawner.spawn(softdevice_task(sd)));

    #[rustfmt::skip]
        let adv_data = &[
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x03, 0x03, 0x09, 0x18,
        0x0a, 0x09, b'H', b'e', b'l', b'l', b'o', b'R', b'u', b's', b't',
    ];
    #[rustfmt::skip]
        let scan_data = &[
        0x03, 0x03, 0x09, 0x18,
    ];

    let mut custom_value: i16 = 0;

    info!("starting advertising");
    loop {
        let config = peripheral::Config::default();

        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data,
            scan_data,
        };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);
        info!("advertising done! I have a connection.");

        // We have a GATT connection. Now we will create two futures:
        //  - An infinite loop gathering data from the ADC and notifying the clients.
        //  - A GATT server listening for events from the connected client.
        //
        // Event enums (ServerEvent's) are generated by nrf_softdevice::gatt_server
        // proc macro when applied to the Server struct above
        let adc_fut = notify_adc_value(&mut saadc, &server, &conn);
        let gatt_fut = gatt_server::run(&conn, &server, |e| match e {
            ServerEvent::Bas(e) => match e {
                BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    info!("battery notifications: {}", notifications);
                    if custom_value != 0 {
                        info!("custom value: {}", custom_value);
                    }
                }
            },
            ServerEvent::Custom(e) => match e {
                CustomServiceEvent::CustomValueWrite(value) => {
                    if let Ok(cstr) = CStr::from_bytes_until_nul(&value) {
                        if let Ok(ret) = cstr.to_str() {
                            info!("custom value written: {}", ret.clone());
                            ret
                        } else {
                            warn!("invalid utf8");
                            "invalid utf8"
                        }
                    }
                    else {
                        warn!("invalid utf8");
                        "invalid utf8"
                    };


                    custom_value = value[0] as i16;
                    unsafe {
                        let flag = LED_FLAG.get_mut();
                        *flag = !*flag;
                    }
                }
            },
        });

        pin_mut!(adc_fut);
        pin_mut!(gatt_fut);

        // We are using "select" to wait for either one of the futures to complete.
        // There are some advantages to this approach:
        //  - we only gather data when a client is connected, therefore saving some power.
        //  - when the GATT server finishes operating, our ADC future is also automatically aborted.
        match select(adc_fut, gatt_fut).await {
            Either::Left((_, _)) => {
                info!("ADC encountered an error and stopped!")
            }
            Either::Right((res, _)) => {
                info!("GATT server finished with result {:?}", res);
            }
        };
    }
}
