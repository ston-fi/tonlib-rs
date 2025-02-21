### v0.19.3

* Resolve be-1426 "use lite to adjust init block"
* Ni: add-get-account-state-by-transaction
* Impl #be-1317: serialize_dict
* Impl #be-1412: partialeq for invalidmesage

### v0.20.0

* Impl #be-1604: update_recent_init_block on tonlib client init
* Impl #be-1441: improve dict load/store

### v0.20.1

* Fix dependency pinning

### v0.20.2

* Impl #be-1619: technical release

### v0.20.3

* Impl #be-1464: read dict key TonHash in BE format

### v0.21.0

### v0.21.1

* Impl #BE-1662: Limited get raw txs
* Expose emulator_set_verbosity_level, enabling ability to keep application log dry (#115)
* Impl #BE-1620: More strict dictionary handling
* Impl #BE-1624: Support wallet v5r1
* Impl #BE-1660: made LiteServerInfo fields public
* Impl #BE-1386: Fix handling null instead of dict in TvmStackEntry
* Impl #BE-1625: moved tx hash mismatch handling in factory cache
* Impl #be-1742: fix read_collection_metadata_content for internal
* Impl #ni: update tvm integration tests
* Impl #ni: tvmemulator cleanup
* Bump tonlib-sys to 2024.10.2

### v0.22.2

* Impl #be-1430: config path via environment variable
* Impl #be-1708: added cmp for tonaddress
* Impl #be-1785: removed state_cache feature
* Impl #be-1431: impl tryfrom<internaltransactionid> for tontxid
* Impl #be-1429: tonhash converted to struct
* Ni: added into_single_root method
* Ni: bump thiserror dependency version
* Iml #be-1761: fix memory leak in emulator

### v0.22.1

* added From<TonHash> for [u8; TON_HASH_LEN]

### v0.22.2

* Bump tonlib-sys

### v0.22.3

* Impl #be-1820: rm unwrap from jettonmetadata conversion

### v0.22.4

* make pub TvmEmulatorUnsafe
### v0.23.0

* Impl #BE-1846: library cache
* Impl #BE-1881: Replaced dashmap by tokio::sync:Mutex to avoid mem leaks
* Impl #BE-1839: Boxed long errors, updated builder methods
* Impl #BE-1892: Display for TonHash
* Impl #BE-1893: Library provider trait and implementation of BlockChainLibraryProvider
* NI: bump tonlib-sys to 2024.10.4
### v0.23.1
* NI: Factory cache disabled by default
### v0.23.2
* Impl #be-1988: impl tlb-types and support anycast in tonaddress
* Implement load/store tonhash
### v0.24.1
* Impl #BE-1854: Factory cache disabled by default
* Impl #BE-2034: Support tlb StateInit 
* Impl #BE-2032: Support tl-b Message
* Impl #BE-1989: Support tl-b TonAddress
* Impl #BE-2088: Wallet v5 message  building