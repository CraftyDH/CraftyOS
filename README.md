# CraftyOS

## To run steps
### 1. OVMF   
    Grab OVMF-pure-efi.fd and place it in the OVMF folder with that **exact** name. 
    You can get this file from https://github.com/rust-osdev/ovmf-prebuilt

### 2. Run
    Execute cargo run in the root folder. This will automatically build the kernel and bootloader into a fat img then run qemu.
```sh
cargo run
```