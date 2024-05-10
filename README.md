# Rust SDK for The Open Network

Rust SDK for [The Open Network](https://ton.org/)

## Features

* Rust SDK for The Open Network
* Using `tonlibjson` as data provider
* Support parsing and generation of Cells methods for more convenient interaction with data structures
* Support of Wallet versions (3, 3 revision 2, 4 revision 2)
* Derive wallet address
* Support of TON Mnemonics
* NaCL-compatible Ed25519 signing of transactions
* Support jetton functions: getting of jetton data and wallet address for jetton
* Support internal and external jetton metadata loading
* Connection pooling & retries support for better server-level interaction
* Support of IPFS jetton metadata

## Dependencies

`tonlib-sys` - https://github.com/ston-fi/tonlib-sys

For macOS must be preinstalled next components:
```shell
brew install --cask mactex
brew install readline secp256k1 ccache pkgconfig cmake libsodium
```

### Build library

You can build the library using the following command:

```bash
cargo build
```

## Usage

To use this library in your Rust application, add the following to your Cargo.toml file:

```toml
[dependencies]
tonlib = "0.14"
```

Then, in your Rust code, you can import the library with:

```rust
use tonlib;
```

### Cell

Creating a `Cell` and writing data to it:

``` rust
use anyhow::anyhow;
use tonlib::address::TonAddress;
use tonlib::cell::CellBuilder;

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
use tonlib::cell::Cell;
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

### TON blockchain client

To call methods, create a client:

```rust

use tonlib::client::TonClient;
use tonlib::client::TonClientBuilder;
async fn create_client()-> anyhow::Result<()>{
    TonClient::set_log_verbosity_level(2); //setup of logging level
    let client = TonClientBuilder::new()
    .with_pool_size(10)
    .with_keystore_dir(String::from("/tmp"))
    .build()
    .await?;
Ok(())
}
```



`TonClient::set_log_verbosity_level(2);` sets the logging level.

By default, the connection is made to mainnet. But you can also specify a test network when creating the client:

```rust
use tonlib::config::TESTNET_CONFIG;
use tonlib::client::TonConnectionParams;
use tonlib::client::TonClientBuilder;
async fn create_client_with_conn_params()-> anyhow::Result<()>{
    let client = TonClientBuilder::new()
        .with_connection_params(&TonConnectionParams {
            config: TESTNET_CONFIG.to_string(),
            blockchain_name: None,
            use_callbacks_for_network: false,
            ignore_cache: false,
            keystore_dir: None,
        })
        .with_pool_size(10)
        .build()
        .await?;
    Ok(())
}
```


After creating the client, you can call methods on the TON blockchain:

```rust
use tonlib::address::TonAddress;
use tonlib::tl::InternalTransactionId;
use tonlib::tl::NULL_BLOCKS_ACCOUNT_TRANSACTION_ID;
use tonlib::tl::BlocksTransactions;
use tonlib::tl::BlocksShards;
use tonlib::tl::BlockId;
use tonlib::tl::BlocksMasterchainInfo;
use tonlib::client::TonClient;
use tonlib::client::TonClientInterface;

async fn call_blockchain_methods()-> anyhow::Result<()>{
    let client = TonClient::builder().build().await?;
    let (_, info) = client.get_masterchain_info().await?;
    println!("MasterchainInfo: {:?}", &info);
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno,
    };
    let block_id_ext = client.lookup_block(1, &block_id, 0, 0).await?;
    println!("BlockIdExt: {:?}", &block_id_ext);
    let block_shards: BlocksShards = client.get_block_shards(&info.last).await?;
    let mut shards = block_shards.shards.clone();
    println!("Shards: {:?}", &block_shards);
    shards.insert(0, info.last.clone());
    for shard in &shards {
        println!("Processing shard: {:?}", shard);
        let workchain = shard.workchain;
        let txs: BlocksTransactions = client
            .get_block_transactions(&shard, 7, 1024, &NULL_BLOCKS_ACCOUNT_TRANSACTION_ID)
            .await?;
        println!(
            "Number of transactions: {}, incomplete: {}",
            txs.transactions.len(),
            txs.incomplete
        );
        for tx_id in txs.transactions {
            let mut t: [u8; 32] = [0; 32];
            t.clone_from_slice(tx_id.account.as_slice());
            let addr = TonAddress::new(workchain, &t);
            let id = InternalTransactionId {
                hash: tx_id.hash.clone(),
                lt: tx_id.lt,
            };
            let tx = client
                .get_raw_transactions_v2(&addr, &id, 1, false)
                .await?;
            println!("Tx: {:?}", tx.transactions[0])
        }
    }
    Ok(())
}
```

You can get the account state for any contract:

```rust
use tonlib::address::TonAddress;
use tonlib::client::TonClient;
use crate::tonlib::client::TonClientInterface;

async fn get_state()-> anyhow::Result<()>{  
    let client = TonClient::builder().build().await?;
    let address = TonAddress::from_base64_url(
        "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt-",
    )?;
    let r = client
            .get_account_state(&address)
            .await;
    Ok(())
}
```



### Working with contracts and jettons

Methods for working with tokens and wallets:

``` rust
use tonlib::client::TonClient;
use tonlib::contract::TonContractFactory;
use crate::tonlib::contract::JettonMasterContract;
use crate::tonlib::contract::JettonWalletContract;

async fn method_call() -> anyhow::Result<()> { 
    let client = TonClient::builder().build().await?;
    let contract_factory = TonContractFactory::builder(&client).build().await?;
    let master_contract = contract_factory.get_contract(
        &"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?,
    );
    let jetton_data = master_contract.get_jetton_data().await?;

    let wallet_contract = contract_factory.get_contract(
        &"EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c".parse()?,
    );
    let wallet_data = wallet_contract.get_wallet_data().await?;
    Ok(())
}
```

To load the metadata of the token, one may use generic `MetaLoader` and it type aliases: `JettonMetaLoader, NftItemMetaLoader NftColletionMetaLoader`:

```rust
use tonlib::client::TonClient;
use tonlib::contract::TonContractFactory;
use tonlib::contract::JettonMasterContract;
use tonlib::meta::JettonMetaLoader;
use tonlib::meta::LoadMeta;

async fn load_meta() -> anyhow::Result<()> { 
    let client = TonClient::builder().build().await?;
    let contract_factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        contract_factory.get_contract(&"EQB-MPwrd1G6WKNkLz_VnV6WqBDd142KMQv-g1O-8QUA3728".parse()?); 
    let jetton_data = contract.get_jetton_data().await?;
    let loader = JettonMetaLoader::default()?;
    let content_res = loader.load(&jetton_data.content).await?;

Ok(())
}
```


Get the wallet address for the token:

```rust
use tonlib::address::TonAddress;
use tonlib::client::TonClient;
use tonlib::contract::TonContractFactory;
use tonlib::contract::JettonMasterContract; 

async fn get_wallet_address() -> anyhow::Result<()> {

    let client = TonClient::default().await?;
    let contract_factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        contract_factory.get_contract(&"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?,
    );
    let owner_address = TonAddress::from_base64_url(
        "EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg",
    )?;
    let wallet_address = contract.get_wallet_address(&owner_address).await?;
    Ok(())
}
```

### Send message to TON

Create key pair from secret phrase ( )

```rust
use tonlib::mnemonic::Mnemonic;
use tonlib::mnemonic::KeyPair;
async fn create_key_pair() -> anyhow::Result<()> {
    let mnemonic = Mnemonic::new(
        vec![
            "dose", "ice", "enrich", "trigger", "test", "dove", "century", "still", "betray",
            "gas", "diet", "dune",
        ],
        &None,
        )?;
    let key_pair = mnemonic.to_key_pair();
    Ok(())
}

```
And now you are ready to send transfer messages to TON blockchain.

Create a jetton transfer:

```rust


use num_bigint::BigUint;
use std::time::SystemTime;

use tonlib::address::TonAddress;
use tonlib::cell::BagOfCells;
use tonlib::client::TonClient;
use tonlib::client::TonClientInterface;
use tonlib::contract::TonContractFactory;
use tonlib::contract::JettonMasterContract;
use tonlib::message::JettonTransferMessage;

use tonlib::message::TransferMessage;
use tonlib::mnemonic::KeyPair;
use tonlib::mnemonic::Mnemonic;
use tonlib::wallet::TonWallet;
use tonlib::wallet::WalletVersion;

async fn create_jetton_transfer() -> anyhow::Result<()> {

    let seqno:i32 = 30000000;

    let self_address: TonAddress = "EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg "
        .parse()
        .unwrap();
    let mnemonic_str = "mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic mnemonic";
    let mnemonic: Mnemonic = Mnemonic::from_str(mnemonic_str, &None).unwrap();
    let key_pair: KeyPair = mnemonic.to_key_pair().unwrap();
    let jetton_master_address: TonAddress = "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86"
        .parse()
        .unwrap();

    let client = TonClient::default().await?;
        let contract_factory = TonContractFactory::builder(&client).build().await?;
    let jetton_master =
        contract_factory.get_contract(&jetton_master_address);
    let self_jetton_wallet_addr = jetton_master.get_wallet_address(&self_address).await?;
    let wallet = TonWallet::derive_default(WalletVersion::V4R2, &key_pair)?;
    let dest: TonAddress = "<destination wallet address>".parse()?;
    let src: TonAddress = "<source wallet address>".parse()?;
    let jetton_amount = BigUint::from(1000000u64);
    let jetton_transfer = JettonTransferMessage::new(&dest, &jetton_amount)
        .with_query_id(100500)
        .with_response_destination(&self_address)
        .build()?;
    let ton_amount = BigUint::from(200000000u64); // 0.2 TON
    let transfer = TransferMessage::new(&src, &ton_amount)
        .with_data(jetton_transfer)
        .build()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as u32;
    let body = wallet.create_external_body(now + 60, seqno.try_into().unwrap(), vec![transfer])?;
    let signed = wallet.sign_external_body(&body)?;
    let wrapped = wallet.wrap_signed_body(signed)?;
    let boc = BagOfCells::from_root(wrapped);
    let tx = boc.serialize(true)?;

    let hash = client.send_raw_message_return_hash(tx.as_slice()).await?;

    Ok(())
}
```

Create a simple transfer:

```rust

use anyhow::anyhow;
use num_bigint::BigUint;
use std::time::SystemTime;

use tonlib::address::TonAddress;
use tonlib::cell::BagOfCells;
use tonlib::message::TransferMessage;
use tonlib::wallet::TonWallet;
use tonlib::client::TonClient;
use tonlib::client::TonClientInterface;
use tonlib::mnemonic::KeyPair;
use tonlib::mnemonic::Mnemonic;
use tonlib::wallet::WalletVersion;


async fn create_simple_transfer() -> anyhow::Result<()> {
    let mnemonic = Mnemonic::new(
        vec![
            "dose", "ice", "enrich", "trigger", "test", "dove", "century", "still", "betray",
            "gas", "diet", "dune",
        ],
        &None,
        )?;
    let key_pair = mnemonic.to_key_pair()?;
    let seqno =  30000000;
    

    let client = TonClient::default().await?;
    let wallet = TonWallet::derive_default(WalletVersion::V4R2, &key_pair)?;
    let dest: TonAddress = "<destination wallet address>".parse()?;
    let value = BigUint::from(10000000u64); // 0.01 TON
    let transfer = TransferMessage::new(&dest, &value).build()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as u32;
    let body = wallet.create_external_body(now + 60, seqno, vec![transfer])?;
    let signed = wallet.sign_external_body(&body)?;
    let wrapped = wallet.wrap_signed_body(signed)?;
    let boc = BagOfCells::from_root(wrapped);
    let tx = boc.serialize(true)?;
    let hash = client.send_raw_message_return_hash(tx.as_slice()).await?;

    Ok(())
}
```

## Cross-compilation
In order to cross-compile for specific cpu microachitecture set environment variable `TARGET_CPU_MARCH` to the required. Supported values are listen in https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html

## Contributing

If you want to contribute to this library, please feel free to open a pull request on GitHub.

## License
This library is licensed under the MIT license. See the LICENSE file for details. -->
