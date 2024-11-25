# Rust SDK for The Open Network

Rust SDK for [The Open Network](https://ton.org/)

## Features

* Support parsing and generation of Cell and BagOfCell for more convenient interaction with data structures
* Support of existing Wallet versions
* Derive wallet address
* Support of TON Mnemonics
* NaCl-compatible Ed25519 signing of transactions

## Usage

To use this library in your Rust application, add the following to your Cargo.toml file:

```toml
[dependencies]
tonlib-core = "version"
```

Then, in your Rust code, you can import the library with:

```rust
use tonlib_core;
```

## Package contents 

### Cell

Data structures and helpers for building and parsing Cell and Bag of Cells. See the documentation on [ton.org ](https://docs.ton.org/develop/data-formats/cell-boc)for details.

### Message

Data structures, builders, and parsers for Message 
See the documentation on [ton.org ](https://docs.ton.org/develop/smart-contracts/messages)for details.

Includes standard messages for Jetton, NFT, and Soulbound NFT, specified by [TON Enhancement Proposal](https://github.com/ton-blockchain/TEPs/blob/master/text/0001-tep-lifecycle.md).

### Mnemonic

Data structure to store mnemonic.

### Types

Data structures for storage and easy conversion of [Ton Smart-contract Address](https://docs.ton.org/learn/overviews/addresses) and [Ton Transaction Id](https://docs.ton.org/develop/data-formats/transaction-layout#transaction)


### Wallet 

Data structure for deriving wallet addresses.

## Usage examples

### Cell

Creating a `Cell` and writing data to it:

``` rust
use anyhow::anyhow;
use tonlib_core::TonAddress;
use tonlib_core::cell::CellBuilder;

fn write_cell() -> anyhow::Result<()> {
let mut writer = CellBuilder::new();
let addr = TonAddress::from_base64_url("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?;
let cell = writer
    .store_u32(32, 0xFAD45AADu32)?
    .store_bit(true)?
    .store_u8(8, 234u8)?
    .store_slice(&[0xFA, 0xD4, 0x5A, 0xAD, 0xAA, 0x12, 0xFF, 0x45])?
    .store_address(&addr)?
    .store_string("Hello, TON")?
    .build()?;
    # Ok(())
}
```

 Reading data from a `Cell`:

```rust
use tonlib_core::cell::Cell;
fn read_cell(cell: Cell) -> anyhow::Result<()> {
    let mut reader = cell.parser();
    let u32_value = reader.load_u32(32)?;
    let bit_value = reader.load_bit()?;
    let u8_value = reader.load_u8(8)?;
    let bytes_value = reader.load_bytes(8)?;
    let address_value = reader.load_address()?;
    let str_value = reader.ensure_empty()?;
    Ok(())
}
```