#![no_std]
#![no_main]

// pick a panicking behavior
//use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use crate::hal::{pac, prelude::*};
use core::panic::PanicInfo;
use cortex_m_rt::entry;
use pac::interrupt;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal as hal;

const PLLI2SM: u8 = 4;
const PLLI2SN: u16 = 192;
const PLLI2SR: u8 = 5;

const I2SDIV: u8 = 12;
const ODD: bool = true;

//const MCK_USE: bool = false;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let device = pac::Peripherals::take().unwrap();
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
        let rcc = &(*pac::RCC::ptr());
        rcc.apb1enr
            .modify(|_, w| w.pwren().set_bit().spi2en().set_bit());
    }

    //setup  and startup common i2s clock
    unsafe {
        let rcc = &(*pac::RCC::ptr());
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
        //wait a stable clock
        while rcc.cr.read().plli2srdy().bit_is_clear() {}
    }
    //i2s gpio
    //  SD pb15,pc3
    //  WS pb9, pb12
    //  CK pb10,pb13,pc7,*pd3
    //  MCK pa3, pa6, pc6,

    let _pb15 = gpiob.pb15.into_alternate_af5(); //SD DIN
    let _pb12 = gpiob.pb12.into_alternate_af5(); //WS LRCK
    let _pb13 = gpiob.pb13.into_alternate_af5(); //CK BCK
    let _pc6 = gpioc.pc6.into_alternate_af5(); //MCK SCK

    //spi2 interrupt
    unsafe {
        let spi2 = &(*pac::SPI2::ptr());
        spi2.cr2
            .modify(|_, w| w.txeie().clear_bit().rxneie().clear_bit().errie().set_bit());
        pac::NVIC::unmask(pac::Interrupt::SPI2);
    }

    //Spi2 setup for i2s mode
    unsafe {
        let spi2 = &(*pac::SPI2::ptr());
        spi2.i2spr.modify(|_, w| {
                w.i2sdiv().bits(I2SDIV).odd().bit(ODD).mckoe().disabled()
        });
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
    rprintln!("init done");
    //check spi2 status
    unsafe {
        let spi2 = &(*pac::SPI2::ptr());
        let spi2_sr = *((pac::SPI2::ptr() as usize + 0x08) as *const u32);
        rprintln!("{:#032b} {}", spi2_sr, spi2.sr.read().txe().bit());
    }

    loop {
        for spl_n in 0..100 {
            let spl = spl_n * ((i16::MAX - i16::MAX / 2) / 100);
            let l = spl;
            let r = spl;

            unsafe {
                let spi2 = &(*pac::SPI2::ptr());
                while !spi2.sr.read().txe().bit() {}
                spi2.dr.modify(|_, w| w.dr().bits(l as u16));
                // i2s_sr_check();
                // while !spi2.sr.read().txe().bit() {}
                // spi2.dr.modify(|_,w| w.dr().bits((l & 0x00FF) as u16));
                // i2s_sr_check();
                while !spi2.sr.read().txe().bit() {}
                spi2.dr.modify(|_, w| w.dr().bits(r as u16));
                // i2s_sr_check();
                // while !spi2.sr.read().txe().bit() {}
                // spi2.dr.modify(|_,w| w.dr().bits((r & 0x00FF) as u16));
                // i2s_sr_check();
            }
        }
        // your code goes here
    }
}

#[interrupt]
fn SPI2() {
    static mut COUNT: i16 = 0;
    const COUNT_MAX: i16 = 20;
    unsafe {
        let spi2 = &(*pac::SPI2::ptr());
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
            let spl = *COUNT * ((i16::MAX - i16::MAX / 2) / 20);

            spi2.dr.modify(|_, w| w.dr().bits(spl as u16));
            if side {
                *COUNT += 1;
                if *COUNT == COUNT_MAX {
                    *COUNT = 0;
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
