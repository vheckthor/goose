use anyhow::Result;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tracing::info;
use url::Url;

lazy_static! {
    static ref OAUTH_MUTEX: TokioMutex<()> = TokioMutex::new(());
}

pub const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";

#[derive(Debug, Clone)]
struct OidcEndpoints {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenCache {
    access_token: String,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<u64>,
}

fn get_cache_path(client_id: &str, scopes: &[String]) -> PathBuf {
    let mut hasher = sha2::Sha256::new();
    hasher.update(client_id.as_bytes());
    hasher.update(scopes.join(",").as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let base_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("goose/google/oauth");

    fs::create_dir_all(&base_path).unwrap_or_default();
    base_path.join(format!("{}.json", hash))
}

fn load_cached_token(client_id: &str, scopes: &[String]) -> Option<String> {
    let cache_path = get_cache_path(client_id, scopes);
    if let Ok(contents) = fs::read_to_string(&cache_path) {
        if let Ok(cache) = serde_json::from_str::<TokenCache>(&contents) {
            if let Some(expires_at) = cache.expires_at {
                if expires_at > Utc::now() {
                    info!(
                        "Using cached OAuth token from {} valid until {}",
                        cache_path.display(),
                        expires_at
                    );
                    return Some(cache.access_token);
                }
            }
        }
    }
    info!(
        "No valid cached OAuth token found at {}",
        cache_path.display()
    );
    None
}

fn save_token_cache(client_id: &str, scopes: &[String], token: &str, expires_in: Option<u64>) {
    let expires_at = expires_in.map(|secs| Utc::now() + Duration::seconds(secs as i64));
    let cache_path = get_cache_path(client_id, scopes);

    info!(
        "Saving new OAuth token to {}{}",
        cache_path.display(),
        expires_at.map_or(String::new(), |exp| format!(" valid until {}", exp))
    );

    let token_cache = TokenCache {
        access_token: token.to_string(),
        expires_at,
    };

    if let Ok(contents) = serde_json::to_string(&token_cache) {
        fs::write(&cache_path, contents)
            .map_err(|e| anyhow::anyhow!("Failed to write token cache: {}", e))
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to write to {}: {}", cache_path.display(), e)
            });
    }
}

async fn get_workspace_endpoints(host: &str) -> Result<OidcEndpoints> {
    let base_url = Url::parse(host).expect("Invalid host URL");
    let oidc_url = base_url
        .join("oidc/.well-known/oauth-authorization-server")
        .expect("Invalid OIDC URL");

    let client = reqwest::Client::new();
    let resp = client.get(oidc_url.clone()).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to get OIDC configuration from {}",
            oidc_url.to_string()
        ));
    }

    let oidc_config: Value = resp.json().await?;

    let authorization_endpoint = oidc_config
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("authorization_endpoint not found in OIDC configuration"))?
        .to_string();

    let token_endpoint = oidc_config
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("token_endpoint not found in OIDC configuration"))?
        .to_string();

    Ok(OidcEndpoints {
        authorization_endpoint,
        token_endpoint,
    })
}

struct OAuthFlow {
    endpoints: OidcEndpoints,
    client_id: String,
    client_secret: String,
    redirect_url: String,
    scopes: Vec<String>,
    state: String,
    verifier: String,
}

impl OAuthFlow {
    fn new(
        endpoints: OidcEndpoints,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            endpoints,
            client_id,
            client_secret,
            redirect_url,
            scopes,
            state: nanoid::nanoid!(16),
            verifier: nanoid::nanoid!(64),
        }
    }

    fn get_authorization_url(&self) -> String {
        let challenge = {
            let digest = sha2::Sha256::digest(self.verifier.as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
        };

        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", &self.redirect_url),
            ("scope", &self.scopes.join(" ")),
            ("state", &self.state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];

        format!(
            "{}?{}",
            self.endpoints.authorization_endpoint,
            serde_urlencoded::to_string(params).unwrap()
        )
    }

    async fn exchange_code(&self, code: &str) -> Result<TokenResponse> {
        let client = reqwest::Client::new();
        let mut params = vec![
            ("client_id", self.client_id.as_str()),
            ("code", code),
            ("redirect_uri", self.redirect_url.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", self.verifier.as_str()),
        ];

        // Only add client_secret if it's not empty (private client)
        if !self.client_secret.is_empty() {
            params.push(("client_secret", self.client_secret.as_str()));
        }

        let response = client
            .post(&self.endpoints.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to exchange code for token: {}",
                error
            ));
        }

        response.json().await.map_err(Into::into)
    }

    async fn execute(&self) -> Result<TokenResponse> {
        // Create a channel that will send the auth code from the app process
        let (tx, rx) = oneshot::channel();
        let state = self.state.clone();
        // Axum can theoretically spawn multiple threads, so we need this to be in an Arc even
        // though it will ultimately only get used once
        let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

        // Setup a server that will recieve the redirect, capture the code, and display success/failure
        let app = Router::new().route(
            "/",
            get(move |Query(params): Query<HashMap<String, String>>| {
                let tx = Arc::clone(&tx);
                let state = state.clone();
                async move {
                    let code = params.get("code").cloned();
                    let received_state = params.get("state").cloned();

                    if let (Some(code), Some(received_state)) = (code, received_state) {
                        if received_state == state {
                            if let Some(sender) = tx.lock().await.take() {
                                if sender.send(code).is_ok() {
                                    // Use the improved HTML response
                                    return Html(
                                        "<h2>Login Success</h2><p>You can close this window</p>",
                                    );
                                }
                            }
                            Html("<h2>Error</h2><p>Authentication already completed.</p>")
                        } else {
                            Html("<h2>Error</h2><p>State mismatch.</p>")
                        }
                    } else {
                        Html("<h2>Error</h2><p>Authentication failed.</p>")
                    }
                }
            }),
        );

        // Start the server to accept the oauth code
        let redirect_url = Url::parse(&self.redirect_url)?;
        let port = redirect_url.port().unwrap_or(80);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let server_handle = tokio::spawn(async move {
            let server = axum::serve(listener, app);
            server.await.unwrap();
        });

        // Open the browser which will redirect with the code to the server
        let authorization_url = self.get_authorization_url();
        if webbrowser::open(&authorization_url).is_err() {
            println!(
                "Please open this URL in your browser:\n{}",
                authorization_url
            );
        }

        // Wait for the authorization code with a timeout
        let code = tokio::time::timeout(
            std::time::Duration::from_secs(60), // 1 minute timeout
            rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Authentication timed out"))??;

        // Stop the server
        server_handle.abort();

        // Exchange the code for a token
        self.exchange_code(&code).await
    }

    fn new_with_endpoints(
        endpoints: OidcEndpoints,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            endpoints,
            client_id,
            client_secret,
            redirect_url,
            scopes,
            state: nanoid::nanoid!(16),
            verifier: nanoid::nanoid!(64),
        }
    }
}

pub(crate) async fn get_oauth_token_async(
    host: &str,
    client_id: &str,
    redirect_url: &str,
    scopes: &[String],
) -> Result<String> {
    // Try to load from cache first
    if let Some(token) = load_cached_token(client_id, scopes) {
        return Ok(token);
    }

    // Get OIDC configuration
    let endpoints = get_workspace_endpoints(host).await?;

    // If no valid cached token, perform OAuth flow
    let flow = OAuthFlow::new(
        endpoints,
        client_id.to_string(),
        client_id.to_string(),
        redirect_url.to_string(),
        scopes.to_vec(),
    );

    let token_response = flow.execute().await?;

    // Cache the token before returning
    save_token_cache(
        client_id,
        scopes,
        &token_response.access_token,
        token_response.expires_in,
    );

    Ok(token_response.access_token)
}

pub async fn get_oauth_token_with_endpoints_async(
    auth_endpoint: &str,
    token_endpoint: &str,
    client_id: &str,
    client_secret: &str,
    scopes: &[String],
) -> Result<String> {
    // Try to load from cache first
    if let Some(token) = load_cached_token(client_id, scopes) {
        return Ok(token);
    }

    // If no valid cached token, perform OAuth flow
    let flow = OAuthFlow::new_with_endpoints(
        OidcEndpoints {
            authorization_endpoint: auth_endpoint.to_string(),
            token_endpoint: token_endpoint.to_string(),
        },
        client_id.to_string(),
        client_secret.to_string(),
        DEFAULT_REDIRECT_URL.to_string(),
        scopes.to_vec(),
    );

    let token_response = flow.execute().await?;

    // Cache the token before returning
    save_token_cache(
        client_id,
        scopes,
        &token_response.access_token,
        token_response.expires_in,
    );

    Ok(token_response.access_token)
}

// Add new function for public client OAuth
pub async fn get_oauth_token_public_client_async(
    auth_endpoint: &str,
    token_endpoint: &str,
    client_id: &str,
    scopes: &[String],
) -> Result<String> {
    // Try to load from cache first
    if let Some(token) = load_cached_token(client_id, scopes) {
        return Ok(token);
    }

    // If no valid cached token, perform OAuth flow
    let flow = OAuthFlow::new_with_endpoints(
        OidcEndpoints {
            authorization_endpoint: auth_endpoint.to_string(),
            token_endpoint: token_endpoint.to_string(),
        },
        client_id.to_string(),
        String::new(), // Empty client secret for public clients
        DEFAULT_REDIRECT_URL.to_string(),
        scopes.to_vec(),
    );

    let token_response = flow.execute().await?;

    // Cache the token before returning
    save_token_cache(
        client_id,
        scopes,
        &token_response.access_token,
        token_response.expires_in,
    );

    Ok(token_response.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_get_workspace_endpoints() -> Result<()> {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "authorization_endpoint": "https://example.com/oauth2/authorize",
            "token_endpoint": "https://example.com/oauth2/token"
        });

        Mock::given(method("GET"))
            .and(path("/oidc/.well-known/oauth-authorization-server"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let endpoints = get_workspace_endpoints(&mock_server.uri()).await?;

        assert_eq!(
            endpoints.authorization_endpoint,
            "https://example.com/oauth2/authorize"
        );
        assert_eq!(endpoints.token_endpoint, "https://example.com/oauth2/token");

        Ok(())
    }

    #[test]
    fn test_token_cache() -> Result<()> {
        let cache = TokenCache {
            access_token: "test-token".to_string(),
            expires_at: Some(Utc::now() + Duration::seconds(3600)),
        };

        let token_data = TokenResponse {
            access_token: "test-token".to_string(),
            expires_in: Some(3600),
        };

        save_token_cache(
            "https://example.com",
            &["scope1".to_string()],
            &token_data.access_token,
            token_data.expires_in,
        );

        let loaded_token =
            load_cached_token("https://example.com", &["scope1".to_string()]).unwrap();
        assert_eq!(loaded_token, token_data.access_token);

        Ok(())
    }
}
