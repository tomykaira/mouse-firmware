#![no_std]
#![no_main]
// #![deny(warnings)]

#[cfg(not(feature = "semihosting"))]
extern crate panic_halt;
#[cfg(feature = "semihosting")]
extern crate panic_semihosting;

use cortex_m::peripheral::DWT;
#[allow(unused_imports)]
#[cfg(feature = "semihosting")]
use cortex_m_semihosting::hprintln;
use rtic::cyccnt::{Instant, U32Ext as _};
use stm32f1xx_hal::usb::{Peripheral, UsbBus, UsbBusType};
use stm32f1xx_hal::{prelude::*, stm32};
use usb_device::bus;
use usb_device::prelude::*;

use cortex_m::asm::delay;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::{Mode, Phase, Polarity};
use hid::HIDClass;
use stm32f1xx_hal::gpio::gpioa::*;
use stm32f1xx_hal::gpio::gpioc::PC13;
use stm32f1xx_hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use stm32f1xx_hal::spi::{Spi, Spi1NoRemap};

mod hid;
mod pmw3360;
mod reset;

// Do not change CLOCK while using STM32L412.
const CLOCK: u32 = 8; // MHz
const READ_PERIOD: u32 = CLOCK * 1000; // about 1ms

#[rtic::app(device = stm32f1xx_hal::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice<'static, UsbBusType>,
        hid: HIDClass<'static, UsbBusType>,
        spi: Spi<
            stm32::SPI1,
            Spi1NoRemap,
            (
                PA5<Alternate<PushPull>>,
                PA6<Input<Floating>>,
                PA7<Alternate<PushPull>>,
            ),
            u8,
        >,
        indicator: PC13<Output<PushPull>>,
    }

    #[init(schedule = [read_loop])]
    fn init(mut cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        cx.core.DCB.enable_trace();
        DWT::unlock();
        cx.core.DWT.enable_cycle_counter();

        let mut flash = cx.device.FLASH.constrain();
        let mut rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);

        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);
        let mut gpioc = cx.device.GPIOC.split(&mut rcc.apb2);

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        let spi_mode = Mode {
            polarity: Polarity::IdleHigh,
            phase: Phase::CaptureOnSecondTransition,
        };

        let mut spi = Spi::spi1(
            cx.device.SPI1,
            (
                gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl),
                gpioa.pa6.into_floating_input(&mut gpioa.crl),
                gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl),
            ),
            &mut afio.mapr,
            spi_mode,
            100.khz(),
            clocks,
            &mut rcc.apb2,
        );

        delay(clocks.sysclk().0 / 100);
        let mut indicator = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        pmw3360::Pmw3360::new(&mut spi, &mut indicator).power_up();

        let usb_dm = gpioa.pa11;
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low().ok();
        delay(clocks.sysclk().0 / 100);

        let usb = Peripheral {
            usb: cx.device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
        };

        *USB_BUS = Some(UsbBus::new(usb));

        let hid = HIDClass::new(USB_BUS.as_ref().unwrap());

        let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0xc410, 0x0000))
            .manufacturer("hello")
            .product("mouse")
            .serial_number("TEST")
            .device_class(0)
            .device_protocol(0x2) // mouse
            .build();

        cx.schedule.read_loop(cx.start + READ_PERIOD.cycles()).ok();

        init::LateResources {
            usb_dev,
            hid,
            spi,
            indicator,
        }
    }

    #[task(schedule = [read_loop], resources = [spi, hid, indicator], priority = 1)]
    fn read_loop(mut cx: read_loop::Context) {
        cx.schedule
            .read_loop(Instant::now() + READ_PERIOD.cycles())
            .ok();

        let spi = &mut cx.resources.spi;
        let indicator = &mut cx.resources.indicator;
        let hid = &mut cx.resources.hid;
        let mut pmw = pmw3360::Pmw3360::new(spi, indicator);

        let (dx, dy) = pmw.read_dx_dy();
        let dx = 1;

        let report_buffer = [
            0u8, // clicks
            dx as u8, dy as u8,
        ];

        match hid.lock(|h| h.write(&report_buffer)) {
            Err(UsbError::WouldBlock) => (),
            Err(UsbError::BufferOverflow) => panic!("BufferOverflow"),
            Err(_) => panic!("Undocumented usb error"),
            Ok(_) => (),
        }
    }

    #[task(binds=USB_LP_CAN_RX0, resources = [usb_dev, hid], priority = 2)]
    fn usb_rx(mut cx: usb_rx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.hid);
    }

    #[task(binds=USB_HP_CAN_TX, resources = [usb_dev, hid], priority = 2)]
    fn usb_tx(mut cx: usb_tx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.hid);
    }

    extern "C" {
        fn EXTI0();
    }
};

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>, hid: &mut HIDClass<'static, B>) {
    if !usb_dev.poll(&mut [hid]) {
        return;
    }
}
