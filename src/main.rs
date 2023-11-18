use clap::Parser;

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

fn main() {
    let args = Cli::parse();

    println!("args: {:?}", args)
}