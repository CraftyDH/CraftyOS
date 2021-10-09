use crate::executor::{task::TaskPriority, Executor};

pub mod keyboard;
pub mod mouse;

pub extern "C" fn driver_task() {
    let mut executor = Executor::new();

    let spawner = executor.get_spawner();

    spawner.spawn(keyboard::print_keypresses(), TaskPriority::Interrupt);
    spawner.spawn(mouse::print_mousemovements(), TaskPriority::Interrupt);

    executor.run();
}
