//! AI inference layer: ONNX Runtime for embeddings/classification, LLM for generative tasks.

#[cfg(feature = "onnx")]
mod embedder;
#[cfg(feature = "onnx")]
pub use embedder::Embedder;

#[cfg(feature = "onnx")]
mod extractor;
#[cfg(feature = "onnx")]
pub use extractor::{DrrpExtraction, DrrpExtractor};

pub mod classifier;
pub mod labels;
pub use classifier::{
    CentroidSummary, Classification, ClassificationStatus, Classifier, aggregate_law_embeddings,
};
pub use labels::{EXCLUDE_FAMILIES, LabelSet, LabelSummary};
