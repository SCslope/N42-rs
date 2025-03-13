use crate::unverifiedblock::UnverifiedBlock;
use jsonrpsee::{
    core::{RpcResult, SubscriptionResult},
    proc_macros::rpc,
    tokio,
    ws_client::WsClientBuilder,
    PendingSubscriptionSink, SubscriptionMessage, SubscriptionSink,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref MINEDBLOCK_INSTANCE: Arc<MinedblockExt> = Arc::new(MinedblockExt::new());
}

#[cfg_attr(not(test), rpc(server, namespace = "minedblockExt"))]
#[cfg_attr(test, rpc(server, client, namespace = "minedblockExt"))]
pub trait MinedblockExtApi {
    #[subscription(name = "subscribeMinedblock", item = UnverifiedBlock)]
    fn subscribe_minedblock(&self) -> SubscriptionResult;

    /// 发送区块数据
    #[method(name = "sendBlock")]
    fn send_block(&self, block: UnverifiedBlock) -> RpcResult<()>;
}
#[derive(Clone)]
pub struct MinedblockExt {
    // 添加一个channel来存储订阅者
    subscribers: Arc<Mutex<Vec<SubscriptionSink>>>,
}

impl MinedblockExt {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn instance() -> Arc<MinedblockExt> {
        MINEDBLOCK_INSTANCE.clone()
    }
}

impl MinedblockExtApiServer for MinedblockExt {
    fn subscribe_minedblock(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
        let subscribers = self.subscribers.clone();
        
        // 这里我们启动异步任务并立即返回
        tokio::spawn(async move {
            if let Ok(sink) = pending.accept().await {
                let mut subs = subscribers.lock().await;
                subs.push(sink);
            }
        });

        Ok(())  // 返回一个立即返回的 Result
    }

    // 使 `send_block` 不再是 `async fn`，而是同步的，执行异步任务时用 tokio::spawn
    fn send_block(&self, block: UnverifiedBlock) -> RpcResult<()> {
        let subscribers = self.subscribers.clone();

        // 异步处理订阅者发送消息
        tokio::spawn(async move {
            let mut subs = subscribers.lock().await;
            for mut sub in subs.iter_mut() {
                // 发送区块数据到订阅者
                let message = SubscriptionMessage::from_json(&block.clone()).unwrap();
                if let Err(e) = sub.send(message).await {
                    // 处理发送错误
                    println!("Error sending block to subscriber: {:?}", e);
                }
            }
        });


        Ok(()) // 同步返回 RpcResult
    }
}

impl MinedblockExtApiServer for Arc<MinedblockExt> {
    fn subscribe_minedblock(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
        let subscribers = self.subscribers.clone();
        
        tokio::spawn(async move {
            if let Ok(sink) = pending.accept().await {
                let mut subs = subscribers.lock().await;
                subs.push(sink);
            }
        });

        Ok(())
    }

    fn send_block(&self, block: UnverifiedBlock) -> RpcResult<()> {
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut subs = subscribers.lock().await;
            for mut sub in subs.iter_mut() {
                let message = SubscriptionMessage::from_json(&block.clone()).unwrap();
                if let Err(e) = sub.send(message).await {
                    println!("Error sending block to subscriber: {:?}", e);
                }
            }
        });

        Ok(())
    }
}


#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::unverifiedblock::UnverifiedBlock;
    use jsonrpsee::{server::ServerBuilder, ws_client::WsClientBuilder};
    use tokio::{sync::oneshot, time::{sleep, Duration}};

    async fn start_server() -> (std::net::SocketAddr, MinedblockExt) {
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        let api = MinedblockExt::new();
        let api_clone = api.clone();
        let server_handle = server.start(api.into_rpc());

        tokio::spawn(server_handle.stopped());

        (addr, api_clone)
    }

    async fn run_client(ws_url: String, client_id: u32, tx: oneshot::Sender<bool>) {
        let client = WsClientBuilder::default().build(&ws_url).await.unwrap();
        let mut subscription = MinedblockExtApiClient::subscribe_minedblock(&client)
            .await
            .unwrap();

        let mut received_blocks = 0;
        while let Some(block) = subscription.next().await {
            let block = block.unwrap();
            println!("Client {} received block: {:?}", client_id, block);
            received_blocks += 1;
            
            if received_blocks >= 10 {
                tx.send(true).unwrap();
                break;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_clients_receive_blocks() {
        // 启动服务器
        let (server_addr, api) = start_server().await;
        let ws_url = format!("ws://{}", server_addr);

        // 设置两个客户端的通道
        let (tx1, rx1) = oneshot::channel::<bool>();
        let (tx2, rx2) = oneshot::channel::<bool>();

        // 启动两个客户端
        let client1 = tokio::spawn(run_client(ws_url.clone(), 1, tx1));
        let client2 = tokio::spawn(run_client(ws_url.clone(), 2, tx2));

        // 等待一段时间确保客户端完成订阅
        sleep(Duration::from_secs(1)).await;

        // 服务端定时发送区块
        tokio::spawn(async move {
            for i in 0..10 {
                let block = UnverifiedBlock::default();
                api.send_block(block).unwrap();
                sleep(Duration::from_millis(100)).await;  // 添加短暂延迟
            }
        });

        // 等待两个客户端都接收到全部区块
        let (result1, result2) = tokio::join!(rx1, rx2);
        assert!(result1.unwrap(), "Client 1 did not receive all blocks");
        assert!(result2.unwrap(), "Client 2 did not receive all blocks");

        // 等待客户端任务完成
        let _ = tokio::join!(client1, client2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_client_subscription() {
        // 创建客户端连接到已存在的服务器
        let ws_url = "ws://127.0.0.1:8545".to_string();
        let (tx, rx) = oneshot::channel::<bool>();

        // 启动客户端
        let client = tokio::spawn(run_client(ws_url, 1, tx));

        // 等待接收数据
        let result = rx.await;
        assert!(result.is_ok(), "Client did not receive blocks");

        // 等待客户端任务完成
        let _ = client.await;
    }

}
