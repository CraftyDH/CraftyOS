extern crate anyhow;
extern crate fatfs;
extern crate fscommon;

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{self, BufReader};
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

    let partition = FileSystem::new(
        BufStream::new(&crafty_img),
        FsOptions::new().update_accessed_date(true),
    )?;
    let root = partition.root_dir();
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

    root.create_dir("/EFI")?;
    root.create_dir("/EFI/BOOT")?;

    let names = ["../bootloader", "../kernel"];

    for name in names {
        let mut command = Command::new("cargo")
            .current_dir(name)
            .args(&["build", "--message-format=json-render-diagnostics"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let reader = BufReader::new(command.stdout.take().unwrap());
        for message in Message::parse_stream(reader) {
            match message.unwrap() {
                Message::CompilerMessage(msg) => {
                    // Pass messages straight back
                    println!("{:?}", msg);
                }
                Message::CompilerArtifact(artifact) => match artifact.target.name.as_str() {
                    "kernel" => {
                        let mut file = partition.root_dir().create_file("/kernel.elf")?;
                        io::copy(&mut File::open(&artifact.executable.unwrap())?, &mut file)?;
                    }
                    "bootloader" => {
                        let mut file = partition.root_dir().create_file("/EFI/BOOT/BootX64.efi")?;
                        io::copy(&mut File::open(&artifact.executable.unwrap())?, &mut file)?;
                    }
                    _ => (),
                },
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

    let mut file = partition.root_dir().create_file("/startup.nsh")?;
    io::copy(&mut File::open("startup.nsh")?, &mut file)?;

    // Launch qemu
    let _command = Command::new("qemu-system-x86_64")
        .args([
            "-nodefaults",
            "-vga",
            "qxl",
            "-cpu",
            "qemu64",
            "-m",
            "128M",
            "-drive",
            "if=pflash,format=raw,file=../OVMF/OVMF-pure-efi.fd",
            "-drive",
            "format=raw,file=CraftyOS.img",
        ])
        .spawn()
        .unwrap();

    Ok(())
}
