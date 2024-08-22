use crossbeam_channel::bounded;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_svc::hal::prelude::*;
use log::info;


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

    // Get Max Duty and Calculate Upper and Lower Limits for Servo
    let max_duty = driver.get_max_duty();
    info!("Max Duty {}", max_duty);
    let min_limit = max_duty * 25 / 1000;
    info!("Min Limit {}", min_limit);
    let max_limit = max_duty * 125 / 1000;
    info!("Max Limit {}", max_limit);

    // Define Starting Position
    driver
        .set_duty(compute_duty(0, 0, 180, min_limit, max_limit))
        .unwrap();
    // Give servo some time to update
    FreeRtos::delay_ms(500);

    let _servo_thread = std::thread::Builder::new()
        .stack_size(SERVO_STACK_SIZE)
        .spawn(move || servo_run_function(driver, rx));

}

fn servo_run_function(
    mut driver: esp_idf_hal::ledc::LedcDriver,
    rx: crossbeam_channel::Receiver<bool>,
) {
    loop {
        driver.set_duty(compute_duty(45,0, 180, 409, 2048)).unwrap();
        FreeRtos::delay_ms(20);
    }
}

fn compute_duty(
    angle: u32,
    in_min: u32,
    in_max: u32,
    min_limit: u32,
    max_limit: u32
) -> u32 {
    (angle - in_min) * (max_limit - min_limit) / (in_max - in_min) + max_limit
}