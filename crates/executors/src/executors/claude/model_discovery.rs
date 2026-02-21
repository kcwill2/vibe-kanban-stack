//! Fetches available models from Anthropic and Gemini APIs for the model selector.

use crate::model_selector::ModelInfo;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const GEMINI_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Option<Vec<AnthropicModel>>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelsResponse {
    models: Option<Vec<GeminiModel>>,
}

#[derive(Debug, Deserialize)]
struct GeminiModel {
    name: String,
    display_name: Option<String>,
}

/// Fetch models from Anthropic API. Returns empty vec on error.
pub async fn fetch_anthropic_models(api_key: &str) -> Vec<ModelInfo> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let res = client
        .get(ANTHROPIC_MODELS_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    let Ok(res) = res else {
        tracing::debug!("Anthropic models fetch failed: {:?}", res.err());
        return vec![];
    };

    if !res.status().is_success() {
        tracing::debug!(
            "Anthropic models API returned {}: {}",
            res.status(),
            res.text().await.unwrap_or_default()
        );
        return vec![];
    }

    let body: AnthropicModelsResponse = match res.json().await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!("Anthropic models parse error: {}", e);
            return vec![];
        }
    };

    let Some(data) = body.data else {
        return vec![];
    };

    let models: Vec<ModelInfo> = data
        .into_iter()
        .map(|m| ModelInfo {
            id: format!("anthropic,{}", m.id),
            name: m.display_name.unwrap_or_else(|| m.id.clone()),
            provider_id: None,
            reasoning_options: vec![],
        })
        .collect();

    tracing::debug!("Fetched {} Anthropic models", models.len());
    models
}

/// Fetch models from Gemini API. Returns empty vec on error.
pub async fn fetch_gemini_models(api_key: &str) -> Vec<ModelInfo> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = format!("{}?key={}", GEMINI_MODELS_URL, api_key);
    let res = client.get(&url).send().await;

    let Ok(res) = res else {
        tracing::debug!("Gemini models fetch failed: {:?}", res.err());
        return vec![];
    };

    if !res.status().is_success() {
        tracing::debug!(
            "Gemini models API returned {}: {}",
            res.status(),
            res.text().await.unwrap_or_default()
        );
        return vec![];
    }

    let body: GeminiModelsResponse = match res.json().await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!("Gemini models parse error: {}", e);
            return vec![];
        }
    };

    let Some(models_data) = body.models else {
        return vec![];
    };

    let models: Vec<ModelInfo> = models_data
        .into_iter()
        .filter_map(|m| {
            // name is like "models/gemini-2.0-flash" - strip "models/" for CCR format
            let id = m
                .name
                .strip_prefix("models/")
                .unwrap_or(&m.name)
                .to_string();
            // Skip embedding/generation-only models; keep chat-capable ones
            if id.contains("embedding") || id.contains("embed") {
                return None;
            }
            Some(ModelInfo {
                id: format!("gemini,{}", id),
                name: m.display_name.unwrap_or_else(|| id.clone()),
                provider_id: None,
                reasoning_options: vec![],
            })
        })
        .collect();

    tracing::debug!("Fetched {} Gemini models", models.len());
    models
}

/// Fetch models from both APIs in parallel. Merges results, with API-fetched models first,
/// then fallback models. Deduplicates by id.
pub async fn fetch_models_from_apis(
    anthropic_key: Option<&str>,
    gemini_key: Option<&str>,
    fallback_models: Vec<ModelInfo>,
) -> Vec<ModelInfo> {
    let a_key = anthropic_key
        .filter(|k| !k.is_empty())
        .map(|k| k.to_string());
    let g_key = gemini_key
        .filter(|k| !k.is_empty())
        .map(|k| k.to_string());

    let (anthropic_models, gemini_models) = tokio::join!(
        async move {
            if let Some(k) = a_key {
                fetch_anthropic_models(&k).await
            } else {
                vec![]
            }
        },
        async move {
            if let Some(k) = g_key {
                fetch_gemini_models(&k).await
            } else {
                vec![]
            }
        },
    );

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for m in anthropic_models.into_iter().chain(gemini_models) {
        if seen.insert(m.id.clone()) {
            result.push(m);
        }
    }

    for m in fallback_models {
        if seen.insert(m.id.clone()) {
            result.push(m);
        }
    }

    result
}
