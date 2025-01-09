# Rust client for The Open Network

Rust client for [The Open Network](https://ton.org/)

## Features

* Rust client for The Open Network
* Using `tonlibjson` as data provider
* Support jetton functions: getting of jetton data and wallet address for jetton
* Support internal and external jetton metadata loading
* Connection pooling & retries support for better server-level interaction
* Support of IPFS jetton metadata

### Feature flags
- `state_cache` - Enables caching of ton contract states. This feature is recommended to use if the contract state received from blockchain is reused multiple times. 
- `emulate_get_method` - Enables the usage of emulator to run get_methods locally. 
- `no_avx512` - Forces dependent tonlib-sys to be built without avx512 instruction set.
- `with_debug_info` - Enables debug information and stack-trace received from underlying  tonlibjson C++ code.


## Dependencies

`tonlib-sys` - https://github.com/ston-fi/tonlib-sys
`tonlib-core` - https://github.com/ston-fi/tonlib-rs

## Prerequisites

For Linux:
```shell
sudo apt install build-essential cmake libsodium-dev libsecp256k1-dev lz4 liblz4-dev
```

For macOS:
```shell
brew install readline secp256k1 ccache pkgconfig cmake libsodium
```


## Usage

To use this library in your Rust application, add the following to your Cargo.toml file:

```toml
[dependencies]
tonlib-client = "version"
```

Then, in your Rust code, you can import the library with:

```rust
use tonlib_client;
```


### TON blockchain client

To call methods, create a client:

```rust
use tonlib_client::client::TonClient;
use tonlib_client::client::TonClientBuilder;
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
use tonlib_client::config::TESTNET_CONFIG;
use tonlib_client::client::TonConnectionParams;
use tonlib_client::client::TonClientBuilder;
async fn create_client_with_conn_params()-> anyhow::Result<()>{
    let client = TonClientBuilder::new()
        .with_connection_params(&TonConnectionParams {
            config: TESTNET_CONFIG.to_string(),
            blockchain_name: None,
            use_callbacks_for_network: false,
            ignore_cache: false,
            keystore_dir: None,
            ..Default::default()
        })
        .with_pool_size(10)
        .build()
        .await?;
    Ok(())
}
```


After creating the client, you can call methods on the TON blockchain:

```rust
use tonlib_core::TonAddress;
use tonlib_client::tl::InternalTransactionId;
use tonlib_core::types::ZERO_HASH;
use tonlib_client::tl::NULL_BLOCKS_ACCOUNT_TRANSACTION_ID;
use tonlib_client::tl::BlocksTransactions;
use tonlib_client::tl::BlocksShards;
use tonlib_client::tl::BlockId;
use tonlib_client::tl::BlocksMasterchainInfo;
use tonlib_client::client::TonClient;
use tonlib_client::client::TonClientInterface;
use tonlib_core::TonHash;

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
            let t = TonHash::try_from(tx_id.account.as_slice())?;
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
use tonlib_core::TonAddress;
use tonlib_client::client::TonClient;
use crate::tonlib_client::client::TonClientInterface;

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
use tonlib_client::client::TonClient;
use tonlib_client::contract::TonContractFactory;
use tonlib_client::contract::JettonMasterContract;
use tonlib_client::contract::JettonWalletContract;

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
use tonlib_client::client::TonClient;
use tonlib_client::contract::TonContractFactory;
use tonlib_client::contract::JettonMasterContract;
use tonlib_client::meta::JettonMetaLoader;
use tonlib_client::meta::LoadMeta;

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
use tonlib_core::TonAddress;
use tonlib_client::client::TonClient;
use tonlib_client::contract::TonContractFactory;
use tonlib_client::contract::JettonMasterContract; 

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
use tonlib_core::mnemonic::Mnemonic;
use tonlib_core::mnemonic::KeyPair;
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
use std::sync::Arc;
use tonlib_core::TonAddress;
use tonlib_core::cell::BagOfCells;
use tonlib_client::client::TonClient;
use tonlib_client::client::TonClientInterface;
use tonlib_client::contract::TonContractFactory;
use tonlib_client::contract::JettonMasterContract;
use tonlib_core::message::JettonTransferMessage;
use tonlib_core::message::TransferMessage;
use tonlib_core::message::TonMessage;
use tonlib_core::message::HasOpcode;
use tonlib_core::mnemonic::KeyPair;
use tonlib_core::mnemonic::Mnemonic;
use tonlib_core::wallet::TonWallet;
use tonlib_core::wallet::WalletVersion;
use tonlib_core::message::CommonMsgInfo;
use tonlib_core::message::ExternalIncomingMessage;

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
    let external_msg_info = ExternalIncomingMessage{
        src,
        dest,
        import_fee: ton_amount,
    };
    let common_msg_info = CommonMsgInfo::ExternalIncomingMessage(external_msg_info);
    let transfer = TransferMessage::new(common_msg_info)
        .with_data(jetton_transfer.into())
        .build()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as u32;
    let body = wallet.create_external_body(now + 60, seqno.try_into().unwrap(), vec![Arc::new(transfer)])?;
    let signed = wallet.sign_external_body(&body)?;
    let wrapped = wallet.wrap_signed_body(signed, true)?;
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
use std::sync::Arc;

use tonlib_core::TonAddress;
use tonlib_core::cell::BagOfCells;
use tonlib_core::message::TransferMessage;
use tonlib_core::wallet::TonWallet;
use tonlib_client::client::TonClient;
use tonlib_client::client::TonClientInterface;
use tonlib_core::mnemonic::KeyPair;
use tonlib_core::mnemonic::Mnemonic;
use tonlib_core::wallet::WalletVersion;
use tonlib_core::message::TonMessage;
use tonlib_core::message::CommonMsgInfo;
use tonlib_core::message::ExternalIncomingMessage;

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
    let src: TonAddress = "<source wallet address>".parse()?;
    let dest: TonAddress = "<destination wallet address>".parse()?;
    let value = BigUint::from(10000000u64); // 0.01 TON
    let external_msg_info = ExternalIncomingMessage{
        src,
        dest,
        import_fee: value,
    };
    let common_msg_info = CommonMsgInfo::ExternalIncomingMessage(external_msg_info);
    let transfer = TransferMessage::new(common_msg_info).build()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as u32;
    let body = wallet.create_external_body(now + 60, seqno, vec![Arc::new(transfer)])?;
    let signed = wallet.sign_external_body(&body)?;
    let wrapped = wallet.wrap_signed_body(signed, true)?;
    let boc = BagOfCells::from_root(wrapped);
    let tx = boc.serialize(true)?;
    let hash = client.send_raw_message_return_hash(tx.as_slice()).await?;

    Ok(())
}
```

## Contributing

If you want to contribute to this library, please feel free to open a pull request on GitHub.

## License
This library is licensed under the MIT license. See the LICENSE file for details. -->
