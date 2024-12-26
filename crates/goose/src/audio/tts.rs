use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::key_manager::{get_keyring_secret, KeyRetrievalStrategy};

const ELEVENLABS_API_URL: &str = "https://api.elevenlabs.io/v1/text-to-speech";

#[derive(Debug, Serialize)]
struct TextToSpeechRequest {
    text: String,
    // Add optional fields like model_id, voice_settings if needed
}

#[derive(Debug, Deserialize)]
struct TextToSpeechError {
    message: String,
}

pub struct ElevenLabsTTS {
    api_key: String,
    client: reqwest::Client,
}

impl ElevenLabsTTS {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let api_key = get_keyring_secret("ELEVENLABS_API_KEY", KeyRetrievalStrategy::Both)?;
        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
        })
    }

    pub async fn synthesize_speech(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let url = format!("{}/{}", ELEVENLABS_API_URL, voice_id);
        
        let request_body = TextToSpeechRequest {
            text: text.to_string(),
        };

        let response = self.client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error: TextToSpeechError = response.json().await?;
            return Err(format!("API request failed: {}", error.message).into());
        }

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn save_audio(&self, audio_data: Vec<u8>, output_path: &str) -> Result<(), Box<dyn Error>> {
        std::fs::write(output_path, audio_data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tts_synthesis() {
        let tts = ElevenLabsTTS::new().await.unwrap();
        let text = "Hello, this is a test of text to speech synthesis.";
        let voice_id = "your_voice_id"; // Replace with actual voice ID
        
        let result = tts.synthesize_speech(text, voice_id).await;
        assert!(result.is_ok());
        
        // Save the audio file for testing
        if let Ok(audio_data) = result {
            let save_result = tts.save_audio(audio_data, "test_output.mp3").await;
            assert!(save_result.is_ok());
        }
    }
}