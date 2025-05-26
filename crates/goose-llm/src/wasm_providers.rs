/// WebAssembly provider wrappers for real providers
/// This module provides WebAssembly-compatible wrappers for the standard providers
/// like OpenAI and Databricks without duplicating their implementation.

#[cfg(all(feature = "wasm", feature = "http"))]
use wasm_bindgen::prelude::*;
use crate::wasm::{WasmMessage, WasmUsage, WasmRuntimeMetrics, WasmCompletionResponse};
use crate::providers::openai::OpenAiProviderConfig; 
use crate::providers::databricks::DatabricksProviderConfig; 
use crate::providers::errors::ProviderError; 


/// WebAssembly wrapper for OpenAI provider
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub async fn wasm_complete_with_openai(request: crate::wasm::WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> {
    // use std::time::Instant; // Timing removed due to WASM issues
    use crate::providers::utils::handle_response_openai_compat; 
    use crate::providers::formats::openai::{create_request as openai_create_request, response_to_message, get_usage};
    use crate::providers::utils::get_model;

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] Entered wasm_complete_with_openai"));
    
    if request.provider_name() != "openai" {
        let error_msg = format!("[OpenAI WASM] Invalid provider name: {}. Expected 'openai'", request.provider_name());
        #[cfg(feature = "wasm")]
        web_sys::console::error_1(&JsValue::from_str(&error_msg));
        return Err(JsValue::from_str(&error_msg));
    }
    
    let system_preamble = request.inner_system_preamble();
    let messages_json = request.inner_messages_json()?;
    let model_name_str = request.inner_model_name();
    let provider_config_json_str = request.inner_provider_config_json()?;
    
    let messages: Vec<crate::message::Message> = serde_json::from_str(&messages_json)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse messages: {}", e)))?;
    
    let model_config = crate::model::ModelConfig::new(model_name_str.clone()).with_max_tokens(Some(2048));
    
    let client = reqwest::Client::new();
    let provider_config_obj: OpenAiProviderConfig = serde_json::from_str(&provider_config_json_str)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse provider_config_json_str into OpenAiProviderConfig: {}", e)))?;

    let payload = openai_create_request(&model_config, &system_preamble, &messages, &[], &crate::providers::utils::ImageFormat::OpenAi)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to create request payload: {}", e)))?;

    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] Payload created. Sending request to OpenAI... URL: {}/{}", &provider_config_obj.host, &provider_config_obj.base_path)));
        
    let base_url = url::Url::parse(&provider_config_obj.host)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[OpenAI WASM] URL parse error: {}", e.to_string())))?;
    let url_to_request = base_url.join(&provider_config_obj.base_path)
        .map_err(|e: url::ParseError| JsValue::from_str(&format!("[OpenAI WASM] URL join error: {}", e.to_string())))?;
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] Final url_to_request: {}", url_to_request.as_str())));

    let mut request_builder = client
        .post(url_to_request.clone())
        .header("Authorization", format!("Bearer {}", provider_config_obj.api_key));

    if let Some(org) = &provider_config_obj.organization {
        request_builder = request_builder.header("OpenAI-Organization", org);
    }
    if let Some(project) = &provider_config_obj.project {
        request_builder = request_builder.header("OpenAI-Project", project);
    }
    if let Some(custom_headers) = &provider_config_obj.custom_headers {
        for (key, value) in custom_headers {
            request_builder = request_builder.header(key, value);
        }
    }
    
    let final_request_builder = request_builder.json(&payload);
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] About to .send().await..."));

    let http_response = match final_request_builder.send().await {
        Ok(resp) => resp,
        Err(reqwest_error) => {
            let error_msg = format!("[OpenAI WASM] reqwest .send().await error: {}", reqwest_error);
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            return Err(JsValue::from_str(&error_msg));
        }
    };
    
    #[cfg(feature = "wasm")]
    web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAI WASM] HTTP request .send().await finished. Status: {}.", http_response.status())));
    
    match handle_response_openai_compat(http_response).await {
        Ok(json_response) => {
            #[cfg(feature = "wasm")]
            web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] Response handled by handle_response_openai_compat. Processing..."));

            let resp_message = response_to_message(json_response.clone())
                .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to parse message from response: {}", e)))?;
            let usage = get_usage(&json_response)
                .map_err(|e: ProviderError| JsValue::from_str(&format!("[OpenAI WASM] Failed to get usage from response: {}", e.to_string())))?;
            let model_name_from_response = get_model(&json_response);

            let wasm_message = WasmMessage::from_json(&serde_json::to_string(&resp_message).map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to serialize final message to JSON: {}",e)))?)?;
            let wasm_usage = WasmUsage::new(usage.input_tokens, usage.output_tokens, usage.total_tokens);
            // Create dummy metrics as time was removed
            let wasm_metrics = WasmRuntimeMetrics::new(0.0, 0.0, None);
            
            #[cfg(feature = "wasm")]
            web_sys::console::log_1(&JsValue::from_str("[OpenAI WASM] OpenAI completion successful. Returning WasmCompletionResponse."));
            
            Ok(WasmCompletionResponse::new(wasm_message, model_name_from_response, wasm_usage, wasm_metrics))
        }
        Err(provider_error) => { 
            let error_msg = format!("[OpenAI WASM] Error from handle_response_openai_compat: {}", provider_error.to_string());
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            Err(JsValue::from_str(&error_msg))
        }
    }
}


/* // Temporarily commented out wasm_complete_with_databricks and minimal_reqwest_test
// ... (ensure wasm_complete_with_databricks is commented out or similarly refactored if it was being worked on)
*/

/// Generic WebAssembly wrapper for any provider
/// This function will route to the appropriate provider based on the request's provider_name
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub async fn wasm_complete_with_provider(request: crate::wasm::WasmCompletionRequest) -> Result<WasmCompletionResponse, JsValue> { 
    match request.provider_name().as_str() {
        "openai" => {
            wasm_complete_with_openai(request).await 
        }
        // "databricks" => wasm_complete_with_databricks(request).await, 
        "mock" => crate::wasm::wasm_complete_with_mock_async(request).await,
        _ => {
            let error_msg = format!("[WASM Provider Router] Unsupported provider: {}", request.provider_name());
            #[cfg(feature = "wasm")]
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            Err(JsValue::from_str(&error_msg))
        }
    }
}


/// Helper function to create a provider configuration for OpenAI
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub fn wasm_create_openai_config(api_key: String, base_url: Option<String>) -> Result<String, JsValue> {
    let config = serde_json::json!({
        "api_key": api_key,
        "host": base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
        "base_path": "v1/chat/completions",
        "timeout": 60,
    });
    
    serde_json::to_string(&config)
        .map_err(|e| JsValue::from_str(&format!("[OpenAI WASM] Failed to serialize OpenAI config: {}", e)))
}

/// Helper function to create a provider configuration for Databricks
#[cfg(all(feature = "wasm", feature = "http"))]
#[wasm_bindgen]
pub fn wasm_create_databricks_config(api_key: String, host: String, endpoint_name: String) -> Result<String, JsValue> {
    let config = serde_json::json!({
        "api_key": api_key,
        "host": host,
        "endpoint_name": endpoint_name,
        "timeout": 60,
    });
    
    serde_json::to_string(&config)
        .map_err(|e| JsValue::from_str(&format!("[Databricks WASM] Failed to serialize Databricks config: {}", e)))
}
