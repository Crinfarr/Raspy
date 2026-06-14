use rppal::{
    self,
    gpio::{Gpio, InputPin, OutputPin},
    spi::{self, Spi},
};
use std::{io::Result, thread::yield_now, time::Duration};
use tokio::{join, sync::RwLock};

use crate::display::PartialRenderCapableDisplay;

pub struct DisplayPins {
    pub spi_bus: u8,
    pub spi_chip: u8,
    pub ds_pin: u8,
    pub res_pin: u8,
    pub busy_pin: u8,
}

pub struct Display {
    //pins: self::DisplayPins,
    spi: RwLock<Spi>,
    _gpio: Gpio,
    p_data_select: RwLock<OutputPin>,
    p_reset: RwLock<OutputPin>,
    p_busy: InputPin,
}
#[repr(u8)]
#[allow(unused)]
enum SpiFlag {
    ///reset
    SPIVoid = 0x00,
    /// 0bAAAAAAAA
    /// 0b00000BBB
    DriverOutputControl = 0x01,
    /// 0b000AAAAA
    /// 0b0000BBBB
    GateDrivingVoltageControl = 0x03,
    /// 0b000AAAAA
    SourceDrivingVoltageControl,
    /// 0b0000000A
    /// A=1: Sleep
    DeepSleepMode = 0x10,
    /// 00000ABB
    /// A=1: Address counter horizontal, else vertical
    /// BB: motion direction, 0=decrement, YX
    DataEntryMode,
    /// No trailing data
    SWReset,
    /// 0bAAAAAAAA 0bBBBB0000
    TempSensorCtl = 0x1A,
    /// Activates full display update sequence
    Activate = 0x20,
    /// 0bAAAAAAAA
    DisplayUpdateControlA,
    /// 0bAAAAAAAA
    /// 0xFF: Clock, Analog, LUT, Initial, Pattern, rm(Analog), rm(OSC)
    /// 0xD7: LUT from OTP - Clock, Analog, LUT, Pattern, rm(Analog), rm(OSC)
    /// 0xC7: LUT from MCU - Clock, Analog, Pattern, rm(Analog), rm(OSC)
    DisplayUpdateControlB,
    /// No trailing data, check with 2F
    BreakDetect,
    /// Any amount of trailing data until next command call
    WriteRAM,
    /// 0bAAAAAAAA
    WriteVCOM = 0x2C,
    ///UNSUPPORTED
    StatusRead = 0x2F,
    /// 30 bytes trailing
    WriteLUT = 0x32,
    /// 0b0AAAAAAA
    SetDummyLine = 0x3A,
    ///0b0000AAAA
    SetGateLine,
    /// 0bAAAA00AA
    BorderWaveformCtl,
    /// 0b000AAAAA 0b000BBBBB
    /// AAAAA: Start addr
    /// BBBBB: End addr
    XWindow = 0x44,
    /// 0b000AAAAA 0b000BBBBB
    /// AAAAA: Start addr
    /// BBBBB: End addr
    YWindow,
    /// 0b000AAAAA
    XSeek = 0x4E,
    /// 0bAAAAAAAA
    YSeek,
}

impl Display {
    pub fn new(pin_setup: DisplayPins) -> Result<Self> {
        let gpio_handle = Gpio::new().map_err(std::io::Error::other)?;
        Ok(Display {
            //pins: pin_setup,
            spi: RwLock::new(
                Spi::new(
                    match pin_setup.spi_bus {
                        0 => spi::Bus::Spi0,
                        1 => spi::Bus::Spi1,
                        2 => spi::Bus::Spi2,
                        3 => spi::Bus::Spi3,
                        4 => spi::Bus::Spi4,
                        5 => spi::Bus::Spi5,
                        6 => spi::Bus::Spi6,
                        oob => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Unsupported,
                                format!("Spi{oob} outside supported bounds [0,6]"),
                            ));
                        }
                    },
                    match pin_setup.spi_chip {
                        00 => spi::SlaveSelect::Ss0,
                        01 => spi::SlaveSelect::Ss1,
                        02 => spi::SlaveSelect::Ss2,
                        03 => spi::SlaveSelect::Ss3,
                        04 => spi::SlaveSelect::Ss4,
                        05 => spi::SlaveSelect::Ss5,
                        06 => spi::SlaveSelect::Ss6,
                        07 => spi::SlaveSelect::Ss7,
                        08 => spi::SlaveSelect::Ss8,
                        09 => spi::SlaveSelect::Ss9,
                        10 => spi::SlaveSelect::Ss10,
                        11 => spi::SlaveSelect::Ss11,
                        12 => spi::SlaveSelect::Ss12,
                        13 => spi::SlaveSelect::Ss13,
                        14 => spi::SlaveSelect::Ss14,
                        15 => spi::SlaveSelect::Ss15,
                        oob => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Unsupported,
                                format!("Chip {oob} outside supported bounds [0,16)"),
                            ));
                        }
                    },
                    5 * 1024 * 1024,
                    spi::Mode::Mode0,
                )
                .map_err(std::io::Error::other)?,
            ),
            p_busy: gpio_handle
                .get(pin_setup.busy_pin)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        format!("Failed to get pin {}: {e}", pin_setup.busy_pin),
                    )
                })?
                .into_input(),
            p_data_select: RwLock::new(
                gpio_handle
                    .get(pin_setup.ds_pin)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            format!("Failed to get pin {}: {e}", pin_setup.busy_pin),
                        )
                    })?
                    .into_output_high(),
            ),
            p_reset: RwLock::new(
                gpio_handle
                    .get(pin_setup.res_pin)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            format!("Failed to get pin {}: {e}", pin_setup.busy_pin),
                        )
                    })?
                    .into_output_high(),
            ),
            _gpio: gpio_handle,
        })
    }
    pub async fn init(&self) -> Result<()> {
        //Hard reset chip MCU
        self.reset().await?;
        self.wait_for_availability().await;
        self.signal(SpiFlag::SWReset, &[]).await?;

        //ripped directly from the manufacturer's manual if this doesn't work we're fucked i guess
        self.signal(SpiFlag::DriverOutputControl, &[0xf9, 0x00])
            .await?;

        //set motion
        self.signal(SpiFlag::DataEntryMode, &[0b00000_0_11]).await?;
        self.signal(SpiFlag::XWindow, &[0x00, 0x0F]).await?;
        self.signal(SpiFlag::YWindow, &[0x00, 0xF9]).await?;

        //set border waveform
        self.signal(SpiFlag::BorderWaveformCtl, &[0b0000_00_01])
            .await?;

        //maybe command 0x18 here it's in the example code but i have no idea why

        //seek to (0, 199)
        self.signal(SpiFlag::XSeek, &[0x00]).await?;
        self.signal(SpiFlag::YSeek, &[0x00]).await?;
        self.wait_for_availability().await;
        Ok(())
    }
    pub async fn black_screen(&self) -> Result<()> {
        //4000b stolen from example code
        self.wait_for_availability().await;
        let block = [0x00u8; 0x0f * 0xf9];
        self.signal(SpiFlag::WriteRAM, &block).await?;
        self.signal(SpiFlag::DisplayUpdateControlB, &[0xFF]).await?;
        self.signal(SpiFlag::Activate, &[]).await?;
        Ok(())
    }
    pub async fn white_screen(&self) -> Result<()> {
        //4000b stolen from example code
        self.wait_for_availability().await;
        let block = [0xffu8; 0x0f * 0xf9];
        self.signal(SpiFlag::XSeek, &[0x00]).await?;
        self.signal(SpiFlag::WriteRAM, &block).await?;
        self.signal(SpiFlag::DisplayUpdateControlB, &[0xFF]).await?;
        self.signal(SpiFlag::Activate, &[]).await?;
        Ok(())
    }
    pub async fn draw_px(&self, x: u8, y: u8, black: bool) {
        todo!()
    }

    /// Immediately returns if the display is ready for use or not
    pub fn available(&self) -> bool {
        !self.p_busy.is_high()
    }
    /// Blocks until the display is ready for use
    pub async fn wait_for_availability(&self) {
        while self.p_busy.is_high() {
            yield_now();
        }
    }
    /// Hard resets the MCU of the display
    pub async fn reset(&self) -> Result<()> {
        let mut p_writelock = self.p_reset.write().await;
        p_writelock.set_low();
        tokio::time::sleep(Duration::from_millis(10)).await;
        p_writelock.set_high();
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
    ///Sends a SPI command to the display with trailing data
    async fn signal(&self, flag: self::SpiFlag, trailing_data: &[u8]) -> Result<()> {
        self.write_command(flag as u8).await?;
        if trailing_data.len() != 0 {
            self.write_data(&trailing_data).await?;
        }
        Ok(())
    }
    async fn write_command(&self, command: u8) -> Result<()> {
        let (mut ds, mut spi) = join!(self.p_data_select.write(), self.spi.write());
        ds.set_low();
        spi.write(&[command]).map_err(std::io::Error::other)?;
        Ok(())
    }
    async fn write_data(&self, data: &[u8]) -> Result<()> {
        let (mut ds, mut spi) = join!(self.p_data_select.write(), self.spi.write());
        ds.set_high();
        spi.write(data).map_err(std::io::Error::other)?;
        Ok(())
    }
}

impl super::PeripheralDisplay for self::Display {
    const BITS_PER_PIXEL: u8 = 1;
    const DIMENSIONS: [u16; 2] = [16, 250];
    async fn show_img(&self, px: &[u8]) -> std::io::Result<()> {
        if (px.len() * 8)
            < (self::Display::DIMENSIONS[0]
                * self::Display::DIMENSIONS[1]
                * self::Display::BITS_PER_PIXEL as u16) as usize
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Buffer too large to display",
            ));
        }
        self.signal(SpiFlag::XSeek, &[0x00]).await?;
        self.signal(SpiFlag::YSeek, &[0x00]).await?;
        self.signal(SpiFlag::WriteRAM, px).await?;
        self.wait_for_availability().await;
        self.signal(SpiFlag::DisplayUpdateControlB, &[0xFF]).await?;
        self.signal(SpiFlag::Activate, &[]).await?;

        Ok(())
    }
}
impl PartialRenderCapableDisplay<u8> for self::Display {
    const ORIGINAL_WINDOW: [u8; 4] = [0, 0x0f, 0, 0xf7];
    async fn partial_upd(&self, window: &[u8; 4], px: &[u8]) -> std::io::Result<()> {
        if ((window[1] - window[0]) as usize * (window[3] - window[2]) as usize) < px.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                format!("pxbuf too large for specified window {window:?}"),
            ));
        }
        self.signal(SpiFlag::XWindow, &window[0..2]).await?;
        self.signal(SpiFlag::XSeek, &[window[0]]).await?;
        self.signal(SpiFlag::YWindow, &window[2..4]).await?;
        self.signal(SpiFlag::YSeek, &[window[2]]).await?;
        self.signal(SpiFlag::WriteRAM, px).await?;
        self.wait_for_availability().await;
        self.signal(SpiFlag::DisplayUpdateControlB, &[0xFF]).await?;
        self.signal(SpiFlag::Activate, &[]).await?;
        self.signal(SpiFlag::XWindow, &self::Display::ORIGINAL_WINDOW[0..2])
            .await?;
        self.signal(SpiFlag::YWindow, &self::Display::ORIGINAL_WINDOW[2..4])
            .await?;
        Ok(())
    }
}
