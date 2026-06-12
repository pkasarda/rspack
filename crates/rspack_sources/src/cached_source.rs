use std::{
  borrow::Cow,
  cell::OnceCell,
  hash::{Hash, Hasher},
  sync::{Arc, OnceLock},
};

use rspack_cacheable::{
  __private::rkyv::{
    Archive, Archived, Deserialize, Place, Serialize,
    munge::munge,
    option::ArchivedOption,
    rancor::Fallible,
    tuple::ArchivedTuple5,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
  },
  cacheable, cacheable_dyn,
  with::{AsInner, AsOption, Inline},
};
use rustc_hash::FxHasher;

use crate::{
  BoxSource, MapOptions, RawBufferSource, Source, SourceExt, SourceMap, SourceMapAsJson,
  helpers::{
    Chunks, GeneratedInfo, StreamChunks, TextSpan, stream_and_get_source_and_map,
    stream_chunks_of_raw_source, stream_chunks_of_source_map,
  },
  object_pool::ObjectPool,
  source::SourceValue,
};

#[derive(Default, Debug)]
struct CachedData {
  hash: OnceLock<u64>,
  size: OnceLock<usize>,
  is_ascii: OnceLock<bool>,
  chunks: OnceLock<Vec<&'static str>>,
  columns_map: OnceLock<Option<SourceMap>>,
  line_only_map: OnceLock<Option<SourceMap>>,
}

/// It tries to reused cached results from other methods to avoid calculations,
/// usually used after modify is finished.
///
/// - [webpack-sources docs](https://github.com/webpack/webpack-sources/#cachedsource).
///
/// ```
/// use rspack_sources::{
///   BoxSource, CachedSource, ConcatSource, MapOptions, OriginalSource, RawStringSource, Source,
///   SourceExt, SourceMap,
/// };
///
/// let mut concat = ConcatSource::new([
///   RawStringSource::from("Hello World\n".to_string()).boxed(),
///   OriginalSource::new(
///     "console.log('test');\nconsole.log('test2');\n",
///     "console.js",
///   )
///   .boxed(),
/// ]);
/// concat.add(OriginalSource::new("Hello2\n", "hello.md"));
///
/// let cached = CachedSource::new(concat);
///
/// assert_eq!(
///   cached.source().into_string_lossy(),
///   "Hello World\nconsole.log('test');\nconsole.log('test2');\nHello2\n"
/// );
/// // second time will be fast.
/// assert_eq!(
///   cached.source().into_string_lossy(),
///   "Hello World\nconsole.log('test');\nconsole.log('test2');\nHello2\n"
/// );
/// ```
#[cacheable]
pub struct CachedSource {
  inner: BoxSource,
  #[cacheable(with=AsInner<CachedDataAsCache>)]
  cache: Arc<CachedData>,
}

#[doc(hidden)]
pub struct CachedDataAsCache;

type CachedDataMapRef<'a> = Option<&'a Option<SourceMap>>;
type CachedDataMapAsRef = AsOption<Inline<AsOption<SourceMapAsJson>>>;
type CachedDataMapAsOwned = AsOption<AsOption<SourceMapAsJson>>;
type ArchivedCachedDataMap =
  ArchivedOption<ArchivedOption<<SourceMapAsJson as ArchiveWith<SourceMap>>::Archived>>;
type CachedDataMapResolver = Option<Option<<SourceMapAsJson as ArchiveWith<SourceMap>>::Resolver>>;

#[doc(hidden)]
pub type ArchivedCachedData = ArchivedTuple5<
  Archived<Option<u64>>,
  Archived<Option<usize>>,
  Archived<Option<bool>>,
  ArchivedCachedDataMap,
  ArchivedCachedDataMap,
>;

#[doc(hidden)]
pub struct CachedDataAsCacheResolver {
  hash: <Option<u64> as Archive>::Resolver,
  size: <Option<usize> as Archive>::Resolver,
  is_ascii: <Option<bool> as Archive>::Resolver,
  columns_map: CachedDataMapResolver,
  line_only_map: CachedDataMapResolver,
}

impl ArchiveWith<CachedData> for CachedDataAsCache {
  type Archived = ArchivedCachedData;
  type Resolver = CachedDataAsCacheResolver;

  #[inline]
  fn resolve_with(field: &CachedData, resolver: Self::Resolver, out: Place<Self::Archived>) {
    println!("Resolving CachedData: {:#?}", field);
    let hash = field.hash.get().copied();
    let size = field.size.get().copied();
    let is_ascii = field.is_ascii.get().copied();
    let columns_map = field.columns_map.get();
    let line_only_map = field.line_only_map.get();

    munge!(
      let ArchivedTuple5(
        out_hash,
        out_size,
        out_is_ascii,
        out_columns_map,
        out_line_only_map
      ) = out
    );
    hash.resolve(resolver.hash, out_hash);
    size.resolve(resolver.size, out_size);
    is_ascii.resolve(resolver.is_ascii, out_is_ascii);
    CachedDataMapAsRef::resolve_with(&columns_map, resolver.columns_map, out_columns_map);
    CachedDataMapAsRef::resolve_with(&line_only_map, resolver.line_only_map, out_line_only_map);
  }
}

impl<S> SerializeWith<CachedData, S> for CachedDataAsCache
where
  S: Fallible<Error = rspack_cacheable::Error> + ?Sized,
  Option<u64>: Serialize<S>,
  Option<usize>: Serialize<S>,
  Option<bool>: Serialize<S>,
  for<'a> CachedDataMapAsRef: ArchiveWith<CachedDataMapRef<'a>, Resolver = CachedDataMapResolver>
    + SerializeWith<CachedDataMapRef<'a>, S>,
{
  #[inline]
  fn serialize_with(
    field: &CachedData,
    serializer: &mut S,
  ) -> rspack_cacheable::Result<Self::Resolver> {
    let hash = field.hash.get().copied();
    let size = field.size.get().copied();
    let is_ascii = field.is_ascii.get().copied();
    let columns_map = field.columns_map.get();
    let line_only_map = field.line_only_map.get();

    Ok(CachedDataAsCacheResolver {
      hash: hash.serialize(serializer)?,
      size: size.serialize(serializer)?,
      is_ascii: is_ascii.serialize(serializer)?,
      columns_map: CachedDataMapAsRef::serialize_with(&columns_map, serializer)?,
      line_only_map: CachedDataMapAsRef::serialize_with(&line_only_map, serializer)?,
    })
  }
}

impl<D> DeserializeWith<ArchivedCachedData, CachedData, D> for CachedDataAsCache
where
  D: Fallible<Error = rspack_cacheable::Error> + ?Sized,
  Archived<Option<u64>>: Deserialize<Option<u64>, D>,
  Archived<Option<usize>>: Deserialize<Option<usize>, D>,
  Archived<Option<bool>>: Deserialize<Option<bool>, D>,
  CachedDataMapAsOwned: DeserializeWith<ArchivedCachedDataMap, Option<Option<SourceMap>>, D>,
{
  #[inline]
  fn deserialize_with(
    field: &ArchivedCachedData,
    deserializer: &mut D,
  ) -> rspack_cacheable::Result<CachedData> {
    let cache = CachedData::default();
    println!("Deserializing CachedData: {:#?}", cache);

    if let Some(hash) = field.0.deserialize(deserializer)? {
      let _ = cache.hash.set(hash);
    }
    if let Some(size) = field.1.deserialize(deserializer)? {
      let _ = cache.size.set(size);
    }
    if let Some(is_ascii) = field.2.deserialize(deserializer)? {
      let _ = cache.is_ascii.set(is_ascii);
    }
    if let Some(columns_map) = CachedDataMapAsOwned::deserialize_with(&field.3, deserializer)? {
      let _ = cache.columns_map.set(columns_map);
    }
    if let Some(line_only_map) = CachedDataMapAsOwned::deserialize_with(&field.4, deserializer)? {
      let _ = cache.line_only_map.set(line_only_map);
    }

    Ok(cache)
  }
}

impl CachedSource {
  /// Create a [CachedSource] with the original [Source].
  pub fn new<T: SourceExt>(inner: T) -> Self {
    let box_source = inner.boxed();
    // Check if it's already a BoxSource containing a CachedSource
    if let Some(cached_source) = box_source.as_ref().as_any().downcast_ref::<CachedSource>() {
      return cached_source.clone();
    }

    Self {
      inner: box_source,
      cache: Arc::new(CachedData::default()),
    }
  }

  /// Get the inner source.
  pub fn inner(&self) -> &BoxSource {
    &self.inner
  }

  fn get_or_init_chunks(&self) -> &[&str] {
    self.cache.chunks.get_or_init(|| {
      let mut chunks = Vec::new();
      self.inner.rope(&mut |chunk| {
        chunks.push(chunk);
      });
      #[allow(unsafe_code)]
      // SAFETY: CachedSource guarantees that the underlying source outlives the cache,
      // so transmuting Vec<&str> to Vec<&'static str> is safe in this context.
      // This allows us to store string slices in the cache without additional allocations.
      unsafe {
        std::mem::transmute::<Vec<&str>, Vec<&'static str>>(chunks)
      }
    })
  }

  fn is_ascii(&self) -> bool {
    *self.cache.is_ascii.get_or_init(|| {
      if let Some(chunks) = self.cache.chunks.get() {
        return chunks.iter().all(|chunk| chunk.is_ascii());
      }
      self.inner.source().as_bytes().is_ascii()
    })
  }
}

#[cacheable_dyn]
impl Source for CachedSource {
  fn source(&self) -> SourceValue<'_> {
    // Check if it's a RawBufferSource containing a CachedSource
    if let Some(buffer_source) = self
      .inner
      .as_ref()
      .as_any()
      .downcast_ref::<RawBufferSource>()
    {
      return buffer_source.source();
    }

    let chunks = self.get_or_init_chunks();
    let mut string = String::with_capacity(self.size());
    if self.cache.is_ascii.get().is_none() {
      let mut is_ascii = true;
      for chunk in chunks {
        if is_ascii {
          is_ascii = chunk.is_ascii();
        }
        string.push_str(chunk);
      }
      let _ = self.cache.is_ascii.set(is_ascii);
    } else {
      for chunk in chunks {
        string.push_str(chunk);
      }
    }
    SourceValue::String(Cow::Owned(string))
  }

  fn rope<'a>(&'a self, on_chunk: &mut dyn FnMut(&'a str)) {
    let chunks = self.get_or_init_chunks();
    chunks.iter().for_each(|chunk| on_chunk(chunk));
  }

  fn buffer(&self) -> Cow<'_, [u8]> {
    self.inner.buffer()
  }

  fn size(&self) -> usize {
    *self.cache.size.get_or_init(|| {
      if let Some(chunks) = self.cache.chunks.get() {
        return chunks.iter().fold(0, |acc, chunk| acc + chunk.len());
      }
      self.inner.size()
    })
  }

  fn map(&self, object_pool: &ObjectPool, options: &MapOptions) -> Option<SourceMap> {
    if options.columns {
      self
        .cache
        .columns_map
        .get_or_init(|| self.inner.map(object_pool, options))
        .clone()
    } else {
      self
        .cache
        .line_only_map
        .get_or_init(|| self.inner.map(object_pool, options))
        .clone()
    }
  }

  fn to_writer(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    self.inner.to_writer(writer)
  }
}

struct CachedSourceChunks<'source> {
  cache_source: &'source CachedSource,
  chunks: OnceCell<Box<dyn Chunks + 'source>>,
  source: OnceCell<Cow<'source, str>>,
}

impl<'source> CachedSourceChunks<'source> {
  fn new(cache_source: &'source CachedSource) -> Self {
    Self {
      cache_source,
      chunks: OnceCell::new(),
      source: OnceCell::new(),
    }
  }

  fn get_or_init_chunks(&self) -> &dyn Chunks {
    self
      .chunks
      .get_or_init(|| self.cache_source.inner.stream_chunks())
      .as_ref()
  }

  fn get_or_init_source(&self) -> TextSpan<'_> {
    let source = self
      .source
      .get_or_init(|| self.cache_source.source().into_string_lossy());
    let is_ascii = self.cache_source.is_ascii();
    TextSpan::with_known(source.as_ref(), is_ascii)
  }
}

impl Chunks for CachedSourceChunks<'_> {
  fn stream<'a>(
    &'a self,
    object_pool: &'a ObjectPool,
    options: &MapOptions,
    on_chunk: crate::helpers::OnChunk<'_, 'a>,
    on_source: crate::helpers::OnSource<'_, 'a>,
    on_name: crate::helpers::OnName<'_, 'a>,
  ) -> GeneratedInfo {
    let cell = if options.columns {
      &self.cache_source.cache.columns_map
    } else {
      &self.cache_source.cache.line_only_map
    };
    match cell.get() {
      Some(map) => {
        let source = self.get_or_init_source();
        if let Some(map) = map {
          stream_chunks_of_source_map(
            options,
            object_pool,
            source,
            map,
            on_chunk,
            on_source,
            on_name,
          )
        } else {
          stream_chunks_of_raw_source(source, options, on_chunk, on_source, on_name)
        }
      }
      None => {
        let (generated_info, map) = stream_and_get_source_and_map(
          options,
          object_pool,
          self.get_or_init_chunks(),
          on_chunk,
          on_source,
          on_name,
        );
        cell.get_or_init(|| map);
        generated_info
      }
    }
  }
}

impl StreamChunks for CachedSource {
  fn stream_chunks<'a>(&'a self) -> Box<dyn Chunks + 'a> {
    Box::new(CachedSourceChunks::new(self))
  }
}

impl Clone for CachedSource {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      cache: self.cache.clone(),
    }
  }
}

impl Hash for CachedSource {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (self.cache.hash.get_or_init(|| {
      let mut hasher = FxHasher::default();
      self.inner.hash(&mut hasher);
      hasher.finish()
    }))
    .hash(state);
  }
}

impl PartialEq for CachedSource {
  fn eq(&self, other: &Self) -> bool {
    self.inner.as_ref() == other.inner.as_ref()
  }
}

impl Eq for CachedSource {}

impl std::fmt::Debug for CachedSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    let indent = f.width().unwrap_or(0);
    let indent_str = format!("{:indent$}", "", indent = indent);

    writeln!(f, "{indent_str}CachedSource::new(")?;
    writeln!(
      f,
      "{indent_str}{:indent$?}",
      self.inner,
      indent = indent + 2
    )?;
    write!(f, "{indent_str}).boxed()")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    ConcatSource, OriginalSource, RawBufferSource, RawStringSource, ReplaceSource, SourceExt,
    SourceMapSource, WithoutOriginalOptions,
  };

  #[test]
  fn line_number_should_not_add_one() {
    let source = ConcatSource::new([
      CachedSource::new(RawStringSource::from("\n")).boxed(),
      SourceMapSource::new(WithoutOriginalOptions {
        value: "\nconsole.log(1);\n".to_string(),
        name: "index.js".to_string(),
        source_map: SourceMap::new(
          ";AACA",
          vec!["index.js".into()],
          vec!["// DELETE IT\nconsole.log(1)".into()],
          vec![],
        ),
      })
      .boxed(),
    ]);
    let map = source
      .map(&ObjectPool::default(), &Default::default())
      .unwrap();
    assert_eq!(map.mappings(), ";;AACA");
  }

  #[test]
  fn should_allow_to_store_and_share_cached_data() {
    let original = OriginalSource::new("Hello World", "test.txt");
    let source = CachedSource::new(original);
    let clone = source.clone();

    // fill up cache
    let map_options = MapOptions::default();
    source.source();
    source.buffer();
    source.size();
    source.map(&ObjectPool::default(), &map_options);

    assert_eq!(
      *clone.cache.columns_map.get().unwrap(),
      source.map(&ObjectPool::default(), &map_options)
    );
  }

  #[test]
  fn should_preserve_cached_data_when_cacheable_roundtrip() {
    #[rspack_cacheable::cacheable]
    struct Data(#[cacheable(with=rspack_cacheable::with::AsPreset)] BoxSource);

    let source = CachedSource::new(OriginalSource::new("const answer = 42;\n", "answer.js"));
    let object_pool = ObjectPool::default();
    let columns_options = MapOptions::new(true);
    let line_only_options = MapOptions::new(false);

    source.source();
    let expected_size = source.size();
    let expected_is_ascii = source.cache.is_ascii.get().copied();
    let mut hasher = FxHasher::default();
    source.hash(&mut hasher);
    let expected_hash = source.cache.hash.get().copied();
    let expected_columns_map = source.map(&object_pool, &columns_options);
    let expected_line_only_map = source.map(&object_pool, &line_only_options);

    assert!(source.cache.chunks.get().is_some());

    let bytes = rspack_cacheable::to_bytes(&Data(source.boxed()), &()).unwrap();
    let Data(restored) = rspack_cacheable::from_bytes(&bytes, &()).unwrap();
    let restored = restored
      .as_ref()
      .as_any()
      .downcast_ref::<CachedSource>()
      .unwrap();

    assert_eq!(restored.cache.size.get().copied(), Some(expected_size));
    assert_eq!(restored.cache.is_ascii.get().copied(), expected_is_ascii);
    assert_eq!(restored.cache.hash.get().copied(), expected_hash);
    assert_eq!(
      restored.cache.columns_map.get(),
      Some(&expected_columns_map)
    );
    assert_eq!(
      restored.cache.line_only_map.get(),
      Some(&expected_line_only_map)
    );
    assert!(restored.cache.chunks.get().is_none());
  }

  #[test]
  fn should_return_the_correct_size_for_binary_files() {
    let source = OriginalSource::new(String::from_utf8(vec![0; 256]).unwrap(), "file.wasm");
    let cached_source = CachedSource::new(source);

    assert_eq!(cached_source.size(), 256);
    assert_eq!(cached_source.size(), 256);
  }

  #[test]
  fn should_return_the_correct_size_for_cached_binary_files() {
    let source = OriginalSource::new(String::from_utf8(vec![0; 256]).unwrap(), "file.wasm");
    let cached_source = CachedSource::new(source);

    cached_source.source();
    assert_eq!(cached_source.size(), 256);
    assert_eq!(cached_source.size(), 256);
  }

  #[test]
  fn should_return_the_correct_size_for_text_files() {
    let source = OriginalSource::new("TestTestTest", "file.js");
    let cached_source = CachedSource::new(source);

    assert_eq!(cached_source.size(), 12);
    assert_eq!(cached_source.size(), 12);
  }

  #[test]
  fn should_return_the_correct_size_for_cached_text_files() {
    let source = OriginalSource::new("TestTestTest", "file.js");
    let cached_source = CachedSource::new(source);

    cached_source.source();
    assert_eq!(cached_source.size(), 12);
    assert_eq!(cached_source.size(), 12);
  }

  #[test]
  fn should_produce_correct_output_for_cached_raw_source() {
    let map_options = MapOptions::new(true);

    let source = RawStringSource::from("Test\nTest\nTest\n");
    let mut on_chunk_count = 0;
    let mut on_source_count = 0;
    let mut on_name_count = 0;
    let generated_info = {
      let object_pool = ObjectPool::default();
      let chunks = source.stream_chunks();
      chunks.stream(
        &object_pool,
        &map_options,
        &mut |_chunk, _mapping| {
          on_chunk_count += 1;
        },
        &mut |_source_index, _source, _source_content| {
          on_source_count += 1;
        },
        &mut |_name_index, _name| {
          on_name_count += 1;
        },
      )
    };

    let cached_source = CachedSource::new(source);
    cached_source.stream_chunks().stream(
      &ObjectPool::default(),
      &map_options,
      &mut |_chunk, _mapping| {},
      &mut |_source_index, _source, _source_content| {},
      &mut |_name_index, _name| {},
    );

    let mut cached_on_chunk_count = 0;
    let mut cached_on_source_count = 0;
    let mut cached_on_name_count = 0;
    let cached_generated_info = cached_source.stream_chunks().stream(
      &ObjectPool::default(),
      &map_options,
      &mut |_chunk, _mapping| {
        cached_on_chunk_count += 1;
      },
      &mut |_source_index, _source, _source_content| {
        cached_on_source_count += 1;
      },
      &mut |_name_index, _name| {
        cached_on_name_count += 1;
      },
    );

    assert_eq!(on_chunk_count, cached_on_chunk_count);
    assert_eq!(on_source_count, cached_on_source_count);
    assert_eq!(on_name_count, cached_on_name_count);
    assert_eq!(generated_info, cached_generated_info);
  }

  #[test]
  fn should_have_correct_buffer_if_cache_buffer_from_cache_source() {
    let buf = vec![128u8];
    let source = CachedSource::new(RawBufferSource::from(buf.clone()));

    source.source();
    assert_eq!(source.buffer(), buf.as_slice());
  }

  #[test]
  fn hash_should_different_when_map_are_different() {
    let hash1 = {
      let mut source = ReplaceSource::new(OriginalSource::new("Hello", "hello.txt").boxed());
      source.insert_static(5, " world", None);
      let cache = CachedSource::new(source);
      let mut hasher = FxHasher::default();
      cache.hash(&mut hasher);
      hasher.finish()
    };

    let hash2 = {
      let source = OriginalSource::new("Hello world", "hello.txt").boxed();
      let cache = CachedSource::new(source);
      let mut hasher = FxHasher::default();
      cache.hash(&mut hasher);
      hasher.finish()
    };

    assert!(hash1 != hash2);
  }

  #[test]
  fn size_over_a_raw_buffer_source() {
    // buffer from PNG
    let raw = RawBufferSource::from(vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13]);
    let raw_size = raw.size();
    let cached = CachedSource::new(raw.boxed());
    let cached_size = cached.size();
    assert_eq!(raw_size, cached_size);
  }
}
