# lain

This project is a fork of the (seemingly unmaintained) [lain](https://github.com/landaire/lain), which
itself is a fork of the (deprecated) [lain from Microsoft](https://github.com/microsoft/lain).

This crate provides functionality one may find useful while developing a fuzzer. A recent nightly Rust
build is required for the specialization feature.

Please consider this crate in "beta" and subject to breaking changes for minor version releases for pre-1.0.

### Documentation

Please build the documentation locally with `cargo doc --open`.

### Installation

Lain requires rust nightly builds for specialization support.

Add the following to your Cargo.toml:

```toml
[dependencies]
lain = { git = "https://github.com/AFLplusplus/lain.git" }
```

You may wish to pin to a specific revision or tag.

### Example Usage

```rust
extern crate lain;

use lain::prelude::*;
use lain::rand;
use lain::hexdump;

#[derive(Debug, Mutatable, NewFuzzed, BinarySerialize)]
struct MyStruct {
    field_1: u8,

    #[lain(bits = 3)]
    field_2: u8,

    #[lain(bits = 5)]
    field_3: u8,

    #[lain(min = 5, max = 10000)]
    field_4: u32,

    #[lain(ignore)]
    ignored_field: u64,
}

fn main() {
    let mut mutator = Mutator::new(rand::thread_rng());

    let mut instance = MyStruct::new_fuzzed(&mut mutator, None);

    let mut serialized_data = Vec::with_capacity(instance.serialized_size());
    instance.binary_serialize::<_, BigEndian>(&mut serialized_data);

    println!("{:?}", instance);
    println!("hex representation:\n{}", hexdump(&serialized_data));

    // perform small mutations on the instance
    instance.mutate(&mut mutator, None);

    println!("{:?}", instance);
}

// Output:
//
// MyStruct { field_1: 95, field_2: 5, field_3: 14, field_4: 8383, ignored_field: 0 }
// hex representation:
// ------00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
// 0000: 5F 75 00 00 20 BF 00 00 00 00 00 00 00 00         _u...¿........
// MyStruct { field_1: 160, field_2: 5, field_3: 14, field_4: 8383, ignored_field: 0 }
```

A complete example of a fuzzer and its target can be found in the [examples](examples/)
directory. The server is written in C and takes data over a TCP socket, parses a message, and
mutates some state. The fuzzer has Rust definitions of the C data structure and will send fully
mutated messages to the server and utilizes the `Driver` object to manage fuzzer threads and
state.

## Contributing

This project welcomes contributions and suggestions.

License: MIT
