use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rspack_cacheable::{
  cacheable,
  with::{AsCacheable, AsMap, AsPreset},
};
use rspack_collections::{Identifier, IdentifierDashMap};
use rspack_error::Result;
use rspack_hash::RspackHashDigest;
use rspack_sources::BoxSource;
use rustc_hash::FxHashMap;

use super::{
  super::{codec::CacheCodec, storage::Storage},
  Occasion,
};
use crate::{BindingCell, CodeGenerationResult, RayonConsumer, RuntimeGlobals, SourceType};

pub const SCOPE: &str = "occasion_code_generate";

#[cacheable]
struct Entry {
  #[cacheable(with=AsMap<AsCacheable, AsPreset>)]
  pub sources: FxHashMap<SourceType, BoxSource>,
  pub runtime_requirements: RuntimeGlobals,
  pub hash: Option<RspackHashDigest>,
}

#[derive(Debug, Default)]
pub struct CodeGeneratePersistentCacheArtifact {
  entries: IdentifierDashMap<CachedCodeGenerationResult>,
  dirty_keys: Mutex<Vec<Identifier>>,
}

#[derive(Debug, Clone)]
pub struct CachedCodeGenerationResult {
  pub sources: FxHashMap<SourceType, BoxSource>,
  pub runtime_requirements: RuntimeGlobals,
  pub hash: Option<RspackHashDigest>,
}

impl CachedCodeGenerationResult {
  pub fn from_code_generation_result(value: &CodeGenerationResult) -> Option<Self> {
    if !value.data.is_empty()
      || !value.chunk_init_fragments.is_empty()
      || value.concatenation_scope.is_some()
    {
      return None;
    }

    Some(Self {
      sources: value.inner.as_ref().clone(),
      runtime_requirements: value.runtime_requirements,
      hash: value.hash.clone(),
    })
  }

  pub fn into_code_generation_result(self) -> CodeGenerationResult {
    CodeGenerationResult {
      inner: BindingCell::from(self.sources),
      data: Default::default(),
      chunk_init_fragments: Default::default(),
      runtime_requirements: self.runtime_requirements,
      hash: self.hash,
      id: Default::default(),
      concatenation_scope: None,
    }
  }
}

impl CodeGeneratePersistentCacheArtifact {
  pub fn get(&self, key: &Identifier) -> Option<CachedCodeGenerationResult> {
    self.entries.get(key).map(|entry| entry.clone())
  }

  pub fn insert(&self, key: Identifier, entry: CachedCodeGenerationResult) {
    self.entries.insert(key, entry);
    self
      .dirty_keys
      .lock()
      .expect("code generate persistent cache dirty keys mutex should not be poisoned")
      .push(key);
  }

  fn dirty_keys(&self) -> Vec<Identifier> {
    self
      .dirty_keys
      .lock()
      .expect("code generate persistent cache dirty keys mutex should not be poisoned")
      .clone()
  }
}

#[derive(Debug)]
pub struct CodeGenerateOccasion {
  codec: Arc<CacheCodec>,
}

impl CodeGenerateOccasion {
  pub fn new(codec: Arc<CacheCodec>) -> Self {
    Self { codec }
  }
}

impl Occasion for CodeGenerateOccasion {
  type Artifact = CodeGeneratePersistentCacheArtifact;

  fn name(&self) -> &'static str {
    "code generate"
  }

  #[tracing::instrument(name = "Cache::Occasion::CodeGenerate::reset", skip_all)]
  fn reset(&self, storage: &mut dyn Storage) {
    storage.reset(SCOPE);
  }

  #[tracing::instrument(name = "Cache::Occasion::CodeGenerate::save", skip_all)]
  fn save(&self, storage: &mut dyn Storage, artifact: &CodeGeneratePersistentCacheArtifact) {
    let dirty_keys = artifact.dirty_keys();
    dirty_keys
      .par_iter()
      .filter_map(|key| {
        let entry = artifact.entries.get(key)?;
        let storage_entry = Entry {
          sources: entry.sources.clone(),
          runtime_requirements: entry.runtime_requirements,
          hash: entry.hash.clone(),
        };
        match self.codec.encode(&storage_entry) {
          Ok(bytes) => Some((key.as_bytes().to_vec(), bytes)),
          Err(err) => {
            tracing::warn!("code generate persistent cache encode failed: {:?}", err);
            None
          }
        }
      })
      .consume(|(key, bytes)| {
        storage.set(SCOPE, key, bytes);
      });

    tracing::debug!(
      "saved {} code generate persistent cache entries",
      dirty_keys.len()
    );
  }

  #[tracing::instrument(name = "Cache::Occasion::CodeGenerate::recovery", skip_all)]
  async fn recovery(&self, storage: &dyn Storage) -> Result<CodeGeneratePersistentCacheArtifact> {
    let items = storage.load(SCOPE).await?;
    let artifact = CodeGeneratePersistentCacheArtifact::default();

    for (key, value) in items {
      let Ok(key) = String::from_utf8(key) else {
        tracing::warn!("code generate persistent cache key is not valid utf-8");
        continue;
      };
      match self.codec.decode::<Entry>(&value) {
        Ok(entry) => {
          artifact.entries.insert(
            Identifier::from(key),
            CachedCodeGenerationResult {
              sources: entry.sources,
              runtime_requirements: entry.runtime_requirements,
              hash: entry.hash,
            },
          );
        }
        Err(err) => {
          tracing::warn!("code generate persistent cache decode failed: {:?}", err);
        }
      }
    }

    tracing::debug!(
      "recovered {} code generate persistent cache entries",
      artifact.entries.len()
    );
    Ok(artifact)
  }
}

#[cfg(test)]
mod tests {
  use rspack_sources::{RawStringSource, SourceExt};

  use super::CachedCodeGenerationResult;
  use crate::{CodeGenerationDataUrl, CodeGenerationResult, SourceType};

  #[test]
  fn should_convert_cacheable_code_generation_result() {
    let result = CodeGenerationResult::default()
      .with_javascript(RawStringSource::from_static("console.log(1);").boxed());

    let cached = CachedCodeGenerationResult::from_code_generation_result(&result)
      .expect("result without extra state should be cacheable");
    let restored = cached.into_code_generation_result();

    assert_eq!(
      restored
        .get(&SourceType::JavaScript)
        .expect("should restore javascript source")
        .source()
        .into_string_lossy(),
      "console.log(1);"
    );
    assert!(restored.data.is_empty());
    assert!(restored.chunk_init_fragments.is_empty());
    assert!(restored.concatenation_scope.is_none());
  }

  #[test]
  fn should_skip_code_generation_result_with_extra_data() {
    let mut result = CodeGenerationResult::default()
      .with_javascript(RawStringSource::from_static("console.log(1);").boxed());
    result
      .data
      .insert(CodeGenerationDataUrl::new("asset-url".to_string()));

    assert!(CachedCodeGenerationResult::from_code_generation_result(&result).is_none());
  }
}
