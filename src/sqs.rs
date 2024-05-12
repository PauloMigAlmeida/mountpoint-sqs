use anyhow::anyhow;
use aws_config::BehaviorVersion;
use aws_sdk_sqs::Client;
use url::Url;

pub struct SQSClient {
    client: Client,
}

impl SQSClient {
    #[tokio::main]
    pub async fn new() -> Self {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        SQSClient {
            client: aws_sdk_sqs::Client::new(&config)
        }
    }

    #[tokio::main]
    async fn list_queues_async(&self) -> anyhow::Result<Vec<String>> {
        let response = self.client.list_queues()
            .into_paginator()
            .items()
            .send()
            .try_collect()
            .await?;

        Ok(response)
    }

    pub fn list_queues(&self) -> anyhow::Result<Vec<String>> {
        self.list_queues_async()
    }

    #[tokio::main]
    async fn send_message_async(&self, queue_url: &str, message: &str) -> anyhow::Result<u32> {
        if message.len() > 256 * 1024 {
            return Err(anyhow!("message length can't be above 256kb as per SQS limits"));
        }

        self.client.send_message()
            .queue_url(queue_url)
            .message_body(message)
            .send()
            .await?;

        Ok(message.len() as u32)
    }

    pub fn send_message(&self, queue_url: &str, message: &str) -> anyhow::Result<u32> {
        self.send_message_async(queue_url, message)
    }
}

pub fn get_queue_name(queue_url: &str) -> anyhow::Result<String> {
    let url = Url::parse(queue_url)?;

    let segments = url.path_segments();

    if segments.is_none() {
        return Err(anyhow!("No segments: {}", queue_url));
    }

    let last = segments.unwrap().last();

    if last.is_none() {
        return Err(anyhow!("No queue name: {}", queue_url));
    }

    Ok(last.unwrap().to_string())
}
