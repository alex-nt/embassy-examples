#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

use bbqueue::{
    BBQueue,
    nicknames::SiuMei,
    prod_cons::stream::{StreamConsumer, StreamProducer},
    traits::notifier::maitake::MaiNotSpsc,
    traits::storage::BoxedSlice,
};
use core::{slice, sync::atomic::AtomicU8};
use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker, Timer};
use embedded_alloc::LlffHeap as Heap;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const BUFFER_SIZE: usize = 1024;

type BBQUEUE = &'static BBQueue<BoxedSlice, bbqueue::traits::coordination::cs::CsCoord, MaiNotSpsc>;
static PRODUCER: StaticCell<StreamProducer<BBQUEUE>> = StaticCell::new();
static CONSUMER: StaticCell<StreamConsumer<BBQUEUE>> = StaticCell::new();
static QUEUE: StaticCell<SiuMei<MaiNotSpsc>> = StaticCell::new();

static ATOMIC_COUNTER: AtomicU8 = AtomicU8::new(u8::MIN);

#[global_allocator]
static HEAP: Heap = Heap::empty();

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

    let _: &mut [u8] = unsafe {
        let psram_ptr = psram.base_address();
        let slice: &'static mut [u8] = slice::from_raw_parts_mut(psram_ptr, psram.size() as usize);
        HEAP.init(slice.as_ptr() as usize, psram.size());
        slice
    };

    let bb: &'static SiuMei<MaiNotSpsc> =
        QUEUE.init(SiuMei::new_with_storage(BoxedSlice::new(BUFFER_SIZE)));

    let prod: &'static StreamProducer<BBQUEUE> = PRODUCER.init(bb.stream_producer());
    let cons: &'static StreamConsumer<BBQUEUE> = CONSUMER.init(bb.stream_consumer());
    spawner.spawn(unwrap!(read(cons)));
    spawner.spawn(unwrap!(write(prod, Duration::from_secs(1))));
}

#[embassy_executor::task]
async fn read(consumer: &'static StreamConsumer<BBQUEUE>) {
    loop {
        let data = consumer.wait_read().await;
        let len = data.len();
        info!("Read nr of bytes {}", len);
        for i in 0..len {
            info!("Read value {}", data[i]);
        }
        data.release(len);
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
