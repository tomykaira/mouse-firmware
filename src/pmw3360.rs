use cortex_m::asm::delay;
#[allow(unused_imports)]
#[cfg(feature = "semihosting")]
use cortex_m_semihosting::hprintln;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::*;
use stm32f1xx_hal::gpio::gpioc::PC13;
use stm32f1xx_hal::gpio::{Output, PushPull};
use stm32f1xx_hal::spi::*;
use stm32f1xx_hal::stm32::SPI1;

// Assume CPU clock is 48MHz.
const CLOCK_MHZ: u32 = 8;
const CYCLES_50MS: u32 = CLOCK_MHZ * 1000 * 50;
const CYCLES_160US: u32 = CLOCK_MHZ * 160;

pub struct Pmw3360<'a, PINS> {
    spi: &'a mut Spi<SPI1, Spi1NoRemap, PINS, u8>,
    indicator: &'a mut PC13<Output<PushPull>>,
}

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug)]
pub enum Register {
    ProductId,
    RevisionId,
    Motion,
    DeltaXL,
    DeltaXH,
    DeltaYL,
    DeltaYH,
    SQUAL,
    RawDataSum,
    MaximumRawData,
    MinimumRawData,
    ShutterLower,
    ShutterUpper,
    Control,
    Config1,
    Config2,
    AngleTune,
    FrameCapture,
    SromEnable,
    RunDownshift,
    Rest1RateLower,
    Rest1RateUpper,
    Rest1Downshift,
    Rest2RateLower,
    Rest2RateUpper,
    Rest2Downshift,
    Rest3RateLower,
    Rest3RateUpper,
    Observation,
    DataOutLower,
    DataOutUpper,
    RawDataDump,
    SromId,
    MinSqRun,
    RawDataThreshold,
    Config5,
    PowerUpReset,
    Shutdown,
    InverseProductId,
    LiftCutoffTune3,
    AngleSnap,
    LiftCutoffTune1,
    MotionBurst,
    LiftCutoffTuneTimeout,
    LiftCutoffTuneMinLength,
    SromLoadBurst,
    LiftConfig,
    RawDataBurst,
    LiftCutoffTune2,
}

impl Register {
    fn value(&self) -> u8 {
        match self {
            Register::ProductId => 0x00,
            Register::RevisionId => 0x01,
            Register::Motion => 0x02,
            Register::DeltaXL => 0x03,
            Register::DeltaXH => 0x04,
            Register::DeltaYL => 0x05,
            Register::DeltaYH => 0x06,
            Register::SQUAL => 0x07,
            Register::RawDataSum => 0x08,
            Register::MaximumRawData => 0x09,
            Register::MinimumRawData => 0x0a,
            Register::ShutterLower => 0x0b,
            Register::ShutterUpper => 0x0c,
            Register::Control => 0x0d,
            Register::Config1 => 0x0f,
            Register::Config2 => 0x10,
            Register::AngleTune => 0x11,
            Register::FrameCapture => 0x12,
            Register::SromEnable => 0x13,
            Register::RunDownshift => 0x14,
            Register::Rest1RateLower => 0x15,
            Register::Rest1RateUpper => 0x16,
            Register::Rest1Downshift => 0x17,
            Register::Rest2RateLower => 0x18,
            Register::Rest2RateUpper => 0x19,
            Register::Rest2Downshift => 0x1a,
            Register::Rest3RateLower => 0x1b,
            Register::Rest3RateUpper => 0x1c,
            Register::Observation => 0x24,
            Register::DataOutLower => 0x25,
            Register::DataOutUpper => 0x26,
            Register::RawDataDump => 0x29,
            Register::SromId => 0x2a,
            Register::MinSqRun => 0x2b,
            Register::RawDataThreshold => 0x2c,
            Register::Config5 => 0x2f,
            Register::PowerUpReset => 0x3a,
            Register::Shutdown => 0x3b,
            Register::InverseProductId => 0x3f,
            Register::LiftCutoffTune3 => 0x41,
            Register::AngleSnap => 0x42,
            Register::LiftCutoffTune1 => 0x4a,
            Register::MotionBurst => 0x50,
            Register::LiftCutoffTuneTimeout => 0x58,
            Register::LiftCutoffTuneMinLength => 0x5a,
            Register::SromLoadBurst => 0x62,
            Register::LiftConfig => 0x63,
            Register::RawDataBurst => 0x64,
            Register::LiftCutoffTune2 => 0x65,
        }
    }
}

impl<PINS> Pmw3360<'_, PINS> {
    pub fn new<'a>(
        spi: &'a mut Spi<SPI1, Spi1NoRemap, PINS, u8>,
        indicator: &'a mut PC13<Output<PushPull>>,
    ) -> Pmw3360<'a, PINS> {
        Pmw3360 { spi, indicator }
    }

    pub fn write(&mut self, register: Register, value: u8) {
        let data = [0x80 | register.value(), value];
        self.spi.write(&data).unwrap();
    }

    fn write_byte(&mut self, data: u8) {
        loop {
            match self.spi.send(data) {
                Ok(_) => {
                    self.indicator.set_high();
                    return;
                }
                Err(nb::Error::WouldBlock) => {
                    self.indicator.set_low();
                } // continue
                Err(other) => {
                    #[cfg(feature = "semihosting")]
                    hprintln!("error {:?}", other);
                    panic!("error {:?}", other);
                }
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        loop {
            match self.spi.read() {
                Ok(v) => {
                    self.indicator.set_high();
                    return v;
                }
                Err(nb::Error::WouldBlock) => {}
                Err(other) => {
                    #[cfg(feature = "semihosting")]
                    hprintln!("error {:?}", other);
                    panic!("error {:?}", other);
                }
            }
        }
    }

    pub fn read(&mut self, register: Register) -> u8 {
        self.write_byte(register.value());
        // Wait a cycle and discard written byte.
        self.read_byte();
        // Wait for t (SRAD) (160us)
        delay(CYCLES_160US);
        // Write dummy 0 to cycle SCK for reading.
        self.write_byte(0);
        self.read_byte()
    }

    pub fn read_dx_dy(&mut self) -> (i16, i16) {
        // Run step 2 and 3 of Motion register usage.
        let motion = self.read(Register::Motion);
        // MOT is high
        if (motion & 0x80) != 0 {
            let dxl = self.read(Register::DeltaXL);
            let dxh = self.read(Register::DeltaXH);
            let dyl = self.read(Register::DeltaYL);
            let dyh = self.read(Register::DeltaYH);

            // Interpret bytes as 2's complement number
            let dx = (((dxh as u16) << 8) | dxl as u16) as i16;
            let dy = (((dyh as u16) << 8) | dyl as u16) as i16;

            (dx, dy)
        } else {
            (0, 0)
        }
    }

    pub fn power_up(&mut self) {
        self.write(Register::PowerUpReset, 0x5a);
        delay(CYCLES_50MS);
        self.read(Register::Motion);
        self.read(Register::DeltaXL);
        self.read(Register::DeltaXH);
        self.read(Register::DeltaYL);
        self.read(Register::DeltaYH);
        // SROM download, if needed.
        // Setup registers if needed.

        // Reset motion (Register usage step 1)
        self.write(Register::Motion, 0x00);
    }
}
