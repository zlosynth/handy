#![no_main]
#![no_std]

use handy_firmware as _; // Global logger and panicking behavior.

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use core::mem::MaybeUninit;

    use fugit::ExtU64;
    use heapless::spsc::{Consumer, Producer, Queue};
    use systick_monotonic::Systick;

    use handy_firmware::audio::{AudioInterface, SAMPLE_RATE};
    use handy_firmware::control_input::ControlInputInterface;
    use handy_firmware::control_input::ControlInputSnapshot;
    use handy_firmware::control_output::ControlOutputInterface;
    use handy_firmware::queue_utils;
    use handy_firmware::random_generator::RandomGenerator;
    use handy_firmware::startup_sequence;
    use handy_firmware::system::System;

    struct Dsp {}
    struct DspAttributes {}
    struct Controller {}

    #[link_section = ".sram"]
    static mut MEMORY: [MaybeUninit<u32>; 96 * 1024] =
        unsafe { MaybeUninit::uninit().assume_init() };

    // 1 kHz granularity for task scheduling.
    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        audio_interface: AudioInterface,
        random_generator: RandomGenerator,
        control_input_interface: ControlInputInterface,
        control_output_interface: ControlOutputInterface,
        dsp: Dsp,
        controller: Controller,
        dsp_attributes_producer: Producer<'static, DspAttributes, 8>,
        dsp_attributes_consumer: Consumer<'static, DspAttributes, 8>,
        control_input_snapshot_producer: Producer<'static, ControlInputSnapshot, 8>,
        control_input_snapshot_consumer: Consumer<'static, ControlInputSnapshot, 8>,
    }

    #[init(
        local = [
            dsp_attributes_queue: Queue<DspAttributes, 8> = Queue::new(),
            input_snapshot_queue: Queue<ControlInputSnapshot, 8> = Queue::new(),
        ]
    )]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Starting the firmware, initializing resources");

        if cfg!(feature = "idle-measuring") {
            cx.core.DCB.enable_trace();
            cx.core.DWT.enable_cycle_counter();
        }

        let (dsp_attributes_producer, dsp_attributes_consumer) =
            cx.local.dsp_attributes_queue.split();
        let (control_input_snapshot_producer, control_input_snapshot_consumer) =
            cx.local.input_snapshot_queue.split();

        let system = System::init(cx.core, cx.device);
        let mono = system.mono;
        let mut random_generator = system.random_generator;
        let mut audio_interface = system.audio_interface;
        let mut control_input_interface = system.control_input_interface;
        let control_output_interface = system.control_output_interface;

        startup_sequence::warm_up_control_input(&mut control_input_interface);
        // let controller = Controller::new(seed, save);
        // let mut stack_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        // let dsp = Dsp::new(SAMPLE_RATE as f32, &mut stack_manager);
        let controller = Controller {};
        let dsp = Dsp {};

        defmt::info!("Spawning tasks");

        audio_interface.spawn();
        control_loop::spawn().unwrap();
        input_collection_loop::spawn().unwrap();

        (
            Shared {},
            Local {
                audio_interface,
                random_generator,
                control_input_interface,
                control_output_interface,
                dsp,
                controller,
                dsp_attributes_producer,
                dsp_attributes_consumer,
                control_input_snapshot_producer,
                control_input_snapshot_consumer,
            },
            init::Monotonics(mono),
        )
    }

    #[task(
        local = [
            control_input_interface,
            control_input_snapshot_producer,
        ],
        priority = 2,
    )]
    fn input_collection_loop(cx: input_collection_loop::Context) {
        let control_input_interface = cx.local.control_input_interface;
        let control_input_snapshot_producer = cx.local.control_input_snapshot_producer;

        control_input_interface.sample();

        let _ = control_input_snapshot_producer.enqueue(control_input_interface.snapshot());

        // NOTE: This must be timed at the end. Otherwise, this task may get an interrupt
        // during sampling, which would then follow by immediate second execution of this
        // task, not giving enough time for the probe signal to propagate.
        input_collection_loop::spawn_after(1.millis()).ok().unwrap();
    }

    #[task(
        local = [
            controller,
            control_output_interface,
            dsp_attributes_producer,
            control_input_snapshot_consumer,
        ],
        priority = 3,
    )]
    fn control_loop(cx: control_loop::Context) {
        control_loop::spawn_after(1.millis()).ok().unwrap();

        let controller = cx.local.controller;
        let control_output_interface = cx.local.control_output_interface;
        let dsp_attributes_producer = cx.local.dsp_attributes_producer;
        let control_input_snapshot_consumer = cx.local.control_input_snapshot_consumer;

        queue_utils::warn_about_capacity("input_snapshot", control_input_snapshot_consumer);

        if let Some(snapshot) = queue_utils::dequeue_last(control_input_snapshot_consumer) {
            // let result = controller.apply_input_snapshot(snapshot);
            // let _ = dsp_attributes_producer.enqueue(result.dsp_attributes);
        }

        // let desired_output_state = controller.tick();
        // control_output_interface.set_state(&desired_output_state);
    }

    #[task(
        binds = DMA1_STR1,
        local = [
            audio_interface,
            random_generator,
            dsp,
            dsp_attributes_consumer,
        ],
        priority = 4,
    )]
    fn dsp_loop(cx: dsp_loop::Context) {
        let audio_interface = cx.local.audio_interface;
        // let random_generator = cx.local.random_generator;
        // let dsp = cx.local.dsp;
        let dsp_attributes_consumer = cx.local.dsp_attributes_consumer;

        queue_utils::warn_about_capacity("dsp_attributes", dsp_attributes_consumer);

        if let Some(attributes) = queue_utils::dequeue_last(dsp_attributes_consumer) {
            // dsp.set_attributes(attributes);
        }

        audio_interface.update_buffer(|buffer| {
            // dsp.process(buffer, random_generator);
        });
    }

    #[idle(local = [idling: u32 = 0, start: u32 = 0])]
    fn idle(cx: idle::Context) -> ! {
        if cfg!(feature = "idle-measuring") {
            use core::sync::atomic::{self, Ordering};
            use daisy::pac::DWT;

            const USECOND: u32 = 480;
            const TIME_LIMIT: u32 = USECOND * 10_000; // 0.01 second

            defmt::info!("Idle measuring is enabled");

            let idling: &'static mut u32 = cx.local.idling;
            let start: &'static mut u32 = cx.local.start;

            atomic::compiler_fence(Ordering::Acquire);
            *start = DWT::cycle_count();

            loop {
                cortex_m::interrupt::free(|_cs| {
                    cortex_m::asm::delay(USECOND);
                    *idling += USECOND;
                });

                if *idling >= TIME_LIMIT {
                    let now = DWT::cycle_count();
                    atomic::compiler_fence(Ordering::Release);

                    let elapsed = calculate_elapsed_dwt_ticks(now, start);

                    #[allow(clippy::cast_precision_loss)]
                    let idling_relative = *idling as f32 / elapsed as f32;
                    log_idle_time(idling_relative);

                    atomic::compiler_fence(Ordering::Acquire);
                    *start = DWT::cycle_count();
                    *idling = 0;
                }
            }
        } else {
            loop {
                cortex_m::asm::nop();
            }
        }
    }

    fn calculate_elapsed_dwt_ticks(now: u32, start: &mut u32) -> u32 {
        if now >= *start {
            now - *start
        } else {
            now + (u32::MAX - *start)
        }
    }

    fn log_idle_time(idling_relative: f32) {
        const IDLE_LIMIT: f32 = 0.99;
        let idling_percent = idling_relative * 100.0;
        if idling_relative < IDLE_LIMIT {
            defmt::warn!("Idle time={}% is below the limit", idling_percent);
        } else {
            defmt::debug!("Idle time={}%", idling_percent);
        }
    }
}
