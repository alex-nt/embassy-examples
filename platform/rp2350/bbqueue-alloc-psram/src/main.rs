#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use bbqueue::{
    BBQueue,
    nicknames::{Churrasco},
    prod_cons::stream::{StreamConsumer, StreamProducer},
    traits::storage::{Storage},
};
use core::{slice, sync::atomic::AtomicU8};
use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const BUFFER_SIZE: usize = 1024;

type BBQUEUE = &'static BBQueue<
    bbqueue::traits::storage::Inline<BUFFER_SIZE>,
    bbqueue::traits::coordination::cas::AtomicCoord,
    bbqueue::traits::notifier::polling::Polling,
>;
static PRODUCER: StaticCell<StreamProducer<BBQUEUE>> = StaticCell::new();
static CONSUMER: StaticCell<StreamConsumer<BBQUEUE>> = StaticCell::new();
static QUEUE: StaticCell<Churrasco<BUFFER_SIZE>> = StaticCell::new();

static ATOMIC_COUNTER: AtomicU8 = AtomicU8::new(u8::MIN);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = embassy_rp::config::Config::default();
    let p = embassy_rp::init(config);
    let psram_config = embassy_rp::psram::Config::aps6404l();
    let psram = embassy_rp::psram::Psram::new(
        embassy_rp::qmi_cs1::QmiCs1::new(p.QMI_CS1, p.PIN_47),
        psram_config,
    );

    let Ok(psram) = psram else {
        error!("PSRAM not found");
        loop {
            Timer::after_secs(1).await;
        }
    };

    let psram_slice: &mut [u8] = unsafe {
        let psram_ptr = psram.base_address();
        let slice: &'static mut [u8] = slice::from_raw_parts_mut(psram_ptr, psram.size() as usize);
        slice
    };

    let bb: &'static Churrasco<BUFFER_SIZE> = QUEUE.init(Churrasco::new());

    let mut prod: &'static StreamProducer<BBQUEUE> = PRODUCER.init(bb.stream_producer());
    let mut cons: &'static StreamConsumer<BBQUEUE> = CONSUMER.init(bb.stream_consumer());
    spawner.spawn(unwrap!(read(cons, Duration::from_secs(5))));
    spawner.spawn(unwrap!(write(prod, Duration::from_secs(1))));
}

#[embassy_executor::task]
async fn read(consumer: &'static StreamConsumer<BBQUEUE>, delay: Duration) {
    let mut ticker = Ticker::every(delay);
    loop {
        if let Ok(rgr) = consumer.read() {
            let len = rgr.len();
            info!("Read from PSRAM {}", len);
            for i in 0..len {
                info!("Read value {}", rgr[i]);
            }
            rgr.release(len);
        }
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn write(producer: &'static StreamProducer<BBQUEUE>, delay: Duration) {
    let mut ticker = Ticker::every(delay);
    loop {
        let mut wgr = producer.grant_exact(1).unwrap();
        let value: u8 = ATOMIC_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        wgr[0] = value;
        wgr.commit(1);

        info!("Write to PSRAM {}", value);
        ticker.next().await;
    }
}
