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
        let mut queues = vec![];

        let response = self.client.list_queues()
            .into_paginator()
            .items()
            .send()
            .try_collect()
            .await?;

        for output in response {
            let queue_name = get_queue_name(output.as_str())?;
            queues.push(queue_name);
        }

        Ok(queues)
    }

    pub fn list_queues(&self) -> anyhow::Result<Vec<String>> {
        self.list_queues_async()
    }
}

fn get_queue_name(queue_url: &str) -> anyhow::Result<String> {
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
