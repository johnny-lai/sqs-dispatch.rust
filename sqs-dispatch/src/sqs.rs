use aws_sdk_sqs::{
    operation::{
        change_message_visibility::{ChangeMessageVisibilityError, ChangeMessageVisibilityOutput},
        change_message_visibility_batch::{
            ChangeMessageVisibilityBatchError, ChangeMessageVisibilityBatchOutput,
        },
        delete_message::{DeleteMessageError, DeleteMessageOutput},
        delete_message_batch::{DeleteMessageBatchError, DeleteMessageBatchOutput},
        receive_message::{ReceiveMessageError, ReceiveMessageOutput},
        send_message::{SendMessageError, SendMessageOutput},
    },
    types::{ChangeMessageVisibilityBatchRequestEntry, DeleteMessageBatchRequestEntry},
    Client,
};
use aws_smithy_runtime_api::client::{orchestrator::HttpResponse, result::SdkError};
use std::{collections::HashSet, convert::Into};

pub(crate) async fn send_message(
    client: &Client,
    queue_url: &String,
    message_body: impl Into<String>,
) -> Result<SendMessageOutput, SdkError<SendMessageError, HttpResponse>> {
    client
        .send_message()
        .queue_url(queue_url)
        .message_body(message_body)
        .send()
        .await
}

/// Receive messages from the queue.
pub(crate) async fn receive_message(
    client: &Client,
    queue_url: &String,
) -> Result<ReceiveMessageOutput, SdkError<ReceiveMessageError, HttpResponse>> {
    client.receive_message().queue_url(queue_url).send().await
}

/// Change visibility of a given message
#[allow(dead_code)]
pub(crate) async fn change_message_visibility(
    client: &Client,
    queue_url: &String,
    receipt_handle: &String,
) -> Result<ChangeMessageVisibilityOutput, SdkError<ChangeMessageVisibilityError, HttpResponse>> {
    client
        .change_message_visibility()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .visibility_timeout(1) // Seconds
        .send()
        .await
}

/// Change visibility of multiple messages.
pub(crate) async fn change_message_visibility_batch(
    client: &Client,
    queue_url: &String,
    receipt_handles: &HashSet<String>,
) -> Result<
    ChangeMessageVisibilityBatchOutput,
    SdkError<ChangeMessageVisibilityBatchError, HttpResponse>,
> {
    let op = client
        .change_message_visibility_batch()
        .queue_url(queue_url);

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
pub(crate) async fn delete_message(
    client: &Client,
    queue_url: &String,
    receipt_handle: &String,
) -> Result<DeleteMessageOutput, SdkError<DeleteMessageError, HttpResponse>> {
    client
        .delete_message()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .send()
        .await
}

/// Delete a batch of messages
pub(crate) async fn delete_message_batch(
    client: &Client,
    queue_url: &String,
    message_ids: &HashSet<String>,
) -> Result<DeleteMessageBatchOutput, SdkError<DeleteMessageBatchError, HttpResponse>> {
    let op = client.delete_message_batch().queue_url(queue_url);

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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::test_helpers::TestClient;

    #[tokio::test]
    async fn test_add() {
        let t = TestClient::new().await;
        send_message(&t.client, &t.queue_url, "Hello, world!")
            .await
            .unwrap();
        let result = receive_message(&t.client, &t.queue_url).await.unwrap();
        let messages = result.messages();
        assert_eq!(messages.len(), 1);
    }
}
