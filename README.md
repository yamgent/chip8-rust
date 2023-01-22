# chip8-rust

My Rust implementation of chip-8.

## Usage

```sh
$ cargo run -- --path <program>
```

## Details

Implementation is complete (following the "specification" from https://tobiasvl.github.io/blog/write-a-chip-8-emulator/). However, it may not be bug-free, so it may have some issues with some programs (may or may not be due to the ambiguous instructions).

Ran it on:

- IBM Logo
- https://github.com/corax89/chip8-test-rom
- https://github.com/daniel5151/AC8E/blob/master/roms/bc_test.ch8
- http://mir3z.github.io/chip8-emu/ (not all programs can be run with our emulator)

There are several improvements and fixes that can be made to the codebase and the emulator itself. They are all listed under https://github.com/yamgent/chip8-rust/issues. No plans to resolve those issues for now.
