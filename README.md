# CraftyOS
This is a project for learning OS development.
Currently the base for this is [Phil Opp's](https://os.phil-opp.com/) great rust OS tutorial.

You can checkout the blog for this project at https://craftydh.github.io/CraftyOS-Blog/.

# Running
## Install prerequisites
1. Install rust from http://rustup.rs
2. Install rust nightly with ```rustup toolchain install nightly```
3. Install cargo bootimage tool with ```cargo install bootimage``` 
4. Install qemu from https://www.qemu.org/download/

## Running
Run ```cargo run```
For release mode ```cargo run --release```

To check PCI devices uncomment this line ```139 // spawn_thread(|| get_pci_devices());```

For testing the mulitasking uncomment to spawn_thread closure under "Perform A|B|C|D" on line 145.

### Disk tests
To read / write from a disk uncomment ```142 // spawn_thread(|| ata_disk_task());```

Next create a disk called data.img with ```qemu-img create data.img 128M``` and uncomment line 33 in cargo.toml.

For reading set rw to true at line 87.
To write set it to false and choose a string to write on line 93.

## Running the tests
Run ```cargo test```
