#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use bbqueue::{
    BBQueue,
    nicknames::Texas,
    prod_cons::stream::{StreamConsumer, StreamProducer},
    traits::notifier::maitake::MaiNotSpsc,
};
use core::sync::atomic::AtomicU8;
use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const BUFFER_SIZE: usize = 1024;

type BBQUEUE = &'static BBQueue<
    bbqueue::traits::storage::Inline<BUFFER_SIZE>,
    bbqueue::traits::coordination::cas::AtomicCoord,
    MaiNotSpsc,
>;
static PRODUCER: StaticCell<StreamProducer<BBQUEUE>> = StaticCell::new();
static CONSUMER: StaticCell<StreamConsumer<BBQUEUE>> = StaticCell::new();
static QUEUE: StaticCell<Texas<BUFFER_SIZE, MaiNotSpsc>> = StaticCell::new();

static ATOMIC_COUNTER: AtomicU8 = AtomicU8::new(u8::MIN);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = embassy_rp::config::Config::default();
    let _p: embassy_rp::Peripherals = embassy_rp::init(config);

    let bb: &'static Texas<BUFFER_SIZE, MaiNotSpsc> = QUEUE.init(Texas::new());

    let prod: &'static StreamProducer<BBQUEUE> = PRODUCER.init(bb.stream_producer());
    let cons: &'static StreamConsumer<BBQUEUE> = CONSUMER.init(bb.stream_consumer());
    spawner.spawn(unwrap!(read(cons, Duration::from_secs(5))));
    spawner.spawn(unwrap!(write(prod, Duration::from_secs(1))));
}

#[embassy_executor::task]
async fn read(consumer: &'static StreamConsumer<BBQUEUE>, delay: Duration) {
    loop {
        if let data = consumer.wait_read().await {
            let len = data.len();
            info!("Read nr of bytes {}", len);
            for i in 0..len {
                info!("Read value {}", data[i]);
            }
            data.release(len);
        }
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

        info!("Write to queue {}", value);
        ticker.next().await;
    }
}
