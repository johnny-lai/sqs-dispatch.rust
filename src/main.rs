use aws_config::{BehaviorVersion};
use aws_sdk_sqs::{Client, Error};
use clap::Parser;
use std::process::Command;

/// Pulls messages from SQS and executes the specified command
#[derive(Debug, Parser)]
struct Cli {
    /// SQS Endpoint URL
    #[arg(short('E'), long)]
    endpoint_url: Option<String>,

    /// SQS Queue URL
    #[arg(short('Q'), long)]
    queue_url: String,

    /// exec arguments. Use {}.messageId and {}.body to get the message
    #[arg(short, long)]
    exec: Vec<String>,
}

async fn receive(client: &Client, args: &Cli) -> Result<(), Error> {
    let rcv_message_output = client.receive_message().queue_url(&args.queue_url).send().await?;

    println!("Messages from queue with url: {}", args.queue_url);

    for message in rcv_message_output.messages.unwrap_or_default() {
        println!("Got the message: {:#?}", message);

        if args.exec.len() > 0 {
            let mut cmd = Command::new(&args.exec[0]);

            for i in 1..args.exec.len() {
                let x = &args.exec[i];
                if x == "{}.messageId" {
                    if let Some(val) = &message.message_id {
                        cmd.arg(val);
                    }
                } else if x == "{}.body" {
                    if let Some(val) = &message.body {
                        cmd.arg(val);
                    }
                } else {
                    cmd.arg(x);
                }
            }

            println!("{:?}", cmd.output());
        }
    }


    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Cli::parse();

    println!("args: {:?}", args);

    let mut loader = aws_config::defaults(BehaviorVersion::v2023_11_09());
    if let Some(endpoint_url) = &args.endpoint_url {
        loader = loader.endpoint_url(endpoint_url)
    }
    let config = loader.load().await;

    let client = Client::new(&config);
    receive(&client, &args).await?;

    Ok(())
}