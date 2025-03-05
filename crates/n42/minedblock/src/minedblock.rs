use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use crate::unverifiedblock::UnverifiedBlock;
use n42_engine_types::{N42PayloadJob,PayloadBuilder};
/// trait interface for a custom rpc namespace: `minedblock`
///
/// This defines an additional namespace where all methods are configured as trait functions.
#[cfg_attr(not(test), rpc(server, namespace = "minedblockExt"))]
#[cfg_attr(test, rpc(server, client, namespace = "minedblockExt"))]
pub trait MinedBlockExtApi {
    /// send blocks to mobile
    #[method(name = "minedblock")]
    fn minedblock(&self)->RpcResult<UnverifiedBlock>;
}
/// The type that implements the `minedblock` rpc namespace trait
pub struct MinedBlockExt<Client, Pool, Consensus, Tasks, Builder> 
where
    Builder: PayloadBuilder<Pool, Client, Consensus>,
{
    pub n42plj:N42PayloadJob<Client, Pool, Consensus, Tasks, Builder>,
}
impl<Client, Pool, Consensus, Tasks, Builder> MinedBlockExtApiServer for MinedBlockExt<Client, Pool, Consensus, Tasks, Builder> 
where
    Builder: PayloadBuilder<Pool, Client, Consensus>+'static,
    Consensus: Sync+Send+'static,
    Client: Sync+Send+'static,
    Pool: Sync+Send+'static,
    Tasks: Sync+Send+'static,
{
    fn minedblock(&self)->RpcResult<UnverifiedBlock> {
        let block=self.n42plj.cached_reads.as_ref().unwrap().clone();
        let blocknumber=self.n42plj.config.parent_header.as_ref().header.number+1;
        let unverifiedblock=UnverifiedBlock{
            blocknumber,
            block,
        };
        Ok(unverifiedblock)
    }
}