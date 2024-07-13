use aws_sdk_sqs::{
    operation::{
        change_message_visibility::{ChangeMessageVisibilityError, ChangeMessageVisibilityOutput},
        change_message_visibility_batch::{
            ChangeMessageVisibilityBatchError, ChangeMessageVisibilityBatchOutput,
        },
        delete_message::{DeleteMessageError, DeleteMessageOutput},
        delete_message_batch::{DeleteMessageBatchError, DeleteMessageBatchOutput},
        receive_message::{ReceiveMessageError, ReceiveMessageOutput},
    },
    types::{ChangeMessageVisibilityBatchRequestEntry, DeleteMessageBatchRequestEntry, Message},
    Client, Error,
};
use aws_smithy_runtime_api::client::{orchestrator::HttpResponse, result::SdkError};
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

    /// Receive messages from the queue.
    async fn receive_message(
        &self,
    ) -> Result<ReceiveMessageOutput, SdkError<ReceiveMessageError, HttpResponse>> {
        self.client
            .receive_message()
            .queue_url(&self.queue_url)
            .send()
            .await
    }

    /// Change visibility of a given message
    #[allow(dead_code)]
    async fn change_message_visibility(
        &self,
        receipt_handle: &String,
    ) -> Result<ChangeMessageVisibilityOutput, SdkError<ChangeMessageVisibilityError, HttpResponse>>
    {
        self.client
            .change_message_visibility()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .visibility_timeout(1) // Seconds
            .send()
            .await
    }

    /// Change visibility of multiple messages.
    async fn change_message_visibility_batch(
        &self,
        receipt_handles: &HashSet<String>,
    ) -> Result<
        ChangeMessageVisibilityBatchOutput,
        SdkError<ChangeMessageVisibilityBatchError, HttpResponse>,
    > {
        let op = self
            .client
            .change_message_visibility_batch()
            .queue_url(&self.queue_url);

        let mut entries: Vec<ChangeMessageVisibilityBatchRequestEntry> = Vec::new();
        // Create ChangeMessageVisibilityBatchRequestEntry for each message
        for (i, message_id) in receipt_handles.iter().enumerate() {
            let entry = ChangeMessageVisibilityBatchRequestEntry::builder()
                .id(i.to_string()) // Unique identifier for the request entry
                .receipt_handle(message_id)
                .visibility_timeout(1) // Setting the visibility timeout value
                .build();
            entries.push(entry.unwrap());
        }

        op.set_entries(Some(entries)).send().await
    }

    /// Delete a single message.
    #[allow(dead_code)]
    async fn delete_message(
        &self,
        receipt_handle: &String,
    ) -> Result<DeleteMessageOutput, SdkError<DeleteMessageError, HttpResponse>> {
        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
    }

    /// Delete a batch of messages
    async fn delete_message_batch(
        &self,
        message_ids: &HashSet<String>,
    ) -> Result<DeleteMessageBatchOutput, SdkError<DeleteMessageBatchError, HttpResponse>> {
        let op = self
            .client
            .delete_message_batch()
            .queue_url(&self.queue_url);

        let mut entries: Vec<DeleteMessageBatchRequestEntry> = Vec::new();
        // Create ChangeMessageVisibilityBatchRequestEntry for each message
        for (i, message_id) in message_ids.iter().enumerate() {
            let entry = DeleteMessageBatchRequestEntry::builder()
                .id(i.to_string()) // Unique identifier for the request entry
                .receipt_handle(message_id)
                .build();
            entries.push(entry.unwrap());
        }

        op.set_entries(Some(entries)).send().await
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
            let ret = self.delete_message_batch(&finished).await;
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
            let ret = self.delete_message_batch(&finished).await;
            println!("delete_message_batch: {:#?}", ret)
        }
    }

    /// Call periodically to signal messages are being processed.
    async fn tick_inflight(&mut self) {
        if self.inflight.len() > 0 {
            let ret = self.change_message_visibility_batch(&self.inflight).await;
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
                    result = server.receive_message() => {
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
