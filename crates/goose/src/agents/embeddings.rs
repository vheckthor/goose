use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

pub struct EmbeddingProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl EmbeddingProvider {
    pub fn new() -> Result<Self> {
        // Try to get API key from environment
        let api_key = env::var("OPENAI_API_KEY")
            .or_else(|_| env::var("EMBEDDING_API_KEY"))
            .context("No API key found for embeddings. Set OPENAI_API_KEY or EMBEDDING_API_KEY")?;

        let base_url = env::var("EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let model = env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
            model,
        })
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = EmbeddingRequest {
            input: texts,
            model: self.model.clone(),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send embedding request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Embedding API error: {}", error_text);
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }

    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self.embed(vec![text]).await?;
        embeddings
            .into_iter()
            .next()
            .context("No embedding returned")
    }
}

// Fallback embedding provider that generates random embeddings for testing
pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        Ok(texts
            .into_iter()
            .map(|_| {
                (0..1536)
                    .map(|_| rng.gen_range(-1.0..1.0))
                    .collect()
            })
            .collect())
    }

    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self.embed(vec![text]).await?;
        embeddings
            .into_iter()
            .next()
            .context("No embedding returned")
    }
}

// Factory function to create appropriate embedding provider
pub async fn create_embedding_provider() -> Box<dyn EmbeddingProviderTrait> {
    match EmbeddingProvider::new() {
        Ok(provider) => Box::new(provider),
        Err(e) => {
            tracing::warn!("Failed to create embedding provider: {}. Using mock provider.", e);
            Box::new(MockEmbeddingProvider::new())
        }
    }
}

#[async_trait::async_trait]
pub trait EmbeddingProviderTrait: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn embed_single(&self, text: String) -> Result<Vec<f32>>;
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for EmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed(texts).await
    }

    async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        self.embed_single(text).await
    }
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for MockEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed(texts).await
    }

    async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        self.embed_single(text).await
    }
}