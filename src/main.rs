use std::time::Duration;

use rand::random_range;
use tokio::time::sleep;

use crate::display::{
    PartialRenderCapableDisplay, PeripheralDisplay, inland_fpc_a002::DisplayPins,
};

mod display;
//GPIO23 Busy
//GPIO24 Reset (on LOW)
//GPIO22 Data/Command (Data when HIGH)
// Device on SPI0 CHIP0
#[tokio::main]
async fn main() -> std::io::Result<()> {
    print!("Initializing display...");
    let disp = display::inland_fpc_a002::Display::new(DisplayPins {
        spi_bus: 0,
        spi_chip: 0,
        busy_pin: 23,
        res_pin: 24,
        ds_pin: 22,
    })
    .await?;
    println!("Done");
    sleep(Duration::from_millis(1000)).await;
    println!("Running display test");
    let rect = &display::inland_fpc_a002::Display::DIMENSIONS;
    loop {
        let (x, y) = (random_range(0..rect[0]), random_range(0..rect[1]));
    }
}
