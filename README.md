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

### Build library

You can build the library using the following command:

```bash
cargo build
```

## Usage

To use this library in your Rust application, add the following to your Cargo.toml file:

```toml
[dependencies]
tonlib = "0.5"
```

Then, in your Rust code, you can import the library with:

```rust
use tonlib;
```

### Cell

Creating a `Cell` and writing data to it:

```rust
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
```

Reading data from a `Cell`:

```rust
let mut reader = cell.parser();
let u32_value = reader.load_u32(32)?;
let bit_value = reader.load_bit()?;
let u8_value = reader.load_u8(8)?;
let bytes_value = reader.load_bytes(8)?;
let address_value = reader.load_address()?;
let str_value = reader.load_string(reader.remaining_bytes())?;
```

### TON blockchain client

To call methods, create a client:

```rust
TonClient::set_log_verbosity_level(2); //setup of logging level
let client = TonClientBuilder::new()
  .with_pool_size(10)
  .with_keystore_dir(String::from("/tmp"))
  .build()
  .await?;
```

`TonClient::set_log_verbosity_level(2);` sets the logging level.

By default, the connection is made to mainnet. But you can also specify a test network when creating the client:

```rust
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
```

After creating the client, you can call methods on the TON blockchain:

```rust
let info: BlocksMasterchainInfo = client.get_masterchain_info().await?;
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
            .get_raw_transactions_v2(addr.to_hex().as_str(), &id, 1, false)
            .await?;
        println!("Tx: {:?}", tx.transactions[0])
    }
}
```

You can get the account state for any contract:

```rust
let r = client
        .get_account_state("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")
        .await;
```

### Working with contracts and jettons

Methods for working with tokens and wallets:

```rust
let master_contract = TonContract::new(
    &client,
    &"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?,
);
let jetton_data = master_contract.get_jetton_data().await?;

let wallet_contract = TonContract::new(
    &client,
    &"EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c".parse()?,
);
let wallet_data = wallet_contract.get_wallet_data().await?;
```

To load the metadata of the token, there is `JettonContentLoader`:

```rust
let loader = JettonContentLoader::default()?;
let content_res = loader.load(&jetton_data.content).await?;
```

Get the wallet address for the token:

```rust
let contract = TonContract::new(
    &client,
    &"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?,
);
let owner_address = TonAddress::from_base64_url(
    "EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg",
)?;
let wallet_address = contract.get_wallet_address(&owner_address).await?;
```

### Send message to TON

Create key pair from secret phrase (mnemonic)

```rust
let mnemonic_str = "<your secret phrase composed of 24 words>";
let mnemonic: Mnemonic = Mnemonic::from_str(mnemonic_str, &None).unwrap();
let key_pair: KeyPair = mnemonic.to_key_pair().unwrap();
```
And now you are ready to send transfer messages to TON blockchain.

Create a jetton transfer:

```rust
const SELF_ADDR: TonAddress = "<wallet address>"
    .parse()
    .unwrap();
const JETTON_MASTER_ADDR: TonAddress = "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86"
    .parse()
    .unwrap();

let client = TonClient::default().await?;
let jetton_master = TonContract::new(&client, &JETTON_MASTER_ADDR);
let self_jetton_wallet_addr = jetton_master.get_wallet_address(&SELF_ADDR).await?;

let wallet = TonWallet::derive(0, WalletVersion::V4R2, &key_pair)?;
let dest: TonAddress = "<destination wallet address>".parse()?;
let jetton_amount = BigUint::from(1000000u64);
let jetton_transfer = JettonTransferBuilder::new(&dest, &jetton_amount)
    .with_query_id(100500)
    .with_response_destination(&SELF_ADDR)
    .build()?;
let ton_amount = BigUint::from(200000000u64); // 0.2 TON
let transfer = TransferBuilder::new(self_wallet_addr, &ton_amount)
    .with_data(jetton_transfer)
    .build()?;
let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_secs()? as u32;
let body = wallet.create_external_body(now + 60, SEQNO, transfer)?;
let signed = wallet.sign_external_body(&body)?;
let wrapped = wallet.wrap_signed_body(signed)?;
let boc = BagOfCells::from_root(wrapped);
let tx = boc.serialize(true)?;

let hash = client.send_raw_message_return_hash(tx.as_slice()).await?;
```

Create a simple transfer:

```rust
let client = TonClient::default().await?;
let wallet = TonWallet::derive(0, WalletVersion::V4R2, &key_pair)?;
let dest: TonAddress = "<destination wallet address>".parse()?;
let value = BigUint::from(10000000u64); // 0.01 TON
let transfer = TransferBuilder::new(&dest, &value).build()?;
let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_secs()? as u32;
let body = wallet.create_external_body(now + 60, SEQNO, transfer)?;
let signed = wallet.sign_external_body(&body)?;
let wrapped = wallet.wrap_signed_body(signed)?;
let boc = BagOfCells::from_root(wrapped);
let tx = boc.serialize(true)?;
let hash = client.send_raw_message_return_hash(tx.as_slice()).await?;
```

## Contributing

If you want to contribute to this library, please feel free to open a pull request on GitHub.

## License
This library is licensed under the MIT license. See the LICENSE file for details.
