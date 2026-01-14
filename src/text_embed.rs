//! Local text embedding provider using ONNX Runtime.
//!
//! This module provides text-only embedding generation using local ONNX models,
//! enabling semantic search without cloud APIs. It follows the same patterns as
//! the CLIP implementation for consistency.
//!
//! ## Supported Models
//!
//! - **BGE-small-en-v1.5** (default): 384 dimensions, fast and efficient
//! - **BGE-base-en-v1.5**: 768 dimensions, better quality
//! - **nomic-embed-text-v1.5**: 768 dimensions, versatile
//! - **GTE-large**: 1024 dimensions, highest quality
//!
//! ## Usage
//!
//! ```ignore
//! use memvid_core::text_embed::{LocalTextEmbedder, TextEmbedConfig};
//!
//! let config = TextEmbedConfig::default(); // Uses BGE-small
//! let embedder = LocalTextEmbedder::new(config)?;
//!
//! let embedding = embedder.embed_text("hello world")?;
//! assert_eq!(embedding.len(), 384);
//! ```

use crate::types::embedding::EmbeddingProvider;
use crate::{MemvidError, Result};
use ndarray::Array;
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokenizers::tokenizer::{Tokenizer, TruncationParams};
use tokenizers::{
    PaddingDirection, PaddingParams, PaddingStrategy, TruncationDirection, TruncationStrategy,
};

// ============================================================================
// Configuration Constants
// ============================================================================

/// Default directory for storing text embedding models

/// Maximum sequence length for text embedding models (standard for BERT-based models)
const MAX_SEQUENCE_LENGTH: usize = 512;

/// Model unload timeout - unload after 5 minutes of inactivity
pub const MODEL_UNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

// ============================================================================
// Model Registry
// ============================================================================

/// Available text embedding models with verified HuggingFace URLs
#[derive(Debug, Clone)]
pub struct TextEmbedModelInfo {
    /// Model identifier
    pub name: &'static str,
    /// HuggingFace URL for ONNX model
    pub model_url: &'static str,
    /// HuggingFace URL for tokenizer
    pub tokenizer_url: &'static str,
    /// Embedding dimensions
    pub dims: u32,
    /// Maximum token length
    pub max_tokens: usize,
    /// Whether this is the default model
    pub is_default: bool,
}

/// Available text embedding models registry
pub static TEXT_EMBED_MODELS: &[TextEmbedModelInfo] = &[
    // BGE-small: Default, fast, good quality (384d)
    TextEmbedModelInfo {
        name: "bge-small-en-v1.5",
        model_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx",
        tokenizer_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json",
        dims: 384,
        max_tokens: 512,
        is_default: true,
    },
    // BGE-base: Better quality, still fast (768d)
    TextEmbedModelInfo {
        name: "bge-base-en-v1.5",
        model_url: "https://huggingface.co/BAAI/bge-base-en-v1.5/resolve/main/onnx/model.onnx",
        tokenizer_url: "https://huggingface.co/BAAI/bge-base-en-v1.5/resolve/main/tokenizer.json",
        dims: 768,
        max_tokens: 512,
        is_default: false,
    },
    // Nomic: Versatile, good for various tasks (768d)
    TextEmbedModelInfo {
        name: "nomic-embed-text-v1.5",
        model_url: "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/onnx/model.onnx",
        tokenizer_url: "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/tokenizer.json",
        dims: 768,
        max_tokens: 512,
        is_default: false,
    },
    // GTE-large: Highest quality, slower (1024d)
    TextEmbedModelInfo {
        name: "gte-large",
        model_url: "https://huggingface.co/thenlper/gte-large/resolve/main/onnx/model.onnx",
        tokenizer_url: "https://huggingface.co/thenlper/gte-large/resolve/main/tokenizer.json",
        dims: 1024,
        max_tokens: 512,
        is_default: false,
    },
];

/// Get model info by name, defaults to bge-small-en-v1.5
#[must_use]
pub fn get_text_model_info(name: &str) -> &'static TextEmbedModelInfo {
    TEXT_EMBED_MODELS
        .iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| default_text_model_info())
}

/// Get the default model info
#[must_use]
pub fn default_text_model_info() -> &'static TextEmbedModelInfo {
    TEXT_EMBED_MODELS
        .iter()
        .find(|m| m.is_default)
        .expect("No default text embedding model configured")
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for local text embedding provider
#[derive(Debug, Clone)]
pub struct TextEmbedConfig {
    /// Model name to use
    pub model_name: String,
    /// Directory to store/load ONNX models and tokenizers
    pub models_dir: PathBuf,
    /// Offline mode - don't attempt downloads, fail if model missing
    pub offline: bool,
}

impl Default for TextEmbedConfig {
    fn default() -> Self {
        let models_dir = dirs_next::cache_dir()
            .map(|p| p.join("memvid").join("text-models"))
            .unwrap_or_else(|| {
                // Fallback to local directory if cache dir not available
                PathBuf::from(".memvid-cache/text-models")
            });

        Self {
            model_name: default_text_model_info().name.to_string(),
            models_dir,
            offline: true, // Default to offline (no auto-download)
        }
    }
}

impl TextEmbedConfig {
    /// Create config for BGE-small model (default)
    #[must_use]
    pub fn bge_small() -> Self {
        Self {
            model_name: "bge-small-en-v1.5".to_string(),
            ..Default::default()
        }
    }

    /// Create config for BGE-base model
    #[must_use]
    pub fn bge_base() -> Self {
        Self {
            model_name: "bge-base-en-v1.5".to_string(),
            ..Default::default()
        }
    }

    /// Create config for Nomic model
    #[must_use]
    pub fn nomic() -> Self {
        Self {
            model_name: "nomic-embed-text-v1.5".to_string(),
            ..Default::default()
        }
    }

    /// Create config for GTE-large model
    #[must_use]
    pub fn gte_large() -> Self {
        Self {
            model_name: "gte-large".to_string(),
            ..Default::default()
        }
    }
}

// ============================================================================
// Local Text Embedder
// ============================================================================

/// Local text embedding provider using ONNX Runtime
///
/// This struct provides text embedding generation using local ONNX models.
/// Models are lazy-loaded on first use and automatically unloaded after
/// a period of inactivity to minimize memory usage.
pub struct LocalTextEmbedder {
    config: TextEmbedConfig,
    model_info: &'static TextEmbedModelInfo,
    /// Lazy-loaded ONNX session
    session: Mutex<Option<Session>>,
    /// Lazy-loaded tokenizer
    tokenizer: Mutex<Option<Tokenizer>>,
    /// Last time the model was used (for idle unloading)
    last_used: Mutex<Instant>,
}

impl LocalTextEmbedder {
    /// Create a new text embedder with the given configuration
    pub fn new(config: TextEmbedConfig) -> Result<Self> {
        let model_info = get_text_model_info(&config.model_name);

        Ok(Self {
            config,
            model_info,
            session: Mutex::new(None),
            tokenizer: Mutex::new(None),
            last_used: Mutex::new(Instant::now()),
        })
    }

    /// Get model info
    #[must_use]
    pub fn model_info(&self) -> &'static TextEmbedModelInfo {
        self.model_info
    }

    /// Ensure model file exists, returning error with download instructions if not
    fn ensure_model_file(&self) -> Result<PathBuf> {
        let filename = format!("{}.onnx", self.model_info.name);
        let path = self.config.models_dir.join(&filename);

        if path.exists() {
            return Ok(path);
        }

        // Model file doesn't exist
        Err(MemvidError::EmbeddingFailed {
            reason: format!(
                "Text embedding model not found at {}. Please download manually:\n\
                 mkdir -p {}\n\
                 curl -L '{}' -o '{}'",
                path.display(),
                self.config.models_dir.display(),
                self.model_info.model_url,
                path.display()
            )
            .into(),
        })
    }

    /// Ensure tokenizer file exists, returning error with download instructions if not
    fn ensure_tokenizer_file(&self) -> Result<PathBuf> {
        let filename = format!("{}_tokenizer.json", self.model_info.name);
        let path = self.config.models_dir.join(&filename);

        if path.exists() {
            return Ok(path);
        }

        // Tokenizer file doesn't exist
        Err(MemvidError::EmbeddingFailed {
            reason: format!(
                "Tokenizer not found at {}. Please download manually:\n\
                 curl -L '{}' -o '{}'",
                path.display(),
                self.model_info.tokenizer_url,
                path.display()
            )
            .into(),
        })
    }

    /// Load ONNX session lazily
    fn load_session(&self) -> Result<()> {
        let mut session_guard = self
            .session
            .lock()
            .map_err(|_| MemvidError::Lock("Failed to lock text embed session".into()))?;

        if session_guard.is_some() {
            return Ok(());
        }

        let model_path = self.ensure_model_file()?;

        tracing::debug!(path = %model_path.display(), "Loading text embedding model");

        let session = Session::builder()
            .map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to create session builder: {}", e).into(),
            })?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to set optimization level: {}", e).into(),
            })?
            .with_intra_threads(4)
            .map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to set intra threads: {}", e).into(),
            })?
            .commit_from_file(&model_path)
            .map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to load text embedding model: {}", e).into(),
            })?;

        *session_guard = Some(session);
        tracing::info!(model = %self.model_info.name, "Text embedding model loaded");

        Ok(())
    }

    /// Load tokenizer lazily
    fn load_tokenizer(&self) -> Result<()> {
        let mut tokenizer_guard = self
            .tokenizer
            .lock()
            .map_err(|_| MemvidError::Lock("Failed to lock tokenizer".into()))?;

        if tokenizer_guard.is_some() {
            return Ok(());
        }

        let tokenizer_path = self.ensure_tokenizer_file()?;

        tracing::debug!(path = %tokenizer_path.display(), "Loading tokenizer");

        let mut tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to load tokenizer: {}", e).into(),
            })?;

        // Configure padding to max sequence length
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(MAX_SEQUENCE_LENGTH),
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: 0,
            pad_type_id: 0,
            pad_token: "[PAD]".to_string(),
        }));

        // Configure truncation
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQUENCE_LENGTH,
                strategy: TruncationStrategy::LongestFirst,
                stride: 0,
                direction: TruncationDirection::Right,
            }))
            .map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to apply truncation config: {}", e).into(),
            })?;

        *tokenizer_guard = Some(tokenizer);
        tracing::info!(model = %self.model_info.name, "Tokenizer loaded");

        Ok(())
    }

    /// Encode text to embedding
    pub fn encode_text(&self, text: &str) -> Result<Vec<f32>> {
        // Ensure session and tokenizer are loaded
        self.load_session()?;
        self.load_tokenizer()?;

        // Tokenize the text
        let encoding = {
            let tokenizer_guard = self
                .tokenizer
                .lock()
                .map_err(|_| MemvidError::Lock("Failed to lock tokenizer".into()))?;
            let tokenizer =
                tokenizer_guard
                    .as_ref()
                    .ok_or_else(|| MemvidError::EmbeddingFailed {
                        reason: "Tokenizer not loaded".into(),
                    })?;

            tokenizer
                .encode(text, true)
                .map_err(|e| MemvidError::EmbeddingFailed {
                    reason: format!("Text tokenization failed: {}", e).into(),
                })?
        };

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|id| *id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|id| *id as i64)
            .collect();
        let token_type_ids: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .map(|id| *id as i64)
            .collect();
        let max_length = input_ids.len();

        // Create input arrays
        let input_ids_array = Array::from_shape_vec((1, max_length), input_ids).map_err(|e| {
            MemvidError::EmbeddingFailed {
                reason: format!("Failed to create input_ids array: {}", e).into(),
            }
        })?;
        let attention_mask_array =
            Array::from_shape_vec((1, max_length), attention_mask).map_err(|e| {
                MemvidError::EmbeddingFailed {
                    reason: format!("Failed to create attention_mask array: {}", e).into(),
                }
            })?;
        let token_type_ids_array =
            Array::from_shape_vec((1, max_length), token_type_ids).map_err(|e| {
                MemvidError::EmbeddingFailed {
                    reason: format!("Failed to create token_type_ids array: {}", e).into(),
                }
            })?;

        // Update last used timestamp
        if let Ok(mut last) = self.last_used.lock() {
            *last = Instant::now();
        }

        // Run inference
        let mut session_guard = self
            .session
            .lock()
            .map_err(|_| MemvidError::Lock("Failed to lock session".into()))?;

        let session = session_guard
            .as_mut()
            .ok_or_else(|| MemvidError::EmbeddingFailed {
                reason: "Session not loaded".into(),
            })?;

        // Get input and output names from session
        let input_names: Vec<String> = session.inputs.iter().map(|i| i.name.clone()).collect();
        let output_name = session
            .outputs
            .first()
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "last_hidden_state".to_string());

        // Create tensors
        let input_ids_tensor =
            Tensor::from_array(input_ids_array).map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to create input_ids tensor: {}", e).into(),
            })?;
        let attention_mask_tensor =
            Tensor::from_array(attention_mask_array).map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to create attention_mask tensor: {}", e).into(),
            })?;
        let token_type_ids_tensor =
            Tensor::from_array(token_type_ids_array).map_err(|e| MemvidError::EmbeddingFailed {
                reason: format!("Failed to create token_type_ids tensor: {}", e).into(),
            })?;

        // Build inputs based on what the model expects
        let outputs = if input_names.len() >= 3 {
            // Full BERT model with token_type_ids
            session
                .run(ort::inputs![
                    input_names[0].clone() => input_ids_tensor,
                    input_names[1].clone() => attention_mask_tensor,
                    input_names[2].clone() => token_type_ids_tensor
                ])
                .map_err(|e| MemvidError::EmbeddingFailed {
                    reason: format!("Text inference failed: {}", e).into(),
                })?
        } else if input_names.len() >= 2 {
            // Model without token_type_ids (some variants)
            session
                .run(ort::inputs![
                    input_names[0].clone() => input_ids_tensor,
                    input_names[1].clone() => attention_mask_tensor
                ])
                .map_err(|e| MemvidError::EmbeddingFailed {
                    reason: format!("Text inference failed: {}", e).into(),
                })?
        } else {
            // Single input model
            let name = input_names
                .first()
                .cloned()
                .unwrap_or_else(|| "input_ids".to_string());
            session
                .run(ort::inputs![name => input_ids_tensor])
                .map_err(|e| MemvidError::EmbeddingFailed {
                    reason: format!("Text inference failed: {}", e).into(),
                })?
        };

        // Extract embeddings from output
        let output = outputs
            .get(&output_name)
            .ok_or_else(|| MemvidError::EmbeddingFailed {
                reason: format!("No output '{}' from model", output_name).into(),
            })?;

        let (_shape, data) =
            output
                .try_extract_tensor::<f32>()
                .map_err(|e| MemvidError::EmbeddingFailed {
                    reason: format!("Failed to extract embeddings: {}", e).into(),
                })?;

        // For BERT-style models, use [CLS] token embedding (first token)
        // The output shape is typically [batch_size, sequence_length, hidden_size]
        let embedding_dim = self.model_info.dims as usize;
        let embedding: Vec<f32> = data.iter().take(embedding_dim).copied().collect();

        if embedding.iter().any(|v| !v.is_finite()) {
            return Err(MemvidError::EmbeddingFailed {
                reason: "Text embedding contains non-finite values".into(),
            });
        }

        // L2 normalize
        let normalized = l2_normalize(&embedding);

        tracing::debug!(
            text_len = text.len(),
            dims = normalized.len(),
            "Generated text embedding"
        );

        Ok(normalized)
    }

    /// Encode multiple texts in batch
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.encode_text(text)?);
        }
        Ok(embeddings)
    }

    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.session.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Maybe unload model if unused for too long (memory management)
    pub fn maybe_unload(&self) -> Result<()> {
        let last_used = self
            .last_used
            .lock()
            .map_err(|_| MemvidError::Lock("Failed to check last_used".into()))?;

        if last_used.elapsed() > MODEL_UNLOAD_TIMEOUT {
            tracing::debug!(model = %self.model_info.name, "Model idle, unloading");

            // Unload session
            if let Ok(mut guard) = self.session.lock() {
                *guard = None;
            }

            // Unload tokenizer
            if let Ok(mut guard) = self.tokenizer.lock() {
                *guard = None;
            }
        }

        Ok(())
    }

    /// Force unload model and tokenizer
    pub fn unload(&self) -> Result<()> {
        if let Ok(mut guard) = self.session.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.tokenizer.lock() {
            *guard = None;
        }
        tracing::debug!(model = %self.model_info.name, "Text embedding model unloaded");
        Ok(())
    }
}

// ============================================================================
// EmbeddingProvider Implementation
// ============================================================================

impl EmbeddingProvider for LocalTextEmbedder {
    fn kind(&self) -> &str {
        "local"
    }

    fn model(&self) -> &str {
        self.model_info.name
    }

    fn dimension(&self) -> usize {
        self.model_info.dims as usize
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.encode_text(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.encode_batch(texts)
    }

    fn is_ready(&self) -> bool {
        // Models are lazy-loaded, so always "ready"
        true
    }

    fn init(&mut self) -> Result<()> {
        // Lazy loading, no explicit init needed
        Ok(())
    }
}

// ============================================================================
// Utilities
// ============================================================================

/// L2 normalize a vector (unit length)
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm.is_finite() && norm > 1e-10 {
        v.iter().map(|x| x / norm).collect()
    } else {
        // Fall back to zeros to avoid NaNs propagating through distances
        vec![0.0; v.len()]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry() {
        assert_eq!(TEXT_EMBED_MODELS.len(), 4);

        let default_model = default_text_model_info();
        assert_eq!(default_model.name, "bge-small-en-v1.5");
        assert_eq!(default_model.dims, 384);
        assert!(default_model.is_default);
    }

    #[test]
    fn test_get_model_info() {
        let bge_small = get_text_model_info("bge-small-en-v1.5");
        assert_eq!(bge_small.dims, 384);

        let bge_base = get_text_model_info("bge-base-en-v1.5");
        assert_eq!(bge_base.dims, 768);

        let nomic = get_text_model_info("nomic-embed-text-v1.5");
        assert_eq!(nomic.dims, 768);

        let gte = get_text_model_info("gte-large");
        assert_eq!(gte.dims, 1024);

        // Unknown model should return default
        let unknown = get_text_model_info("unknown-model");
        assert_eq!(unknown.name, "bge-small-en-v1.5");
    }

    #[test]
    fn test_config_defaults() {
        let config = TextEmbedConfig::default();
        assert_eq!(config.model_name, "bge-small-en-v1.5");
        assert!(config.offline);

        let bge_small = TextEmbedConfig::bge_small();
        assert_eq!(bge_small.model_name, "bge-small-en-v1.5");

        let bge_base = TextEmbedConfig::bge_base();
        assert_eq!(bge_base.model_name, "bge-base-en-v1.5");

        let nomic = TextEmbedConfig::nomic();
        assert_eq!(nomic.model_name, "nomic-embed-text-v1.5");

        let gte = TextEmbedConfig::gte_large();
        assert_eq!(gte.model_name, "gte-large");
    }

    #[test]
    fn test_l2_normalize() {
        let v = vec![3.0, 4.0];
        let normalized = l2_normalize(&v);
        assert_eq!(normalized.len(), 2);
        // 3/5 = 0.6, 4/5 = 0.8
        assert!((normalized[0] - 0.6).abs() < 1e-6);
        assert!((normalized[1] - 0.8).abs() < 1e-6);

        // Test zero vector
        let zero = vec![0.0, 0.0];
        let normalized_zero = l2_normalize(&zero);
        assert_eq!(normalized_zero, vec![0.0, 0.0]);
    }

    #[test]
    fn test_embed_provider_trait() {
        let config = TextEmbedConfig::default();
        let embedder = LocalTextEmbedder::new(config).unwrap();

        assert_eq!(embedder.kind(), "local");
        assert_eq!(embedder.model(), "bge-small-en-v1.5");
        assert_eq!(embedder.dimension(), 384);
        assert!(embedder.is_ready());
    }
}
