extern crate anyhow;
extern crate fatfs;
extern crate fscommon;

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io;
use std::process::{Command, Stdio};

use cargo_metadata::Message;

use fatfs::{format_volume, FileSystem, FormatVolumeOptions, FsOptions, StdIoWrapper};
use fscommon::BufStream;

fn main() -> anyhow::Result<()> {
    let crafty_img = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("CraftyOS.img")?;

    crafty_img.set_len(10 * 1024 * 1024)?;

    format_volume(
        &mut StdIoWrapper::from(&crafty_img),
        FormatVolumeOptions::new().volume_label(*b"CraftyOS   "),
    )?;
    println!("Formated");

    let options = FsOptions::new().update_accessed_date(true);
    let partition = FileSystem::new(BufStream::new(&crafty_img), options)?;

    // let testdir = fs.root_dir().create_dir("test")?;

    // let mut file = partition.root_dir().create_file("hello.txt")?;

    //// ! Get building to work https://github.com/rust-lang/cargo/issues/9451
    // let mut command = Command::new("cargo")
    //     .args(&[
    //         "build",
    //         "--workspace",
    //         "--exclude",
    //         "builder",
    //         "--message-format=json-render-diagnostics",
    //     ])
    //     .stdout(Stdio::piped())
    //     .spawn()
    //     .unwrap();

    partition.root_dir().create_dir("/EFI")?;
    partition.root_dir().create_dir("/EFI/BOOT")?;

    let names = ["../bootloader"];

    for name in names {
        let mut command = Command::new("cargo")
            .current_dir(name)
            .args(&["build", "--message-format=json-render-diagnostics"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let reader = std::io::BufReader::new(command.stdout.take().unwrap());
        for message in cargo_metadata::Message::parse_stream(reader) {
            match message.unwrap() {
                Message::CompilerMessage(msg) => {
                    // Pass messages straight back
                    println!("{:?}", msg);
                }
                Message::CompilerArtifact(artifact) => {
                    match artifact.target.name.as_str() {
                        "kernel" => println!("Kernel!"),
                        "bootloader" => {
                            let mut file =
                                partition.root_dir().create_file("/EFI/BOOT/BootX64.efi")?;
                            io::copy(
                                &mut fs::File::open(&artifact.executable.unwrap())?,
                                &mut file,
                            )?;
                        }
                        _ => (),
                    }
                }
                // We don't care about these
                // Message::BuildScriptExecuted(script) => {
                //     println!("{:?}", script);
                // }
                Message::BuildFinished(finished) => {
                    println!("{:?}", finished);
                    if !finished.success {
                        return Err(anyhow!("Failed..."));
                    }
                }
                _ => (), // Unknown message
            }
        }
    }

    // Launch qemu
    let _command = Command::new("qemu-system-x86_64")
        .args([
            "-nodefaults",
            "-vga",
            "qxl",
            "-machine",
            "q35,accel=kvm:tcg",
            "-drive",
            "if=pflash,format=raw,file=../OVMF/OVMF-pure-efi.fd",
            "-drive",
            "format=raw,file=CraftyOS.img",
        ])
        .spawn()
        .unwrap();

    Ok(())
}
