// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Use module is standalone, you can use it in any project that needs an in process or
//! embedded key/value store.
//!
//! You can use it to store keys that are of whatever type you choose, and values that are
//! whatever type you choose.
//!
//! - It is a wrapper around the [kv] crate, to make it trivially simple to use. There are
//!   only 4 functions that allow you access to the capabilities of the key/value embedded
//!   store.
//!   - [`load_or_create_store`]
//!   - [`load_or_create_bucket_from_store`]
//!   - [`insert_into_bucket`]
//!   - [`get_from_bucket`]
//!   - [`remove_from_bucket`]
//!   - [`is_key_contained_in_bucket`]
//! - And provide lots of really fine grained errors, using [miette] and [thiserror] (see
//!   [`kv_error`]).
//!
//! 1. The values are serialized to [Bincode] (from Rust struct) before they are saved.
//! 2. The values are deserialized from [Bincode] (to Rust struct) after they are loaded.
//!
//! See the tests in this module for an example of how to use this module.
//!
//! [Bincode] is like [`CBOR`](https://en.wikipedia.org/wiki/CBOR), except that it isn't
//! standards based, but it is faster. It also has full support of [serde] just like [kv]
//! does.
//! - [More info comparing [`CBOR`](https://en.wikipedia.org/wiki/CBOR) with [`Bincode`](https://gemini.google.com/share/0684553f3d57)
//!
//! The [kv] crate works really well, even with multiple processes accessing the same
//! database on disk. Even though [sled](https://github.com/spacejam/sled), which the [kv]
//! crate itself wraps, is not multi-process safe.
//!
//! In my testing, I've run multiple processes that write to the key/value store at the
//! same time, and it works as expected. Even with multiple processes writing to the
//! store, the iterator [`kv::Bucket::iter`] can be used to read the current state of the
//! db, as expected.

use crate::fg_cyan;
use kv::{Bincode, Config, Store};
use miette::{Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

/// Convenience type alias for the [`kv::Bucket`] type.
/// 1. A [`kv::Bucket`] is created from a [Store].
/// 2. A [`kv::Bucket`] is given a name, and there may be many [`kv::Bucket`]s in a
///    [Store].
/// 3. A [`kv::Bucket`] provides typed access to a section of the key/value store [kv].
///
/// The [`kv::Bucket`] stores the following key/value pairs.
/// - `KeyT`: The generic type `<KeyT>`. This will not be serialized or deserialized. This
///   also has a trait bound on [`kv::Key`]. See [`insert_into_bucket`] for an example of
///   this.
/// - `ValueT`: This type makes it concrete that [Bincode] will be used to serialize and
///   deserialize the data from the generic type `<ValueT>`, which has trait bounds on
///   [Serialize], [Deserialize]. See [`insert_into_bucket`] for an example of this.
pub type KVBucket<'a, KeyT, ValueT> = kv::Bucket<'a, KeyT, Bincode<ValueT>>;

mod default_settings {
    use super::Debug;

    #[derive(Debug, strum_macros::EnumString, Hash, PartialEq, Eq, Clone, Copy)]
    pub enum Keys {
        /// Your [Store] folder path name. [kv] uses this folder to save your key/value
        /// store. It is your database persistence folder.
        StoreFolderPath,
        /// Your [Bucket] name that is used to store the key/value pairs.
        /// - [Bincode] is used to serialize/deserialize the value stored in the key/value
        ///   pair.
        /// - A [Bucket] provides typed access to a section of the key/value store [kv].
        BucketName,
    }

    pub fn get(key: Keys) -> String {
        match key {
            Keys::StoreFolderPath => "kv_folder".to_string(),
            Keys::BucketName => "my_bucket".to_string(),
        }
    }
}

/// Create the db folder if it doesn't `request_shutdown`. Otherwise load it from the
/// folder on disk. Note there are no lifetime annotations on this function. All the other
/// functions below do have lifetime annotations, since they are all tied to the lifetime
/// of the returned [Store].
///
/// # Errors
///
/// Returns an error if:
/// - The database folder cannot be created
/// - The store cannot be opened due to I/O errors or permission issues
/// - The database is corrupted or locked by another process
#[tracing::instrument]
pub fn load_or_create_store(
    maybe_db_folder_path: Option<&String>,
) -> miette::Result<Store> {
    // Configure the database folder location.
    let db_folder_path = maybe_db_folder_path.cloned().unwrap_or_else(|| {
        default_settings::get(default_settings::Keys::StoreFolderPath)
    });

    let cfg = Config::new(db_folder_path.clone());

    // Open the key/store store using the Config.
    let store =
        Store::new(cfg)
            .into_diagnostic()
            .wrap_err(KvErrorCouldNot::CreateDbFolder {
                db_folder_path: db_folder_path.clone(),
            })?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "📑 load or create a store",
        db_folder_path = %db_folder_path
    );

    Ok(store)
}

/// A [kv::Bucket] provides typed access to a section of the key/value [Store].
/// It has a lifetime, since the [kv::Bucket] is created from a [Store].
///
/// # Errors
///
/// Returns an error if:
/// - The bucket cannot be created in the store
/// - The store is corrupted or inaccessible
#[tracing::instrument(fields(store = ?store.path(), buckets = ?store.buckets()))]
pub fn load_or_create_bucket_from_store<
    'a,
    KeyT: for<'k> kv::Key<'k>,
    ValueT: Serialize + for<'d> Deserialize<'d>,
>(
    store: &Store,
    maybe_bucket_name: Option<&String>,
) -> miette::Result<KVBucket<'a, KeyT, ValueT>> {
    let bucket_name = maybe_bucket_name
        .cloned()
        .unwrap_or_else(|| default_settings::get(default_settings::Keys::BucketName));

    let my_payload_bucket: KVBucket<'_, KeyT, ValueT> = store
        .bucket(Some(&bucket_name))
        .into_diagnostic()
        .wrap_err(KvErrorCouldNot::CreateBucketFromStore {
            bucket_name: bucket_name.clone(),
        })?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "📦 Load or create bucket from store, and instantiate",
        bucket_name = %bucket_name
    );

    Ok(my_payload_bucket)
}

/// The value is serialized using [Bincode] prior to saving it to the key/value store.
///
/// # Errors
///
/// Returns an error if:
/// - The value cannot be serialized to Bincode format
/// - The key/value pair cannot be saved to the bucket due to I/O errors
/// - The bucket is corrupted or the store is locked
#[tracing::instrument(skip(bucket))]
pub fn insert_into_bucket<
    'a,
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &'a KVBucket<'a, KeyT, ValueT>,
    key: KeyT,
    value: ValueT,
) -> miette::Result<()> {
    let value_str = inline_string!("{:?}", value);
    let value_str_fmt = fg_cyan(&value_str).bold();

    // Serialize the Rust struct into a binary payload.
    bucket
        .set(&key, &Bincode(value))
        .into_diagnostic()
        .wrap_err(KvErrorCouldNot::SaveKeyValuePairToBucket)?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "🔽 Save key / value pair to bucket",
        key = %key.to_string(),
        value = %value_str_fmt
    );

    Ok(())
}

/// The value in the key/value store is serialized using [Bincode]. Upon loading that
/// value it is deserialized and returned by this function.
///
/// # Errors
///
/// Returns an error if:
/// - The key cannot be found and an I/O error occurs
/// - The value cannot be deserialized from Bincode format
/// - The bucket is corrupted or inaccessible
#[tracing::instrument(skip(bucket))]
pub fn get_from_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> miette::Result<Option<ValueT>> {
    let maybe_value: Option<Bincode<ValueT>> = bucket
        .get(&key)
        .into_diagnostic()
        .wrap_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

    let it = match maybe_value {
        // Deserialize the binary payload into a Rust struct.
        Some(Bincode(payload)) => Ok(Some(payload)),
        _ => Ok(None),
    };

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "🔼 Load key / value pair from bucket",
        key = %key.to_string(),
        value = ?it
    );

    it
}

/// # Errors
///
/// Returns an error if:
/// - The removal operation fails due to I/O errors
/// - The bucket is corrupted or the store is locked
#[tracing::instrument(skip(bucket))]
pub fn remove_from_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> miette::Result<Option<ValueT>> {
    let maybe_value: Option<Bincode<ValueT>> = bucket
        .remove(&key)
        .into_diagnostic()
        .wrap_err(KvErrorCouldNot::RemoveKeyValuePairFromBucket)?;

    let it = match maybe_value {
        // Deserialize the binary payload into a Rust struct.
        Some(Bincode(payload)) => Ok(Some(payload)),
        _ => Ok(None),
    };

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "❌ Delete key / value pair from bucket",
        key = %key.to_string(),
        value = ?it
    );

    it
}

/// # Errors
///
/// Returns an error if:
/// - The contains check fails due to I/O errors
/// - The bucket is corrupted or inaccessible
#[tracing::instrument(skip(bucket))]
pub fn is_key_contained_in_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> miette::Result<bool> {
    let it = bucket
        .contains(&key)
        .into_diagnostic()
        .wrap_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "🔼 Check if key is contained in bucket",
        key = %key.to_string(),
        value = %if it { "true" } else { "false" }
    );

    Ok(it)
}

pub fn iterate_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    mut fn_to_apply: impl FnMut(KeyT, ValueT),
) {
    for item in /* keep only the Ok variants */ bucket.iter().flatten() {
        let Ok(key) = item.key::<KeyT>().into_diagnostic() else {
            continue;
        };
        let Ok(encoded_value) = item.value::<Bincode<ValueT>>().into_diagnostic() else {
            continue;
        };
        let Bincode(value) = encoded_value; /* decode the value */
        fn_to_apply(key, value);
    }
}

pub mod kv_error {
    #[allow(dead_code)]
    #[derive(thiserror::Error, Debug, miette::Diagnostic)]
    pub enum KvErrorCouldNot {
        #[error("📑 Could not create db folder: '{db_folder_path}' on disk")]
        CreateDbFolder { db_folder_path: String },

        #[error("📦 Could not create bucket from store: '{bucket_name}'")]
        CreateBucketFromStore { bucket_name: String },

        #[error("🔽 Could not save key/value pair to bucket")]
        SaveKeyValuePairToBucket,

        #[error("🔼 Could not load key/value pair from bucket")]
        LoadKeyValuePairFromBucket,

        #[error("❌ Could not remove key/value pair from bucket")]
        RemoveKeyValuePairFromBucket,

        #[error("🔍 Could not get item from iterator from bucket")]
        GetItemFromIteratorFromBucket,

        #[error("🔍 Could not get key from item from iterator from bucket")]
        GetKeyFromItemFromIteratorFromBucket,

        #[error("🔍 Could not get value from item from iterator from bucket")]
        GetValueFromItemFromIteratorFromBucket,

        #[error("⚡ Could not execute transaction")]
        ExecuteTransaction,
    }
}
use crate::inline_string;
use kv_error::KvErrorCouldNot;

#[cfg(test)]
mod kv_tests {
    use super::*;
    use crate::try_create_temp_dir;
    use serial_test::serial;
    use std::{collections::HashMap, path::Path};
    use tracing::instrument;

    fn check_folder_exists(path: &Path) -> bool { path.exists() && path.is_dir() }

    #[instrument]
    fn perform_db_operations() -> miette::Result<()> {
        let bucket_name = "bucket".to_string();

        // Setup temp dir (this will be dropped when `dir` is out of scope).
        let root_temp_dir = try_create_temp_dir()?;
        let path_buf = root_temp_dir.join("db_folder");

        // Create the key/value store.
        let path_str = path_buf.as_path().to_string_lossy().to_string();
        let store = load_or_create_store(Some(&path_str))?;

        // Check that the key/value store folder exists.
        assert!(check_folder_exists(path_buf.as_path()));

        // A bucket provides typed access to a section of the key/value store.
        let bucket = load_or_create_bucket_from_store(&store, Some(&bucket_name))?;

        // Check if "foo" is contained in the bucket.
        assert!(!(is_key_contained_in_bucket(&bucket, "foo".to_string())?));

        // Nothing to iterate (empty bucket).
        let mut count = 0;
        for _ in bucket.iter() {
            count += 1;
        }
        assert_eq!(count, 0);

        // Save to bucket.
        insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string())?;

        // Check if "foo" is contained in the bucket.
        assert!(is_key_contained_in_bucket(&bucket, "foo".to_string())?);

        // Load from bucket.
        assert_eq!(
            get_from_bucket(&bucket, "foo".to_string())?,
            Some("bar".to_string())
        );

        // Enumerate contents of bucket.
        let mut map = HashMap::new();
        for result_item in bucket.iter() {
            let item = result_item
                .into_diagnostic()
                .wrap_err(KvErrorCouldNot::GetItemFromIteratorFromBucket)?;

            let key = item
                .key::<String>()
                .into_diagnostic()
                .wrap_err(KvErrorCouldNot::GetKeyFromItemFromIteratorFromBucket)?;

            // Deserialize the binary payload into a Rust struct.
            let Bincode(payload) = item
                .value::<Bincode<String>>()
                .into_diagnostic()
                .wrap_err(KvErrorCouldNot::GetValueFromItemFromIteratorFromBucket)?;

            map.insert(key.clone(), payload);
        }

        assert_eq!(map.get("foo"), Some(&"bar".to_string()));

        // Remove from bucket.
        assert_eq!(
            remove_from_bucket(&bucket, "foo".to_string())?,
            Some("bar".to_string())
        );

        // Check if "foo" is contained in the bucket.
        assert!(!(is_key_contained_in_bucket(&bucket, "foo".to_string())?));

        // Remove from bucket.
        assert_eq!(remove_from_bucket(&bucket, "foo".to_string())?, None);

        Ok(())
    }

    #[instrument]
    fn perform_db_operations_error_conditions() -> miette::Result<()> {
        let bucket_name = "bucket".to_string();

        // Setup temp dir (this will be dropped when `dir` is out of scope).
        let root_temp_dir = try_create_temp_dir()?;
        let path_buf = root_temp_dir.join("db_folder");

        // Create the key/value store.
        let path_str = path_buf.as_path().to_string_lossy().to_string();
        let store = load_or_create_store(Some(&path_str))?;

        // Check that the kv store folder exists.
        assert!(check_folder_exists(path_buf.as_path()));

        // A bucket provides typed access to a section of the key/value store.
        let bucket = load_or_create_bucket_from_store(&store, Some(&bucket_name))?;

        // Insert key/value pair into bucket.
        insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string())?;

        // Check for errors. The following line will induce errors, since we are
        // intentionally trying to access a bucket that doesn't exist.
        store.drop_bucket(bucket_name).into_diagnostic()?;

        // Insert into bucket.
        let result = insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string());
        match result {
            Err(e) => {
                assert_eq!(e.to_string(), "🔽 Could not save key/value pair to bucket");
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Get from bucket. Take a deeper look in the chain of miette errors.
        let result = get_from_bucket(&bucket, "foo".to_string());
        match result {
            Err(e) => {
                let mut iter = e.chain();
                // First.
                assert_eq!(
                    iter.next().map(ToString::to_string).unwrap(),
                    "🔼 Could not load key/value pair from bucket"
                );

                // Second.
                let second = iter.next().map(ToString::to_string).unwrap();
                assert!(second.contains("Error in Sled: Collection"));
                assert!(second.contains("does not exist"));

                // Third.
                let third = iter.next().map(ToString::to_string).unwrap();
                assert!(third.contains("Collection"));
                assert!(third.contains("does not exist"));
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Remove from bucket.
        let result = remove_from_bucket(&bucket, "foo".to_string());
        match result {
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    "❌ Could not remove key/value pair from bucket"
                );
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Check if key exists in bucket.
        let result = is_key_contained_in_bucket(&bucket, "foo".to_string());
        match result {
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    "🔼 Could not load key/value pair from bucket"
                );
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Enumerate contents of bucket.
        let result = bucket.iter().next();
        match result {
            Some(Err(e)) => {
                assert!(e.to_string().contains("Error in Sled"));
                assert!(e.to_string().contains("does not exist"));
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        Ok(())
    }

    /// Run this test in serial, not parallel.
    #[serial]
    #[test]
    fn test_kv_operations() {
        let result = perform_db_operations();
        assert!(result.is_ok());
    }

    /// Run this test in serial, not parallel.
    #[serial]
    #[test]
    fn test_kv_errors() {
        let result = perform_db_operations_error_conditions();
        assert!(result.is_ok());
    }
}
