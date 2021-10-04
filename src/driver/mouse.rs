use core::{
    convert::TryInto,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use ps2_mouse::{Mouse, MouseState};
use x86_64::instructions::interrupts::{self, without_interrupts};

use crate::vga_buffer::{writer::WRITER, BUFFER_HEIGHT, BUFFER_WIDTH};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping mouse input!");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: mouse scancode queue uninialized!");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once!");
        Self { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("Mouse scancode queue not initialized");

        // Fast path
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub async fn print_mousemovements() {
    let mut mouse = Mouse::new();
    let mut scancodes = ScancodeStream::new();

    println!("Starting mouse handler...");

    // Init the mouse without interrupts
    if let Err(err) = without_interrupts(|| mouse.init()) {
        print!("Mouse failed to enable: {}", err);
        // return;
    }

    mouse.set_on_complete(mouse_packet_handler);

    while let Some(scancode) = scancodes.next().await {
        mouse.process_packet(scancode)
    }
    panic!("This shouldn't end")
}

fn mouse_packet_handler(mouse_state: MouseState) {
    static X: AtomicUsize = AtomicUsize::new(0);
    static Y: AtomicUsize = AtomicUsize::new(0);

    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let mut x = X.load(Ordering::Relaxed);
        let mut y = Y.load(Ordering::Relaxed);

        // Unflip bit for old cursor
        writer.flip_bit(x, y);

        if mouse_state.x_moved() {
            x = match (x as isize).checked_add(mouse_state.get_x().try_into().unwrap()) {
                Some(x) => {
                    let x: usize = x.try_into().unwrap_or(0);
                    if x > BUFFER_WIDTH - 1 {
                        BUFFER_WIDTH - 1
                    } else {
                        x
                    }
                }
                None => 0,
            };
        }

        if mouse_state.y_moved() {
            // Negative Y so that moving the mouse up and down feels normal and not inverted
            y = match (y as isize).checked_add((-mouse_state.get_y()).try_into().unwrap()) {
                Some(y) => {
                    let y: usize = y.try_into().unwrap_or(0);
                    if y > BUFFER_HEIGHT - 1 {
                        BUFFER_HEIGHT - 1
                    } else {
                        y
                    }
                }
                None => 0,
            };
        }

        X.store(x, Ordering::Relaxed);
        Y.store(y, Ordering::Relaxed);

        // Flip bit for new cursor
        writer.flip_bit(x, y);
    });
}
