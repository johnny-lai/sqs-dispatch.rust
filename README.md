# sqs-dispatch.rust

## Build

```
$ cargo build
```

## Run

```
$ target/debug/sqs-dispatch --help
Pulls messages from SQS and executes the specified command

Usage: sqs-dispatch [OPTIONS] --queue-url <QUEUE_URL> --exec <EXEC>

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

You can then create some SQS queues
```
$ awslocal sqs create-queue --queue-name example-queue
{
    "QueueUrl": "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/example-queue"
}
```

You can then send some messages to the queue
```
$ awslocal sqs send-message --queue-url "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/example-queue" --message-body "Message 1"
{
    "MD5OfMessageBody": "68390233272823b7adf13a1db79b2cd7",
    "MessageId": "e4fc4655-d65b-48c3-9cd3-b69aafbfaf5a"
}
$ awslocal sqs send-message --queue-url "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/example-queue" --message-body "Message 2"
{
    "MD5OfMessageBody": "88ef8f31ed540f1c4c03d5fdb06a7935",
    "MessageId": "159f3973-4d64-4086-8a48-4891a1b71661"
}
```

When you run sqs-dispatch
```
$ cargo run -- -E http://localhost:4956 -Q "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/example-queue" -e "echo" -e "I got a message: " -e "{}.messageId" -e "{}.body"
```