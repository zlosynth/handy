use heapless::spsc::Consumer;

pub fn dequeue_last<T, const N: usize>(consumer: &mut Consumer<'static, T, N>) -> Option<T> {
    let mut last_item = None;
    while let Some(attributes) = consumer.dequeue() {
        last_item = Some(attributes);
    }
    last_item
}

pub fn warn_about_capacity<T, const N: usize>(name: &str, consumer: &mut Consumer<'static, T, N>) {
    if consumer.len() > consumer.capacity() / 2 {
        defmt::warn!(
            "Queue={:?} is above the half of its capacity {:?}/{:?}",
            name,
            consumer.len(),
            consumer.capacity()
        );
    }
}
