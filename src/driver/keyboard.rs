use core::{
    convert::TryInto,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{fmt::format, format, string::String, vec::Vec};
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use pc_keyboard::{layouts::Us104Key, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};

use crate::{
    disk::{ata_identify, read_screen, write_screen},
    pci::get_pci_devices,
    vga_buffer::{
        colour::{Colour, ColourCode},
        writer, BUFFER_HEIGHT, BUFFER_WIDTH,
    },
};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input!");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: keyboard scancode queue uninialized!");
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
            .expect("Keyboard scancode queue not initialized");

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

fn handle_arg_int(
    arg1: &mut String,
    key: DecodedKey,
    current_str: &str,
    success_str: &str,
) -> bool {
    match key {
        DecodedKey::Unicode(num) if num.is_digit(10) => {
            arg1.push(num);
            writer::WRITER.lock().write_first_line(
                format!("{}{}", current_str, arg1).as_str(),
                ColourCode::from_fg(Colour::Yellow),
            );
            false
        }
        // Backspace
        DecodedKey::Unicode('\x08') => {
            arg1.pop();
            writer::WRITER.lock().write_first_line(
                format!("{}{}", current_str, arg1).as_str(),
                ColourCode::from_fg(Colour::Yellow),
            );

            false
        }
        DecodedKey::Unicode('\n') if arg1.len() != 0 => {
            writer::WRITER.lock().write_first_line(
                format!("{}{}{}", current_str, arg1, success_str).as_str(),
                ColourCode::from_fg(Colour::Yellow),
            );
            true
        }
        _ => false,
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(Us104Key, ScancodeSet1, HandleControl::Ignore);

    println!("Starting keyboard handler...");
    writer::WRITER
        .lock()
        .write_first_line("Ready For Command: ", ColourCode::from_fg(Colour::Green));

    // Is alt pressed
    let mut alt: bool = false;
    let mut task = None;
    let mut stage = 0;
    let mut arg1 = String::new();
    let mut arg2 = String::new();

    let write_status = |str| {
        writer::WRITER
            .lock()
            .write_first_line(str, ColourCode::from_fg(Colour::Yellow))
    };

    // This loop should never return
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                // Handle ALT code
                if alt {
                    // Do we have a complex task
                    if let Some(task_str) = task {
                        match task_str {
                            "read" => {
                                if stage == 0 {
                                    if handle_arg_int(
                                        &mut arg1,
                                        key,
                                        "Alt: Read from disk: ",
                                        ", section: ",
                                    ) {
                                        stage = 1
                                    }
                                } else if stage == 1 {
                                    if handle_arg_int(
                                        &mut arg2,
                                        key,
                                        format!("Alt: Read from disk: {}, section: ", arg1)
                                            .as_str(),
                                        "",
                                    ) {
                                        writer::WRITER.lock().write_first_line(
                                            format!(
                                                "Success read from disk: {}, section: {} :)",
                                                arg1, arg2
                                            )
                                            .as_str(),
                                            ColourCode::from_fg(Colour::Green),
                                        );
                                        cursor!(0, 1);
                                        read_screen(
                                            arg1.parse::<u8>().expect("Unknown drive"),
                                            arg2.parse::<u32>().expect("Unknown sector"),
                                        );
                                        // Set cursor back to the top
                                        cursor!(0, 1);

                                        stage = 0;
                                        task = None;
                                        alt = false;
                                        arg1.clear();
                                        arg2.clear();
                                    }
                                }
                            }
                            "write" => {
                                if stage == 0 {
                                    if handle_arg_int(
                                        &mut arg1,
                                        key,
                                        "Alt: Write to disk: ",
                                        ", section: ",
                                    ) {
                                        stage = 1
                                    }
                                } else {
                                    if handle_arg_int(
                                        &mut arg2,
                                        key,
                                        format!("Alt: Write to disk: {}, section: ", arg1).as_str(),
                                        "",
                                    ) {
                                        writer::WRITER.lock().write_first_line(
                                            format!(
                                                "Success wrote to disk: {}, section: {} :)",
                                                arg1, arg2
                                            )
                                            .as_str(),
                                            ColourCode::from_fg(Colour::Green),
                                        );
                                        let screen = writer::WRITER.lock().dump_screen();

                                        write_screen(
                                            arg1.parse::<u8>().expect("Unknown drive"),
                                            arg2.parse::<u32>().expect("Unknown sector"),
                                            screen,
                                        );

                                        stage = 0;
                                        alt = false;
                                        task = None;
                                        arg1.clear();
                                        arg2.clear();
                                    }
                                }
                            }
                            "colour" => {
                                if stage == 0 {
                                    if handle_arg_int(
                                        &mut arg1,
                                        key,
                                        "Alt: Change colour, fg: ",
                                        ", bg: ",
                                    ) {
                                        stage = 1;
                                    }
                                } else {
                                    if handle_arg_int(
                                        &mut arg2,
                                        key,
                                        format!("Alt: Change colour, fg: {}, bg: ", arg1).as_str(),
                                        "",
                                    ) {
                                        let fg = arg1
                                            .parse::<usize>()
                                            .expect("Error with converting arg1 to usize...");
                                        let bg = arg2
                                            .parse::<usize>()
                                            .expect("Error with converting arg2 to usize...");

                                        if fg > 15 {
                                            writer::WRITER.lock().write_first_line(
                                                "Failed to change colour, fg was greater than 15 :(", 
                                                ColourCode::from_fg(Colour::LightRed)
                                            );
                                        } else if bg > 15 {
                                            writer::WRITER.lock().write_first_line(
                                                "Failed to change colour, bg was greater than 15 :(",
                                                 ColourCode::from_fg(Colour::LightRed)
                                            );
                                        } else {
                                            writer::WRITER.lock().write_first_line(
                                                "Successfully changed colour :)",
                                                ColourCode::from_fg(Colour::Green),
                                            );

                                            let new_colour = ColourCode::from_u8(
                                                fg.try_into().unwrap(),
                                                bg.try_into().unwrap(),
                                            );

                                            writer::WRITER.lock().set_colour(new_colour);
                                        }

                                        writer::WRITER.lock().fill_screen();
                                        cursor!(0, 1);

                                        // Reset
                                        stage = 0;
                                        alt = false;
                                        task = None;
                                        arg1.clear();
                                        arg2.clear();
                                    }
                                }
                            }
                            _ => {}
                        };
                    } else {
                        match key {
                            DecodedKey::Unicode('h') => {
                                writer::WRITER.lock().write_first_line(
                                    "Success: displayed help :)",
                                    ColourCode::from_fg(Colour::Green),
                                );
                                writer::WRITER.lock().fill_screen();
                                cursor!(0, 1);
                                println!("Alt key HELP \n\nr: Read from disk\nw: Write to disk\nd: Show disks\np: List out PCI devices\nx: Clear screen\nc: Change display colour");
                                alt = false;
                            }
                            DecodedKey::Unicode('r') => {
                                write_status("Alt: Read from disk: ");
                                task = Some("read");
                            }
                            DecodedKey::Unicode('w') => {
                                write_status("Alt: Write to disk: ");
                                task = Some("write");
                            }
                            DecodedKey::Unicode('x') => {
                                writer::WRITER.lock().fill_screen();
                                writer::WRITER.lock().write_first_line(
                                    "Success: cleared screen :)",
                                    ColourCode::from_fg(Colour::Green),
                                );
                                // Set cursor to top of page
                                cursor!(0, 1);

                                alt = false;
                            }
                            DecodedKey::Unicode('c') => {
                                write_status("Alt: Change colour, fg: ");
                                writer::WRITER.lock().fill_screen();

                                // Set cursor to top of page
                                cursor!(0, 1);
                                // Print the colour options
                                println!("Colour options...\n0: Black\n1: Blue\n2: Green\n3: Cyan\n4: Red\n5: Magenta\n6: Brown\n7: LightGray\n8: DarkGray\n9: LightBlue\n10: LightGreen\n11: LightCyan\n12: LightRed\n13: Pink\n14: Yellow\n15: White");
                                task = Some("colour");
                            }
                            DecodedKey::Unicode('d') => {
                                writer::WRITER.lock().fill_screen();
                                writer::WRITER.lock().write_first_line(
                                    "Success: displayed all the ATA disks :)",
                                    ColourCode::from_fg(Colour::Green),
                                );
                                // Set cursor to top of page
                                cursor!(0, 1);
                                println!("ATA Disks found...\n");
                                alt = false;
                                ata_identify();
                            }
                            DecodedKey::Unicode('p') => {
                                writer::WRITER.lock().fill_screen();
                                writer::WRITER.lock().write_first_line(
                                    "Success: displayed all the PCI devices :)",
                                    ColourCode::from_fg(Colour::Green),
                                );
                                // Set cursor to top of page
                                cursor!(0, 1);
                                println!("PCI Devices...\n");
                                alt = false;
                                get_pci_devices();
                            }
                            // Ignore RawKey
                            _ => {
                                writer::WRITER.lock().write_first_line(
                                    "Unknown Alt code :(",
                                    ColourCode::from_fg(Colour::LightRed),
                                );
                                alt = false
                            }
                        }
                    }
                } else {
                    match key {
                        DecodedKey::Unicode(character) => print!("{}", character),
                        DecodedKey::RawKey(KeyCode::AltLeft | KeyCode::AltRight) => {
                            alt = true;
                            write_status("Alt: ");
                        }

                        // Cursor movement around screen
                        DecodedKey::RawKey(KeyCode::ArrowDown) => {
                            let (x, y) = writer::WRITER.lock().get_pos();
                            // Check that the user doesn't go off screen
                            if (y + 1) < BUFFER_HEIGHT {
                                writer::WRITER.lock().set_pos(x, y + 1);
                            }
                        }
                        DecodedKey::RawKey(KeyCode::ArrowUp) => {
                            let (x, y) = writer::WRITER.lock().get_pos();
                            // Check that the user doesn't go off screen
                            if y > 1 {
                                writer::WRITER.lock().set_pos(x, y - 1);
                            }
                        }
                        DecodedKey::RawKey(KeyCode::ArrowLeft) => {
                            let (x, y) = writer::WRITER.lock().get_pos();
                            // Check that the user doesn't go off screen
                            if x > 0 {
                                writer::WRITER.lock().set_pos(x - 1, y);
                            }
                        }
                        DecodedKey::RawKey(KeyCode::ArrowRight) => {
                            let (x, y) = writer::WRITER.lock().get_pos();
                            // Check that the user doesn't go off screen
                            if (x + 1) < BUFFER_WIDTH {
                                writer::WRITER.lock().set_pos(x + 1, y);
                            }
                        }
                        DecodedKey::RawKey(key) => print!("{:?}", key),
                    }
                }
            }
        }
    }
}
