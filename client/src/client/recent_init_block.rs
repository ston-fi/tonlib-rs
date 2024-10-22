use anyhow::bail;
use futures::future::join_all;
use ton_liteapi::tl::response::BlockData;
use tonlib_core::cell::BagOfCells;
use tonlib_core::constants::{MASTERCHAIN_ID, SHARD_FULL};

use crate::client::recent_init_block::lite::Connection;
use crate::config::LiteEndpoint;
use crate::tl::BlockIdExt;

const BLOCK_INFO_TAG: u32 = 0x9bc7a987;

pub(crate) async fn get_recent_init_block(endpoints: &[LiteEndpoint]) -> Option<BlockIdExt> {
    log::info!("Trying to update init_block...");
    let keyblocks_f = endpoints
        .iter()
        .map(|endpoint| get_last_keyblock(endpoint.clone()));

    let keyblocks_res = join_all(keyblocks_f).await;

    // just log errors
    for (pos, res) in keyblocks_res.iter().enumerate() {
        if let Err(err) = res {
            log::warn!(
                "Failed to get recent init block from node with ip: {}, err: {}",
                endpoints[pos].ip,
                err,
            );
        }
    }
    // each endpoint may return error, but we need only 1 successful result - so ignore errors
    keyblocks_res
        .into_iter()
        .flatten()
        .max_by_key(|block| block.seqno)
}

async fn get_last_keyblock(endpoint: LiteEndpoint) -> anyhow::Result<BlockIdExt> {
    let mut conn = Connection::new(endpoint)?;
    let mc_info = conn.get_mc_info().await?;
    let block = conn.get_block(mc_info.last).await?;
    let seqno = parse_key_block_seqno(&block)?;
    let header = conn.get_mc_header(seqno).await?;

    let key_block_id_lite = header.id;
    let block_id = BlockIdExt {
        workchain: MASTERCHAIN_ID,
        shard: SHARD_FULL as i64,
        seqno: seqno as i32,
        root_hash: key_block_id_lite.root_hash.0.to_vec(),
        file_hash: key_block_id_lite.file_hash.0.to_vec(),
    };
    Ok(block_id)
}

fn parse_key_block_seqno(block: &BlockData) -> anyhow::Result<u32> {
    let boc = BagOfCells::parse(&block.data)?;
    let root = boc.single_root()?;
    let block_info = root.reference(0)?;

    let mut parser = block_info.parser();

    let tag = parser.load_u32(32)?;
    if tag != BLOCK_INFO_TAG {
        bail!("Invalid tag: {}, expected: {}", tag, BLOCK_INFO_TAG);
    }
    // version(32), merge_info(8), flags(8), seqno(32), vert_seqno(32), shard(104), utime(32), start/end lt(128),
    // validator_list_hash(32), catchain_seqno(32), min_ref_mc_seqno(32)
    parser.skip_bits(32 + 8 + 8 + 32 + 32 + 104 + 32 + 128 + 32 + 32 + 32)?;
    let key_block_seqno = parser.load_u32(32)?;
    Ok(key_block_seqno)
}

mod lite {
    use std::error::Error;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::time::Duration;

    use adnl::AdnlPeer;
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use tokio::net::TcpStream;
    use tokio::time::timeout;
    use tokio_tower::multiplex::Client;
    use ton_liteapi::layers::{WrapMessagesLayer, WrapService};
    use ton_liteapi::peer::LitePeer;
    use ton_liteapi::tl::adnl::Message;
    use ton_liteapi::tl::common::BlockIdExt as BlockIdExtLite;
    use ton_liteapi::tl::request::{
        GetBlock, LookupBlock, Request, WaitMasterchainSeqno, WrappedRequest,
    };
    use ton_liteapi::tl::response::{BlockData, BlockHeader, MasterchainInfo, Response};
    use ton_liteapi::types::LiteError;
    use tonlib_core::constants::{MASTERCHAIN_ID, SHARD_FULL};
    use tower::{Service, ServiceBuilder, ServiceExt};

    use crate::config::LiteEndpoint;

    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
    const REQ_TIMEOUT: Duration = Duration::from_secs(10);

    type ConnService =
        WrapService<Client<LitePeer<AdnlPeer<TcpStream>>, Box<dyn Error + Sync + Send>, Message>>;

    pub(super) struct Connection {
        public: Vec<u8>,
        addr: SocketAddrV4,
        service: Option<ConnService>,
    }

    impl Connection {
        pub(super) fn new(endpoint: LiteEndpoint) -> anyhow::Result<Self> {
            let LiteEndpoint { ip, port, id } = endpoint;
            let ip_addr = Ipv4Addr::from(ip as u32);
            let public = BASE64_STANDARD.decode(id.key)?;
            let addr = SocketAddrV4::new(ip_addr, port);
            let conn = Self {
                public,
                addr,
                service: None,
            };
            Ok(conn)
        }

        pub(super) async fn get_block(
            &mut self,
            block_id: BlockIdExtLite,
        ) -> anyhow::Result<BlockData> {
            let req = WrappedRequest {
                wait_masterchain_seqno: Some(WaitMasterchainSeqno {
                    seqno: block_id.seqno,
                    timeout_ms: REQ_TIMEOUT.as_millis() as u32,
                }),
                request: Request::GetBlock(GetBlock { id: block_id }),
            };
            match self.execute(req).await? {
                Response::BlockData(block) => Ok(block),
                _ => Err(LiteError::UnexpectedMessage)?,
            }
        }

        pub(super) async fn get_mc_header(&mut self, seqno: u32) -> anyhow::Result<BlockHeader> {
            let req = WrappedRequest {
                wait_masterchain_seqno: None,
                request: Request::LookupBlock(LookupBlock {
                    mode: (),
                    id: ton_liteapi::tl::common::BlockId {
                        workchain: MASTERCHAIN_ID,
                        shard: SHARD_FULL,
                        seqno,
                    },
                    seqno: Some(()),
                    lt: None,
                    utime: None,
                    with_state_update: None,
                    with_value_flow: None,
                    with_extra: None,
                    with_shard_hashes: None,
                    with_prev_blk_signatures: None,
                }),
            };
            match self.execute(req).await? {
                Response::BlockHeader(header) => Ok(header),
                _ => Err(LiteError::UnexpectedMessage)?,
            }
        }

        pub(super) async fn get_mc_info(&mut self) -> anyhow::Result<MasterchainInfo> {
            let req = WrappedRequest {
                wait_masterchain_seqno: None,
                request: Request::GetMasterchainInfo,
            };
            match self.execute(req).await? {
                Response::MasterchainInfo(info) => Ok(info),
                _ => Err(LiteError::UnexpectedMessage)?,
            }
        }

        async fn execute(&mut self, req: WrappedRequest) -> anyhow::Result<Response> {
            let ready_service = self.connect().await?.ready().await?;
            Ok(timeout(REQ_TIMEOUT, ready_service.call(req)).await??)
        }

        async fn connect(&mut self) -> anyhow::Result<&mut ConnService> {
            if self.service.is_none() {
                let adnl = timeout(
                    CONNECTION_TIMEOUT,
                    AdnlPeer::connect(&self.public, self.addr),
                )
                .await??;

                let lite = LitePeer::new(adnl);
                let service = ServiceBuilder::new()
                    .layer(WrapMessagesLayer)
                    .service(Client::<_, Box<dyn Error + Send + Sync + 'static>, _>::new(
                        lite,
                    ));
                self.service = Some(service);
            }
            Ok(self.service.as_mut().unwrap()) // unwrap is safe: we initialized it in branch above
        }
    }
}
