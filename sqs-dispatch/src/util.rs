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
use std::{
    collections::HashSet,
    future::{Future, IntoFuture},
    pin::Pin,
    sync::Arc,
};

pub(crate) async fn change_message_visibility_batch(
    client: Client,
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
