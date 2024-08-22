use std::thread;

use crossbeam_channel::{bounded, TrySendError};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_hal::task::current;
use esp_idf_svc::hal::prelude::*;
use log::{debug, info};

static SERVO_STACK_SIZE: usize = 2000;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Starting servo setup.");

    let peripherals = Peripherals::take().unwrap();

    let (tx, rx) = bounded(1);
    
    info!("Sending 10 to angle!");
    tx.send(10).unwrap();

    
    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(50.Hz())
            .resolution(Resolution::Bits14),
    )
    .unwrap();

    let mut driver = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver,
        peripherals.pins.gpio22,
    )
    .unwrap();

    let _servo_thread = std::thread::Builder::new()
        .stack_size(SERVO_STACK_SIZE)
        .spawn(move || servo_run_function(driver, rx));


    loop {
        info!("Trying to send 100...");
        match tx.try_send(100) {
            Ok(_) => break,
            Err(TrySendError::Full(_)) => {},
            Err(TrySendError::Disconnected(_)) => {} 
        }
        FreeRtos::delay_ms(100);
    }


    loop {
        info!("Trying to send 180...");
        match tx.try_send(180) {
            Ok(_) => break,
            Err(TrySendError::Full(_)) => {},
            Err(TrySendError::Disconnected(_)) => {} 
        }
        FreeRtos::delay_ms(100);
    }

    loop {
        info!("Trying to send 0...");
        match tx.try_send(0) {
            Ok(_) => break,
            Err(TrySendError::Full(_)) => {},
            Err(TrySendError::Disconnected(_)) => {} 
        }
        FreeRtos::delay_ms(100);
    }

    let _ = _servo_thread.unwrap().join();

}

fn servo_run_function(
    mut driver: esp_idf_hal::ledc::LedcDriver,
    rx: crossbeam_channel::Receiver<u32>,
) {
    let mut current_angle: u32 = 0;

    // Compute the min/max limits for the mapping funciton
    // Get Max Duty and Calculate Upper and Lower Limits for Servo
    let max_duty = driver.get_max_duty();
    info!("Max Duty {}", max_duty);
    let min_limit = max_duty * 25 / 1000;
    info!("Min Limit {}", min_limit);
    let max_limit = max_duty * 125 / 1000;
    info!("Max Limit {}", max_limit);

    let duty = compute_duty(current_angle, 0, 180, min_limit, max_limit);
    info!("First Setting {duty}");

    // Define Starting Position
    driver
        .set_duty(duty)
        .unwrap();
    // Give servo some time to update
    FreeRtos::delay_ms(500);

    info!("Moved to initial position");

    let mut received = 0;

    loop {
        match rx.try_recv() {
            Ok(msg) => {
                received = msg;
                info!("Thread Received: {received}");
            }
            Err(_) => {}
        }

        info!("Received: {received} - Current: {current_angle}");

        if current_angle < received {
            while current_angle < received {
                current_angle += 1;
                let duty = compute_duty(current_angle, 0, 180, min_limit, max_limit);
                info!("Current angle: {current_angle} - Setting {duty}");
                driver
                    .set_duty(duty)
                    .unwrap();
                FreeRtos::delay_ms(12);
            }
        }

        if current_angle > received {
            while current_angle > received {
                current_angle -= 1;
                let duty = compute_duty(current_angle, 0, 180, min_limit, max_limit);
                info!("Current angle: {current_angle} - Setting {duty}");
                driver
                    .set_duty(duty)
                    .unwrap();
                FreeRtos::delay_ms(12);
            }
        }

        FreeRtos::delay_ms(1000);
    }
}

fn compute_duty(angle: u32, in_min: u32, in_max: u32, min_limit: u32, max_limit: u32) -> u32 {
    (angle - in_min) * (max_limit - min_limit) / (in_max - in_min) + min_limit
}
