use rspack_hash::{
  HashDigest, HashFunction, HashSalt, RspackHash, RspackHashDigest, RspackHashable,
};

#[test]
fn encodes_base64_with_standard_padding() {
  let digest = RspackHashDigest::new(b"\xfb\xef\xff", &HashDigest::Base64);

  assert_eq!(digest.encoded(), "++//");

  let digest = RspackHashDigest::new(b"hello", &HashDigest::Base64);

  assert_eq!(digest.encoded(), "aGVsbG8=");
}

#[test]
fn encodes_base64url_without_padding() {
  let digest = RspackHashDigest::new(b"\xfb\xef\xff", &HashDigest::Base64Url);

  assert_eq!(digest.encoded(), "--__");

  let digest = RspackHashDigest::new(b"hello", &HashDigest::Base64Url);

  assert_eq!(digest.encoded(), "aGVsbG8");
}

#[test]
fn hash_salt_is_written_as_raw_bytes() {
  let salt = HashSalt::Salt("salt".into());
  let salted = RspackHash::with_salt(&HashFunction::Xxhash64, &salt)
    .digest(&HashDigest::Hex)
    .encoded()
    .to_string();

  let mut expected = RspackHash::new(&HashFunction::Xxhash64);
  expected.write(b"salt");
  let expected = expected.digest(&HashDigest::Hex).encoded().to_string();

  assert_eq!(salted, expected);
}

#[test]
fn derive_rspack_hashable_skips_marked_fields() {
  #[derive(RspackHashable)]
  struct Value {
    content: &'static str,
    #[rspack_hash(skip)]
    _cached: &'static str,
  }

  let mut derived = RspackHash::new(&HashFunction::Xxhash64);
  derived.update(&Value {
    content: "content",
    _cached: "cache",
  });
  let derived = derived.digest(&HashDigest::Hex).encoded().to_string();

  let mut expected = RspackHash::new(&HashFunction::Xxhash64);
  expected.write(b"content");
  let expected = expected.digest(&HashDigest::Hex).encoded().to_string();

  assert_eq!(derived, expected);
}

#[test]
fn derive_rspack_hashable_respects_explicit_field_order() {
  #[derive(RspackHashable)]
  struct Value {
    #[rspack_hash(order = 1)]
    second: &'static str,
    #[rspack_hash(order = 0)]
    first: &'static str,
  }

  let mut derived = RspackHash::new(&HashFunction::Xxhash64);
  derived.update(&Value {
    first: "first",
    second: "second",
  });
  let derived = derived.digest(&HashDigest::Hex).encoded().to_string();

  let mut expected = RspackHash::new(&HashFunction::Xxhash64);
  expected.write(b"first");
  expected.write(b"second");
  let expected = expected.digest(&HashDigest::Hex).encoded().to_string();

  assert_eq!(derived, expected);
}

#[test]
fn option_rspack_hashable_skips_none() {
  let mut none = RspackHash::new(&HashFunction::Xxhash64);
  none.update(&Option::<&str>::None);
  let none = none.digest(&HashDigest::Hex).encoded().to_string();

  let empty = RspackHash::new(&HashFunction::Xxhash64)
    .digest(&HashDigest::Hex)
    .encoded()
    .to_string();

  assert_eq!(none, empty);

  let mut some = RspackHash::new(&HashFunction::Xxhash64);
  some.update(&Some("value"));
  let some = some.digest(&HashDigest::Hex).encoded().to_string();

  let mut expected = RspackHash::new(&HashFunction::Xxhash64);
  expected.write(b"value");
  let expected = expected.digest(&HashDigest::Hex).encoded().to_string();

  assert_eq!(some, expected);
}
