use std::time::Duration;

use tokio::time::sleep;

use crate::display::{PartialRenderCapableDisplay, inland_fpc_a002::DisplayPins};

mod display;
//GPIO23 Busy
//GPIO24 Reset (on LOW)
//GPIO22 Data/Command (Data when HIGH)
// Device on SPI0 CHIP0
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let disp = display::inland_fpc_a002::Display::new(DisplayPins {
        spi_bus: 0,
        spi_chip: 0,
        busy_pin: 23,
        res_pin: 24,
        ds_pin: 22,
    })?;
    println!("Initializing display");
    disp.init().await?;
    disp.white_screen().await?;
    Ok(())
}
