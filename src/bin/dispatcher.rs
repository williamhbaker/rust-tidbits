use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::HeaderMap;
use serde_json::json;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DispatchError {
    #[error("client failed to post")]
    PostFailed(#[from] reqwest::Error),
    #[error("failed to send on dispatcher")]
    SendFailed,
    #[error("failed to flush dispatcher")]
    FlushFailed,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert("key", "val".parse().unwrap());

    let url = std::env::var("TEST_URL").unwrap();

    let client = ReqwestClient::new(headers, url.parse().unwrap());

    let dispatch = Dispatcher::new(5, client, |count| println!("did it {}", count));

    for idx in 0..20 {
        dispatch.post(json!({ "hello": idx })).await.unwrap();
        println!("sent {}", idx);
    }

    dispatch.flush().await.unwrap();

    Ok(())
}

#[async_trait]
trait Client {
    async fn post(&self, body: serde_json::Value) -> Result<(), DispatchError>;
}

struct ReqwestClient {
    builder: reqwest::RequestBuilder,
}

impl ReqwestClient {
    pub fn new(headers: HeaderMap, url: url::Url) -> Self {
        let c = reqwest::Client::builder().build().unwrap();

        ReqwestClient {
            builder: c.post(url).headers(headers),
        }
    }
}

#[async_trait]
impl Client for ReqwestClient {
    async fn post(&self, body: serde_json::Value) -> Result<(), DispatchError> {
        self.builder.try_clone().unwrap().json(&body).send().await?;
        Ok(())
    }
}

struct Dispatcher {
    tx: mpsc::Sender<serde_json::Value>,
    consumer: tokio::task::JoinHandle<()>,
}

impl Dispatcher {
    pub fn new<T, F>(concurrency: usize, client: T, success: F) -> Self
    where
        T: Client + Send + Sync + 'static,
        F: Fn(usize) + Send + Sync + 'static,
    {
        let (tx_body, rx_body): (
            mpsc::Sender<serde_json::Value>,
            mpsc::Receiver<serde_json::Value>,
        ) = mpsc::channel(1);

        let consumer = tokio::spawn(Self::new_consumer(concurrency, rx_body, client, success));

        Dispatcher {
            tx: tx_body,
            consumer,
        }
    }

    async fn new_consumer<T, F>(
        concurrency: usize,
        rx: mpsc::Receiver<serde_json::Value>,
        client: T,
        success: F,
    ) where
        T: Client + Send + Sync + 'static,
        F: Fn(usize),
    {
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
            .map(|val| client.post(val))
            .buffer_unordered(concurrency);

        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(res) = stream.next().await {
            match res {
                Ok(_) => {
                    success(count);
                    count += 1;
                }
                Err(e) => println!("had error: {}", e),
            }
        }
    }

    async fn post(&self, body: serde_json::Value) -> Result<(), DispatchError> {
        self.tx
            .send(body)
            .await
            .map_err(|_| DispatchError::SendFailed)?;

        Ok(())
    }

    async fn flush(self) -> Result<(), DispatchError> {
        drop(self.tx);
        self.consumer
            .await
            .map_err(|_| DispatchError::FlushFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        sync::{Arc, Mutex},
    };

    use super::*;

    struct MockClient {
        calls: Arc<Mutex<RefCell<Vec<serde_json::Value>>>>,
    }

    #[async_trait]
    impl Client for MockClient {
        async fn post(&self, body: serde_json::Value) -> Result<(), DispatchError> {
            self.calls.lock().unwrap().borrow_mut().push(body);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_dispatcher() {
        let calls = Arc::new(Mutex::new(RefCell::new(Vec::new())));

        let client = MockClient {
            calls: calls.clone(),
        };
        let dispatch = Dispatcher::new(3, client, |_| {});

        let mut want_calls = vec![];

        for idx in 0..20 {
            let body = serde_json::json!({ "count": idx });
            dispatch.post(body.clone()).await.unwrap();
            want_calls.push(body);
        }

        dispatch.flush().await.unwrap();

        assert_eq!(want_calls, calls.lock().unwrap().clone().into_inner());
    }
}
