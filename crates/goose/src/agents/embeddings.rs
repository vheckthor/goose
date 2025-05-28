use crate::model::ModelConfig;
use crate::providers::embedding::EmbeddingCapable;
use crate::providers::{self, base::Provider};
use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;

pub struct EmbeddingProvider {
    provider: Arc<dyn Provider>,
}

impl EmbeddingProvider {
    pub fn new() -> Result<Self> {
        // Get embedding model and provider from environment variables
        let embedding_model =
            env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "text-embedding-3-small".to_string());
        let embedding_provider =
            env::var("EMBEDDING_MODEL_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        // Create the provider using the factory
        let model_config = ModelConfig::new(embedding_model);
        let provider = providers::create(&embedding_provider, model_config).context(format!(
            "Failed to create {} provider for embeddings. If using OpenAI, make sure OPENAI_API_KEY env var is set or that you have configured the OpenAI provider via Goose before.",
            embedding_provider
        ))?;

        Ok(Self { provider })
    }

    pub fn from_provider(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Check if provider implements EmbeddingCapable
        if let Some(embedding_capable) = self
            .provider
            .as_any()
            .downcast_ref::<crate::providers::openai::OpenAiProvider>()
        {
            return embedding_capable.create_embeddings(texts).await;
        }

        if let Some(embedding_capable) =
            self.provider
                .as_any()
                .downcast_ref::<crate::providers::databricks::DatabricksProvider>()
        {
            return embedding_capable.create_embeddings(texts).await;
        }

        // If provider doesn't support embeddings, return an error
        Err(anyhow::anyhow!(
            "Provider {} does not support embeddings",
            self.provider.get_name()
        ))
    }

    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self.embed(vec![text]).await?;
        embeddings
            .into_iter()
            .next()
            .context("No embedding returned")
    }
}

// Ebedding provider that generates random embeddings for testing
pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Ok(texts
            .into_iter()
            .map(|_| (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect())
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
pub async fn create_embedding_provider() -> Result<Box<dyn EmbeddingProviderTrait>> {
    let embedding_provider = EmbeddingProvider::new()?;
    Ok(Box::new(embedding_provider))
}

// Create embedding provider from an existing provider instance
pub async fn create_embedding_provider_from_instance(
    provider: Arc<dyn Provider>,
) -> Result<Box<dyn EmbeddingProviderTrait>> {
    Ok(Box::new(EmbeddingProvider::from_provider(provider)))
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

// Mock provider for testing
#[derive(Debug)]
#[allow(dead_code)]
struct MockProvider;

#[async_trait::async_trait]
impl Provider for MockProvider {
    fn metadata() -> crate::providers::base::ProviderMetadata {
        crate::providers::base::ProviderMetadata::new(
            "mock",
            "Mock Provider",
            "Mock provider for testing",
            "mock-model",
            vec![],
            "",
            vec![],
        )
    }

    fn get_model_config(&self) -> crate::model::ModelConfig {
        crate::model::ModelConfig::new("mock-model".to_string())
    }

    async fn complete(
        &self,
        _system: &str,
        _messages: &[crate::message::Message],
        _tools: &[mcp_core::tool::Tool],
    ) -> Result<
        (
            crate::message::Message,
            crate::providers::base::ProviderUsage,
        ),
        crate::providers::errors::ProviderError,
    > {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new();

        // Test single embedding
        let text = "Test text for embedding".to_string();
        let embedding = provider.embed_single(text).await.unwrap();

        // Check dimensions
        assert_eq!(embedding.len(), 1536);

        // Check values are within expected range (-1.0 to 1.0)
        for value in embedding {
            assert!(value >= -1.0 && value <= 1.0);
        }

        // Test batch embedding
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        let embeddings = provider.embed(texts).await.unwrap();

        // Check batch results
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 1536);
            for value in embedding {
                assert!(value >= -1.0 && value <= 1.0);
            }
        }
    }

    #[tokio::test]
    async fn test_empty_input_mock_provider() {
        let provider = MockEmbeddingProvider::new();
        let empty_texts: Vec<String> = vec![];
        let result = provider.embed(empty_texts).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_embedding_provider_creation() {
        // Test that embedding provider can be created from a mock provider instance
        let mock_provider = Arc::new(MockProvider);
        let embedding_provider = EmbeddingProvider::from_provider(mock_provider);

        // Verify the provider was created
        assert_eq!(embedding_provider.provider.get_name(), "MockProvider");
    }

    #[tokio::test]
    async fn test_create_embedding_provider_from_instance() {
        // Test the factory function with a mock provider
        let mock_provider = Arc::new(MockProvider);
        let result = create_embedding_provider_from_instance(mock_provider).await;

        assert!(result.is_ok());
        // The embedding provider should be created successfully
    }

    #[tokio::test]
    async fn test_mock_embedding_consistency() {
        let provider = MockEmbeddingProvider::new();

        // Test that different texts get different embeddings
        let text1 = "First text".to_string();
        let text2 = "Second text".to_string();

        let embedding1 = provider.embed_single(text1.clone()).await.unwrap();
        let embedding2 = provider.embed_single(text2.clone()).await.unwrap();

        // Verify embeddings are different (random values should make this extremely likely)
        assert!(embedding1 != embedding2);

        // Verify same text gets different embeddings (mock doesn't cache)
        let embedding1_repeat = provider.embed_single(text1).await.unwrap();
        assert!(embedding1 != embedding1_repeat);
    }
}
