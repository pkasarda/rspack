use futures::Future;
use rspack_collections::Identifier;
use rspack_error::Result;

use crate::{
  ArtifactExt, CacheOptions, CodeGenerationJob, CodeGenerationResult, CompilerOptions,
  MemoryGCStorage,
  cache::persistent::occasion::{CachedCodeGenerationResult, CodeGeneratePersistentCacheArtifact},
  incremental::{Incremental, IncrementalPasses},
};

#[derive(Debug, Default)]
pub struct CodeGenerateCacheArtifact {
  storage: Option<MemoryGCStorage<CodeGenerationResult>>,
  persistent_cache_artifact: Option<CodeGeneratePersistentCacheArtifact>,
}

impl ArtifactExt for CodeGenerateCacheArtifact {
  const PASS: IncrementalPasses = IncrementalPasses::MODULES_CODEGEN;

  fn recover(_incremental: &Incremental, new: &mut Self, old: &mut Self) {
    *new = std::mem::take(old);
    new.start_next_generation();
  }
}

impl CodeGenerateCacheArtifact {
  pub fn new(options: &CompilerOptions) -> Self {
    Self {
      storage: match &options.cache {
        CacheOptions::Memory { max_generations } => Some(MemoryGCStorage::new(*max_generations)),
        CacheOptions::Persistent(_) => Some(MemoryGCStorage::new(1)),
        CacheOptions::Disabled => None,
      },
      persistent_cache_artifact: None,
    }
  }

  pub fn set_persistent_cache_artifact(&mut self, artifact: CodeGeneratePersistentCacheArtifact) {
    self.persistent_cache_artifact = Some(artifact);
  }

  pub fn persistent_cache_artifact(&self) -> Option<&CodeGeneratePersistentCacheArtifact> {
    self.persistent_cache_artifact.as_ref()
  }

  pub fn start_next_generation(&self) {
    if let Some(storage) = &self.storage {
      storage.start_next_generation();
    }
  }

  pub async fn use_cache<G, F>(
    &self,
    job: &CodeGenerationJob,
    generator: G,
  ) -> (Result<CodeGenerationResult>, bool)
  where
    G: FnOnce() -> F,
    F: Future<Output = Result<CodeGenerationResult>>,
  {
    let Some(storage) = &self.storage else {
      let res = generator().await;
      return (res, false);
    };

    let cache_key = Identifier::from(format!("{}|{}", job.module, job.hash.encoded()));
    if let Some(value) = storage.get(&cache_key) {
      (Ok(value), true)
    } else if let Some(value) = self.get_persistent_cache(job) {
      storage.set(cache_key, value.clone());
      (Ok(value), true)
    } else {
      match generator().await {
        Ok(res) => {
          storage.set(cache_key, res.clone());
          self.set_persistent_cache(job, &res);
          (Ok(res), false)
        }
        Err(err) => (Err(err), false),
      }
    }
  }

  fn get_persistent_cache(&self, job: &CodeGenerationJob) -> Option<CodeGenerationResult> {
    let artifact = self.persistent_cache_artifact.as_ref()?;
    job
      .runtimes
      .iter()
      .find_map(|runtime| {
        let key = persistent_cache_key(job, runtime);
        artifact.get(&key)
      })
      .map(CachedCodeGenerationResult::into_code_generation_result)
  }

  fn set_persistent_cache(&self, job: &CodeGenerationJob, result: &CodeGenerationResult) {
    let Some(artifact) = &self.persistent_cache_artifact else {
      return;
    };
    let Some(entry) = CachedCodeGenerationResult::from_code_generation_result(result) else {
      return;
    };

    for runtime in &job.runtimes {
      artifact.insert(persistent_cache_key(job, runtime), entry.clone());
    }
  }
}

fn persistent_cache_key(job: &CodeGenerationJob, runtime: &crate::RuntimeSpec) -> Identifier {
  let module = job.module.as_str();
  let runtime = runtime.as_str();
  let hash = job.hash.encoded();
  Identifier::from(format!(
    "{}:{module}{}:{runtime}{hash}",
    module.len(),
    runtime.len()
  ))
}
