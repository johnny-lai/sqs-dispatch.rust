use crate::sqs;
use aws_sdk_sqs::{types::Message, Client, Error};
use axum_server;
use std::{
    collections::HashSet,
    future::{Future, IntoFuture},
    pin::Pin,
    sync::Arc,
};
use tokio::{self, sync::watch, task::JoinSet, time};

pub trait Handler: 'static {
    /// Call the handler with the given request.
    fn call(&self, req: &Message)
    where
        Self: 'static;
}

pub struct Server<H> {
    pub client: Client,
    pub queue_url: String,

    handle: axum_server::Handle,
    handler: H,

    futures: JoinSet<Message>,
    inflight: HashSet<String>,
}

unsafe impl<H> Send for Server<H> {}

impl<H> Server<H>
where
    H: Handler + Clone + Send,
{
    /// Create a new server with the given client and queue URL.
    pub fn new(client: Client, queue_url: String, handler: H) -> Self {
        Self {
            client,
            queue_url,
            handle: axum_server::Handle::new(),
            handler: handler,
            futures: JoinSet::new(),
            inflight: HashSet::new(),
        }
    }

    pub fn client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    /// Provide a handle for additional utilities.
    pub fn handle(mut self, handle: axum_server::Handle) -> Self {
        self.handle = handle;
        self
    }

    pub fn queue_url(mut self, queue_url: String) -> Self {
        self.queue_url = queue_url;
        self
    }

    pub fn handler(mut self, handler: H) -> Self {
        self.handler = handler;
        self
    }

    /// Returns a graceful shutdown server.
    pub fn with_graceful_shutdown<F>(self, signal: F) -> WithGracefulShutdown<H, F>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        WithGracefulShutdown {
            server: self,
            signal,
        }
    }

    fn handle_message(&mut self, message: Message) {
        let h = self.handler.clone();
        if let Some(receipt_handle) = &message.receipt_handle {
            self.inflight.insert(receipt_handle.clone());
        }
        self.futures.spawn_blocking(move || {
            h.call(&message);
            message
        });
    }

    /// Check which tasks have finished and which are still in-flight.
    async fn try_join(&mut self) {
        // Remove finished
        let mut finished: HashSet<String> = HashSet::new();
        while let Some(res) = self.futures.try_join_next() {
            match res {
                Ok(message) => {
                    if let Some(receipt_handle) = &message.receipt_handle {
                        finished.insert(receipt_handle.clone());
                        self.inflight.remove(receipt_handle);
                    }
                }
                Err(_) => {
                    //::error!("Error processing message");
                }
            }
        }

        // Mark jobs as done
        if finished.len() > 0 {
            println!("finshed: {:#?}", finished);
            let ret = sqs::delete_message_batch(&self.client, &self.queue_url, &finished).await;
            println!("delete_message_batch: {:#?}", ret)
        }
    }

    /// Check which tasks have finished and which are still in-flight.
    async fn join(&mut self) {
        // Remove finished
        let mut finished: HashSet<String> = HashSet::new();
        while let Some(res) = self.futures.join_next().await {
            match res {
                Ok(message) => {
                    if let Some(receipt_handle) = &message.receipt_handle {
                        finished.insert(receipt_handle.clone());
                        self.inflight.remove(receipt_handle);
                    }
                }
                Err(_) => {
                    //::error!("Error processing message");
                }
            }
        }

        // Mark jobs as done
        if finished.len() > 0 {
            println!("finshed: {:#?}", finished);
            let ret = sqs::delete_message_batch(&self.client, &self.queue_url, &finished).await;
            println!("delete_message_batch: {:#?}", ret)
        }
    }

    /// Call periodically to signal messages are being processed.
    async fn tick_inflight(&mut self) {
        if self.inflight.len() > 0 {
            let ret =
                sqs::change_message_visibility_batch(&self.client, &self.queue_url, &self.inflight)
                    .await;
            println!("change_message_visibility: {:#?}", ret);
        }
    }
}

/// Server with graceful shutdown.
pub struct WithGracefulShutdown<H, F> {
    server: Server<H>,
    signal: F,
}

impl<H, F> IntoFuture for WithGracefulShutdown<H, F>
where
    H: Handler + Clone + Send,
    F: Future<Output = ()> + Send + 'static,
{
    type Output = Result<(), Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        let signal = self.signal;

        let (signal_tx, signal_rx) = watch::channel(());
        let signal_tx = Arc::new(signal_tx);
        tokio::spawn(async move {
            signal.await;
            drop(signal_rx);
        });

        let mut server = self.server;

        Box::pin(async move {
            let mut interval = time::interval(time::Duration::from_millis(500));

            loop {
                tokio::select! {
                    _ = signal_tx.closed() => {
                        break;
                    },
                    _ = interval.tick() => {
                        server.tick_inflight().await;
                    },
                    result = sqs::receive_message(&server.client, &server.queue_url) => {
                        let msg = result.unwrap();
                        for message in msg.messages.unwrap_or_default() {
                            server.handle_message(message);
                        }

                        // Remove finished
                        server.try_join().await;
                    }
                }
            }
            // Wait for remaiing futures to finish
            server.join().await;

            Ok(())
        })
    }
}
