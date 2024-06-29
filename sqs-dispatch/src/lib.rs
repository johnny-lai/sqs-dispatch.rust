use aws_sdk_sqs::{types::Message, Client, Error};
use tokio::{self};

pub trait Handler: 'static {
    /// Call the handler with the given request.
    fn call(&self, req: Message)
    where
        Self: 'static;
}

pub struct Server {
    pub client: Client,

    pub queue_url: String,
}

impl Server {
    pub async fn receive<H: Handler + Clone + Send>(self, handler: H) -> Result<(), Error> {
        let rcv_message_output = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .send()
            .await?;

        for message in rcv_message_output.messages.unwrap_or_default() {
            let h = handler.clone();
            let _ = tokio::task::spawn(async move { h.call(message) }).await;
        }

        Ok(())
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
