#![no_std]
#![no_main]

use crate::hal::{stm32, prelude::*};
use core::panic::PanicInfo;
use cortex_m_rt::entry;
use stm32::interrupt;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal as hal;

//PLLI2S clock configuration
const PLLI2SM: u8 = 4;
const PLLI2SN: u16 = 192;
const PLLI2SR: u8 = 5;

//Clock configuration of the used i2s interface
const I2SDIV: u8 = 12;
const ODD: bool = true;

//generate Master Clock ? Modifying this require to adapt the i2s clock
const MCK:bool = false;

//Periode of Sawtooth in number of sample
const SAWTOOTH_PERIOD: u16 = 48_000 / 110;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let device = stm32::Peripherals::take().unwrap();
    let gpiob = device.GPIOB.split();
    let gpioc = device.GPIOC.split();
    let rcc = device.RCC.constrain();
    let _clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(96.mhz())
        .hclk(96.mhz())
        .pclk1(50.mhz())
        .pclk2(100.mhz())
        .freeze();
    //enable system clock on APB1 bus and SPI2
    unsafe {
        let rcc = &(*stm32::RCC::ptr());
        rcc.apb1enr
            .modify(|_, w| w.pwren().set_bit().spi2en().set_bit());
    }

    //setup  and startup common i2s clock
    unsafe {
        let rcc = &(*stm32::RCC::ptr());
        //setup
        rcc.plli2scfgr.modify(|_, w| {
            w.plli2sr()
                .bits(PLLI2SR)
                .plli2sn()
                .bits(PLLI2SN)
                .plli2sm()
                .bits(PLLI2SM)
        });
        //run the clock
        rcc.cr.modify(|_, w| w.plli2son().set_bit());
        //wait for a stable clock
        while rcc.cr.read().plli2srdy().bit_is_clear() {}
    }
    //i2s2 gpio
    //Note, on nucleo board possible i2s2 gpio are:
    //  SD: pb15, pc3
    //  WS: pb9, pb12
    //  CK: pb10, pb13, pc7
    //  MCK: pa3, pa6, pc6

    let _pb15 = gpiob.pb15.into_alternate_af5(); //SD DIN
    let _pb12 = gpiob.pb12.into_alternate_af5(); //WS LRCK
    let _pb13 = gpiob.pb13.into_alternate_af5(); //CK BCK
    let _pc6 = gpioc.pc6.into_alternate_af5(); //MCK SCK

    //spi2 interrupt
    unsafe {
        let spi2 = &(*stm32::SPI2::ptr());
        spi2.cr2
            .modify(|_, w| w.txeie().clear_bit().rxneie().clear_bit().errie().set_bit());
        stm32::NVIC::unmask(stm32::Interrupt::SPI2);
    }

    //Spi2 setup for i2s mode
    unsafe {
        let spi2 = &(*stm32::SPI2::ptr());
        spi2.i2spr
            .modify(|_, w| w.i2sdiv().bits(I2SDIV).odd().bit(ODD).mckoe().bit(MCK));
        spi2.i2scfgr.modify(|_, w| {
            w.i2smod()
                .i2smode() //
                .i2scfg()
                .master_tx() //
                .pcmsync()
                .long() //
                .i2sstd()
                .philips() //
                .ckpol()
                .idle_low() //
                .datlen()
                .sixteen_bit() //
                .chlen()
                .thirty_two_bit() //
                .i2se()
                .enabled() //start i2S
        })
    }

    loop {
        for spl_n in 0..SAWTOOTH_PERIOD as i16 {
            let spl: i16 = (spl_n - (SAWTOOTH_PERIOD as i16) / 2)
                * (i16::MAX / ((SAWTOOTH_PERIOD as i16) / 2));
            let l = spl;
            let r = spl;

            unsafe {
                let spi2 = &(*stm32::SPI2::ptr());
                while !spi2.sr.read().txe().bit() {}
                spi2.dr.modify(|_, w| w.dr().bits(l as u16));
                while !spi2.sr.read().txe().bit() {}
                spi2.dr.modify(|_, w| w.dr().bits(r as u16));
            }
        }
    }
}

#[interrupt]
fn SPI2() {
    static mut SPL_N: u16 = 0;
    unsafe {
        let spi2 = &(*stm32::SPI2::ptr());
        if spi2.sr.read().fre().bit() {
            rprintln!("Frame Error");
        }
        if spi2.sr.read().ovr().bit() {
            rprintln!("Overrun");
        }
        if spi2.sr.read().udr().bit() {
            rprintln!("underrun");
        }
        if spi2.sr.read().txe().bit() {
            let side = spi2.sr.read().chside().bit();
            let spl: i16 = (*SPL_N as i16 - (SAWTOOTH_PERIOD as i16) / 2)
                * (i16::MAX / ((SAWTOOTH_PERIOD as i16) / 2));

            spi2.dr.modify(|_, w| w.dr().bits(spl as u16));
            if side {
                *SPL_N+= 1;
                if *SPL_N == SAWTOOTH_PERIOD {
                    *SPL_N = 0;
                }
            }
        }
    }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rprintln!("{}", info);
    loop {} // You might need a compiler fence in here.
}
