use embedded_hal::watchdog::WatchdogEnable;
use stm32f1xx_hal::pac::Peripherals;
use stm32f1xx_hal::time::MilliSeconds;
use stm32f1xx_hal::watchdog::IndependentWatchdog;

pub unsafe fn reset() {
    // Restart by watchdog.
    let p = Peripherals::steal();
    let mut wd = IndependentWatchdog::new(p.IWDG);
    wd.start(MilliSeconds(1));

    // Wait asynchronous reset.
    loop {}
}
