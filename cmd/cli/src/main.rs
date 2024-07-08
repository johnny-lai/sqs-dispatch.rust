use aws_config::BehaviorVersion;
use aws_sdk_sqs::{types::Message, Client, Error};
use clap::Parser;
use sqs_dispatch::{Handler, Server};
use std::{process::Command, time::Duration};
use tokio::{self, signal};

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

impl Handler for Cli {
    fn call(&self, message: &Message) {
        println!("Got the message in handler: {:#?}", message);
        if self.exec.len() > 0 {
            let mut cmd = Command::new(&self.exec[0]);

            for i in 1..self.exec.len() {
                let x = &self.exec[i];
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
}

unsafe impl Send for Cli {}

impl Clone for Cli {
    fn clone(&self) -> Self {
        Cli {
            endpoint_url: self.endpoint_url.clone(),
            queue_url: self.queue_url.clone(),
            exec: self.exec.clone(),
        }
    }
}

async fn shutdown_signal(handle: axum_server::Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Received termination signal shutting down");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait
                                                             // to force shutdown
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
    let server = Server::new(client.clone(), args.queue_url.clone(), args);

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone());

    server
        .handle(handle)
        .with_graceful_shutdown(shutdown_future)
        .await
        .unwrap();

    println!("graceful shutdown complete");

    Ok(())
}
