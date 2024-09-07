use aws_config::BehaviorVersion;
use aws_sdk_sqs::{
    config::endpoint,
    operation::{
        change_message_visibility::{ChangeMessageVisibilityError, ChangeMessageVisibilityOutput},
        change_message_visibility_batch::{
            ChangeMessageVisibilityBatchError, ChangeMessageVisibilityBatchOutput,
        },
        delete_message::{DeleteMessageError, DeleteMessageOutput},
        delete_message_batch::{DeleteMessageBatchError, DeleteMessageBatchOutput},
        receive_message::{ReceiveMessageError, ReceiveMessageOutput},
    },
    types::{ChangeMessageVisibilityBatchRequestEntry, DeleteMessageBatchRequestEntry},
    Client,
};

pub(crate) struct TestClient {
    pub(crate) client: Client,
    pub(crate) queue_url: String,
}

impl TestClient {
    pub(crate) async fn new() -> Self {
        let mut loader = aws_config::defaults(BehaviorVersion::v2023_11_09());
        loader = loader.endpoint_url("http://localhost:4566");
        let config = loader.load().await;

        let client = Client::new(&config);

        TestClient {
            client,
            queue_url: String::from(
                "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/example-queue",
            ),
        }
    }
}
