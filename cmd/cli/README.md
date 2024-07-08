# sqs-dispatch.rust

## Build

```
$ cargo build
```

## Run

```
$ cargo run -- --help
Pulls messages from SQS and executes the specified command

Usage: sqs-dispatch [OPTIONS] --queue-url <QUEUE_URL>

Options:
  -E, --endpoint-url <ENDPOINT_URL>  SQS Endpoint URL
  -Q, --queue-url <QUEUE_URL>        SQS Queue URL
  -e, --exec <EXEC>                  exec arguments. Use {}.messageId and {}.body to get the message
  -h, --help                         Print help
```

or

```
$ cargo run -- --help
```

## Example

For local development, you can use `localstack` and `awslocal` to simulate an AWS environment.

```
$ pip3 install localstack
$ pip3 install awscli-local
```

```
$ localstack start
```

You can then create some SQS queues and assign it to `Q`.
```
$ Q=`awslocal sqs create-queue --queue-name example-queue | jq -r ".QueueUrl"`
```

You can then send some messages to the queue
```
$ awslocal sqs send-message --queue-url "$Q" --message-body "Message 1"
{
    "MD5OfMessageBody": "68390233272823b7adf13a1db79b2cd7",
    "MessageId": "e4fc4655-d65b-48c3-9cd3-b69aafbfaf5a"
}
$ awslocal sqs send-message --queue-url "$Q" --message-body "Message 2"
{
    "MD5OfMessageBody": "88ef8f31ed540f1c4c03d5fdb06a7935",
    "MessageId": "159f3973-4d64-4086-8a48-4891a1b71661"
}
```

Before you run sqs-dispatch, you may need to set some fake AWS credentials for `localstack`.
```
# Export some fake credentials in AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY. Localstack accepts any credentials
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=us-east-1
```

Print the mesage.
```
cargo run -- -E http://localhost:4566 -Q "$Q" -e "echo" -e "I got a message: " -e "{}.messageId" -e "{}.body"
```

Take 10 seconds to process each message.
```
cargo run -- -E http://localhost:4566 -Q "$Q" -e "sleep" -e "10"
```
