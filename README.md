# BurritOS
BurritOs is a minimalist, Rust-written operating system project, following the well-known one of Oppermann.
It provides the basic mechanisms for interrupts (hardware, CPU exceptions), paging and mem allocation (see more at Oppermann's project webpage).
I shall add other custom features (filesystem, ...).

# Target Arch
The project, as said in [Oppermann's article](https://os.phil-opp.com/), may be compiled for a target architecture
with no underlying OS, and thus no C runtime (which is necessary since it should run on bare metal).
The rustc target architecture for this project is configured in `x86_64_arch.json`, but you can
essentially provide your own, as long as it follows the same rules.
You must consequently change the default rustc target in `.cargo/config`.

# Bootloader and Environment
The bootloader may be created thanks to the [`bootimage` crate](https://github.com/rust-osdev/bootimage) and run, with the entire kernel, in QEMU, as Oppermann's project suggests.
`bootimage` has to be installed via the command `cargo install bootimage` in your `home` dir.
The cargo config file is configured so that `cargo run` directly invokes `bootimage` and produces the bootimage, and runs it in QEMU. 
The bootloader, bundled with the kernel image, both linked in a compiled artefact, can be found as `target/name_of_target/bootimage-X.bin`.
The whole image may also be copied to a disk/USB drive: `dd if=target/x86_64_arch/debug/bootimage-burritos.bin of=/dev/sdX && sync` after compilation.
