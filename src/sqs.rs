use anyhow::anyhow;
use aws_config::BehaviorVersion;
use aws_sdk_sqs::Client;
use aws_sdk_sqs::operation::delete_message::DeleteMessageOutput;
use aws_sdk_sqs::operation::receive_message::ReceiveMessageOutput;
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
    #[tokio::main]
    async fn receive_message_async(&self, queue_url: &str) -> anyhow::Result<ReceiveMessageOutput> {
        let receive_message_output = self.client.receive_message()
            .queue_url(queue_url)
            .max_number_of_messages(1)
            .send()
            .await?;

        Ok(receive_message_output)
    }
    pub fn receive_message(&self, queue_url: &str) -> anyhow::Result<ReceiveMessageOutput> {
        self.receive_message_async(queue_url)
    }

    #[tokio::main]
    async fn delete_message_async(&self, queue_url: &str, receipt_handle: &str) -> anyhow::Result<DeleteMessageOutput> {
        let delete_message_output = self.client.delete_message()
            .queue_url(queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await?;

        Ok(delete_message_output)
    }

    pub fn delete_message(&self, queue_url: &str, receipt_handle: &str) -> anyhow::Result<DeleteMessageOutput> {
        self.delete_message_async(queue_url, receipt_handle)
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
