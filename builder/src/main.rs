extern crate anyhow;
extern crate cargo_metadata;
extern crate fatfs;
extern crate fscommon;

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::process::{Command, Stdio};

use cargo_metadata::{Message, camino};

use fatfs::{format_volume, FileSystem, FormatVolumeOptions, FsOptions, StdIoWrapper};
use fscommon::BufStream;

fn main() -> Result<()> {
    // Compile all the stuff
    let bootloaderpath = buildcargo("bootloader")?;
    let kernelpath = buildcargo("kernel")?;

    // Open and format the virtual fat disk
    let crafty_img = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open("CraftyOS.img")?;

    // 10 MB
    crafty_img.set_len(10 * 1024 * 1024)?;
    format_volume(
        &mut StdIoWrapper::from(&crafty_img),
        FormatVolumeOptions::new().volume_label(*b"CraftyOS..."),
    )?;

    // Open root partition
    let partition = FileSystem::new(
        BufStream::new(&crafty_img),
        FsOptions::new().update_accessed_date(true),
    )?;
    let root = partition.root_dir();

    // Create basic directories
    root.create_dir("/EFI")?;
    root.create_dir("/EFI/BOOT")?;

    // Copy all the files
    let mut file = root.create_file("/EFI/BOOT/BootX64.EFI")?;
    io::copy(&mut File::open(bootloaderpath)?, &mut file)?;

    let mut file = root.create_file("/kernel.elf")?;
    io::copy(&mut File::open(kernelpath)?, &mut file)?;

    let mut file = root.create_file("/startup.nsh")?;
    io::copy(&mut File::open("resources/startup.nsh")?, &mut file)?;

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
            "if=pflash,format=raw,file=OVMF/OVMF-pure-efi.fd",
            "-drive",
            "format=raw,file=CraftyOS.img",
        ])
        .spawn()
        .unwrap();

    Ok(())
}

fn buildcargo(name: &str) -> Result<camino::Utf8PathBuf> {
    // Cargo build args
    let mut command = Command::new("cargo")
        .current_dir(name)
        .args(&["build", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Grab stdout and read it
    let reader = BufReader::new(command.stdout.take().unwrap());
    for message in Message::parse_stream(reader) {
        match message.unwrap() {
            // Pass messages straight back
            Message::CompilerMessage(msg) => println!("{:?}", msg),

            Message::CompilerArtifact(artifact) if artifact.target.name.as_str() == name => {
                return Ok(artifact
                    .executable
                    .context("Invalid artifact executable path")?);
            }
            Message::BuildFinished(finished) => {
                // Ensure successfull build
                if !finished.success {
                    return Err(anyhow!("Failed build of: {}", name));
                }
            }
            _ => (), // Unknown message
        }
    }
    // Function should have returned allready with the path
    return Err(anyhow!("No Artifact found for: {}", name));
}
