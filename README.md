# Rust SDK for The Open Network

The tonlib-rs project provides a Rust SDK to interact with The Open Network (TON). It offers both a low-level client and a high-level API, making it easier for developers to integrate TON blockchain functionality into their Rust applications.

## Modules

* ```tonlib-client```
    A Rust client for interacting directly with the TON blockchain, offering core functionality and network access. ([Details](./client/README.md))
* ```tonlib-core```
     A collection of methods and structures providing a higher-level API built on top of tonlib-client, simplifying common blockchain interactions. ([Details](./clcoreient/README.md))


## Features

* Seamless integration with TON blockchain using native Rust bindings.
* Built-in support for static and shared libraries via tonlib-sys, which handles the lower-level bindings to the TON library.

## Prerequisites

To build the project, ensure you have the following tools installed:

For Linux:
```shell
sudo apt install build-essential cmake libsodium-dev libsecp256k1-dev lz4 liblz4-dev
```

For macOS:
```shell
brew install readline secp256k1 ccache pkgconfig cmake libsodium
```



## Getting Started

Add the library to your Rust project by including the following in your Cargo.toml:
```toml
[dependencies]
tonlib-core = "version"
tonlib-client = "version"
```

Replace "version" with the latest release version from [Crates.io].

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests. Refer to the contributing guidelines for more information.


## License

This project is licensed under the [MIT License](./LICENSE). See the LICENSE file for details.