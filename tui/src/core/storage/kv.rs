// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Use module is standalone, you can use it in any project that needs an in process or
//! embedded key/value store.
//!
//! You can use it to store keys that are of whatever type you choose, and values that are
//! whatever type you choose.
//!
//! - It is a wrapper around the [`kv`] crate, to make it trivially simple to use. There
//!   are only 4 functions that allow you access to the capabilities of the key/value
//!   embedded store.
//!   - [`try_load_or_create_store`]
//!   - [`try_load_or_create_bucket_from_store`]
//!   - [`try_insert_into_bucket`]
//!   - [`try_get_from_bucket`]
//!   - [`try_remove_from_bucket`]
//!   - [`try_is_key_contained_in_bucket`]
//! - And provide lots of really fine grained errors, using [`miette`] and [`thiserror`]
//!   (see [`kv_error`]).
//! - The values are serialized to JSON (from Rust struct) before they are saved.
//! - The values are deserialized from JSON (to Rust struct) after they are loaded.
//!
//! See the tests in this module for an example of how to use this module.
//!
//! The [`kv`] crate works really well, even with multiple processes accessing the same
//! database on disk. Even though [`sled`], which the [`kv`] crate itself wraps, is not
//! multi-process safe.
//!
//! In my testing, I've run multiple processes that write to the key/value store at the
//! same time, and it works as expected. Even with multiple processes writing to the
//! store, the iterator [`kv::Bucket::iter`] can be used to read the current state of the
//! db, as expected.
//!
//! [`sled`]: https://github.com/spacejam/sled

use crate::fg_cyan;
use kv::{Config, Json, Store};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

/// Convenience type alias for the [`kv::Bucket`] type.
/// 1. A [`kv::Bucket`] is created from a [`Store`].
/// 2. A [`kv::Bucket`] is given a name, and there may be many [`kv::Bucket`]s in a
///    [`Store`].
/// 3. A [`kv::Bucket`] provides typed access to a section of the key/value store [`kv`].
///
/// The [`kv::Bucket`] stores the following key/value pairs.
/// - `KeyT`: The generic type `<KeyT>`. This will not be serialized or deserialized. This
///   also has a trait bound on [`kv::Key`]. See [`try_insert_into_bucket`] for an example of
///   this.
/// - `ValueT`: This type makes it concrete that [`Json`] will be used to serialize and
///   deserialize the data from the generic type `<ValueT>`, which has trait bounds on
///   [`Serialize`], [`Deserialize`]. See [`try_insert_into_bucket`] for an example of this.
pub type KVBucket<'a, KeyT, ValueT> = kv::Bucket<'a, KeyT, Json<ValueT>>;

mod default_settings {
    use super::Debug;

    #[derive(Debug, strum_macros::EnumString, Hash, PartialEq, Eq, Clone, Copy)]
    pub enum Keys {
        /// Your [`kv::Store`] folder path name. [`kv`] uses this folder to save your
        /// key/value store. It is your database persistence folder.
        StoreFolderPath,
        /// Your [`kv::Bucket`] name that is used to store the key/value pairs.
        /// - JSON is used to serialize/deserialize the value stored in the key/value
        ///   pair.
        /// - A [`kv::Bucket`] provides typed access to a section of the key/value store
        ///   [`kv`].
        BucketName,
    }

    pub fn get(key: Keys) -> String {
        match key {
            Keys::StoreFolderPath => "kv_folder".to_string(),
            Keys::BucketName => "my_bucket".to_string(),
        }
    }
}

/// Creates the db folder if it doesn't exist. Otherwise loads it from the folder on disk.
/// Note there are no lifetime annotations on this function. All the other functions below
/// do have lifetime annotations, since they are all tied to the lifetime of the returned
/// [`Store`].
///
/// # Errors
///
/// Returns an error if:
/// - The database folder cannot be created
/// - The store cannot be opened due to I/O errors or permission issues
/// - The database is corrupted or locked by another process
#[tracing::instrument]
pub fn try_load_or_create_store(
    maybe_db_folder_path: Option<&String>,
) -> Result<Store, KvErrorCouldNot> {
    // Configure the database folder location.
    let db_folder_path = maybe_db_folder_path.cloned().unwrap_or_else(|| {
        default_settings::get(default_settings::Keys::StoreFolderPath)
    });

    let cfg = Config::new(db_folder_path.clone());

    // Open the key/store store using the Config.
    let store = Store::new(cfg).map_err(|src_err| KvErrorCouldNot::CreateDbFolder {
        db_folder_path: db_folder_path.clone(),
        src_err,
    })?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "📑 load or create a store",
        db_folder_path = %db_folder_path
    );

    Ok(store)
}

/// A [`kv::Bucket`] provides typed access to a section of the key/value [`Store`].
/// It has a lifetime, since the [`kv::Bucket`] is created from a [`Store`].
///
/// # Errors
///
/// Returns [`KvErrorCouldNot::CreateBucketFromStore`] if the bucket cannot be created or
/// the store is inaccessible.
#[tracing::instrument(fields(store = ?store.path(), buckets = ?store.buckets()))]
pub fn try_load_or_create_bucket_from_store<
    'a,
    KeyT: for<'k> kv::Key<'k>,
    ValueT: Serialize + for<'d> Deserialize<'d>,
>(
    store: &Store,
    maybe_bucket_name: Option<&String>,
) -> Result<KVBucket<'a, KeyT, ValueT>, KvErrorCouldNot> {
    let bucket_name = maybe_bucket_name
        .cloned()
        .unwrap_or_else(|| default_settings::get(default_settings::Keys::BucketName));

    let my_payload_bucket: KVBucket<'_, KeyT, ValueT> =
        store.bucket(Some(&bucket_name)).map_err(|src_err| {
            KvErrorCouldNot::CreateBucketFromStore {
                bucket_name: bucket_name.clone(),
                src_err,
            }
        })?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "📦 Load or create bucket from store, and instantiate",
        bucket_name = %bucket_name
    );

    Ok(my_payload_bucket)
}

/// The value is serialized using JSON prior to saving it to the key/value store.
///
/// # Errors
///
/// Returns [`KvErrorCouldNot::SaveKeyValuePairToBucket`] if serialization or persistence
/// fails.
#[tracing::instrument(skip(bucket))]
pub fn try_insert_into_bucket<
    'a,
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &'a KVBucket<'a, KeyT, ValueT>,
    key: KeyT,
    value: ValueT,
) -> Result<(), KvErrorCouldNot> {
    let value_str = inline_string!("{:?}", value);
    let value_str_fmt = fg_cyan(&value_str).bold();

    // Serialize the Rust struct into a JSON payload.
    bucket
        .set(&key, &Json(value))
        .map_err(KvErrorCouldNot::SaveKeyValuePairToBucket)?;

    // % is Display, ? is Debug.
    tracing::debug!(
        message = "🔽 Save key / value pair to bucket",
        key = %key.to_string(),
        value = %value_str_fmt
    );

    ok!()
}

/// The value in the key/value store is serialized using JSON. Upon loading that value it
/// is deserialized and returned by this function.
///
/// # Errors
///
/// Returns [`KvErrorCouldNot::LoadKeyValuePairFromBucket`] if reading or deserialization
/// fails.
#[tracing::instrument(skip(bucket))]
pub fn try_get_from_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> Result<Option<ValueT>, KvErrorCouldNot> {
    let maybe_value: Option<Json<ValueT>> = bucket
        .get(&key)
        .map_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

    let it = match maybe_value {
        // Deserialize the JSON payload into a Rust struct.
        Some(Json(payload)) => Ok(Some(payload)),
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
/// Returns [`KvErrorCouldNot::RemoveKeyValuePairFromBucket`] if removal fails.
#[tracing::instrument(skip(bucket))]
pub fn try_remove_from_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> Result<Option<ValueT>, KvErrorCouldNot> {
    let maybe_value: Option<Json<ValueT>> = bucket
        .remove(&key)
        .map_err(KvErrorCouldNot::RemoveKeyValuePairFromBucket)?;

    let it = match maybe_value {
        // Deserialize the JSON payload into a Rust struct.
        Some(Json(payload)) => Ok(Some(payload)),
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
/// Returns [`KvErrorCouldNot::LoadKeyValuePairFromBucket`] if the bucket lookup fails.
#[tracing::instrument(skip(bucket))]
pub fn try_is_key_contained_in_bucket<
    KeyT: Debug + Display + for<'k> kv::Key<'k>,
    ValueT: Debug + Serialize + for<'d> Deserialize<'d>,
>(
    bucket: &KVBucket<'_, KeyT, ValueT>,
    key: KeyT,
) -> Result<bool, KvErrorCouldNot> {
    let it = bucket
        .contains(&key)
        .map_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

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
    mut fn_mut: impl FnMut(KeyT, ValueT),
) {
    for item in /* keep only the Ok variants */ bucket.iter().flatten() {
        let Ok(key) = item.key::<KeyT>() else {
            continue;
        };
        let Ok(encoded_value) = item.value::<Json<ValueT>>() else {
            continue;
        };
        let Json(value) = encoded_value; /* decode the value */
        fn_mut(key, value);
    }
}

pub mod kv_error {
    #[derive(thiserror::Error, Debug, miette::Diagnostic)]
    pub enum KvErrorCouldNot {
        #[error("📑 Could not create db folder: '{db_folder_path}' on disk")]
        CreateDbFolder {
            db_folder_path: String,
            #[source]
            src_err: kv::Error,
        },

        #[error("📦 Could not create bucket from store: '{bucket_name}'")]
        CreateBucketFromStore {
            bucket_name: String,
            #[source]
            src_err: kv::Error,
        },

        #[error("🔽 Could not save key/value pair to bucket")]
        SaveKeyValuePairToBucket(#[source] kv::Error),

        #[error("🔼 Could not load key/value pair from bucket")]
        LoadKeyValuePairFromBucket(#[source] kv::Error),

        #[error("❌ Could not remove key/value pair from bucket")]
        RemoveKeyValuePairFromBucket(#[source] kv::Error),
    }
}
use kv_error::KvErrorCouldNot;

#[cfg(test)]
mod kv_tests {
    use super::*;
    use crate::{HashMap, try_create_temp_dir};
    use serial_test::serial;
    use std::path::Path;
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
        let store = try_load_or_create_store(Some(&path_str))?;

        // Check that the key/value store folder exists.
        assert!(check_folder_exists(path_buf.as_path()));

        // A bucket provides typed access to a section of the key/value store.
        let bucket = try_load_or_create_bucket_from_store(&store, Some(&bucket_name))?;

        // Check if "foo" is contained in the bucket.
        assert!(!(try_is_key_contained_in_bucket(&bucket, "foo".to_string())?));

        // Nothing to iterate (empty bucket).
        let mut count = 0;
        for _ in bucket.iter() {
            count += 1;
        }
        assert_eq!(count, 0);

        // Save to bucket.
        try_insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string())?;

        // Check if "foo" is contained in the bucket.
        assert!(try_is_key_contained_in_bucket(&bucket, "foo".to_string())?);

        // Load from bucket.
        assert_eq!(
            try_get_from_bucket(&bucket, "foo".to_string())?,
            Some("bar".to_string())
        );

        // Enumerate contents of bucket.
        let mut map = HashMap::default();
        for result_item in bucket.iter() {
            let item = result_item.map_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;
            let key = item
                .key::<String>()
                .map_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

            // Deserialize the JSON payload into a Rust struct.
            let Json(payload) = item
                .value::<Json<String>>()
                .map_err(KvErrorCouldNot::LoadKeyValuePairFromBucket)?;

            map.insert(key.clone(), payload);
        }

        assert_eq!(map.get("foo"), Some(&"bar".to_string()));

        // Remove from bucket.
        assert_eq!(
            try_remove_from_bucket(&bucket, "foo".to_string())?,
            Some("bar".to_string())
        );

        // Check if "foo" is contained in the bucket.
        assert!(!(try_is_key_contained_in_bucket(&bucket, "foo".to_string())?));

        // Remove from bucket.
        assert_eq!(try_remove_from_bucket(&bucket, "foo".to_string())?, None);

        ok!()
    }

    #[instrument]
    fn perform_db_operations_error_conditions() -> miette::Result<()> {
        let bucket_name = "bucket".to_string();

        // Setup temp dir (this will be dropped when `dir` is out of scope).
        let root_temp_dir = try_create_temp_dir()?;
        let path_buf = root_temp_dir.join("db_folder");

        // Create the key/value store.
        let path_str = path_buf.as_path().to_string_lossy().to_string();
        let store = try_load_or_create_store(Some(&path_str))?;

        // Check that the kv store folder exists.
        assert!(check_folder_exists(path_buf.as_path()));

        // A bucket provides typed access to a section of the key/value store.
        let bucket = try_load_or_create_bucket_from_store(&store, Some(&bucket_name))?;

        // Insert key/value pair into bucket.
        try_insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string())?;

        use miette::IntoDiagnostic;
        use std::error::Error;

        // Check for errors. The following line will induce errors, since we are
        // intentionally trying to access a bucket that doesn't exist.
        store.drop_bucket(bucket_name).into_diagnostic()?;

        // Insert into bucket.
        let result =
            try_insert_into_bucket(&bucket, "foo".to_string(), "bar".to_string());
        match result {
            Err(e) => {
                assert_eq!(e.to_string(), "🔽 Could not save key/value pair to bucket");
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Get from bucket. Take a deeper look in the chain of source errors.
        let result = try_get_from_bucket(&bucket, "foo".to_string());
        match result {
            Err(e) => {
                // First.
                assert_eq!(
                    e.to_string(),
                    "🔼 Could not load key/value pair from bucket"
                );

                // Second (`kv::Error`).
                let second_err = e.source().expect("expected kv::Error source");
                let second = second_err.to_string();
                assert!(second.contains("Error in Sled: Collection"));
                assert!(second.contains("does not exist"));

                // Third (`sled::Error`).
                let third = second_err
                    .source()
                    .expect("expected sled::Error source")
                    .to_string();
                assert!(third.contains("Collection"));
                assert!(third.contains("does not exist"));
            }
            _ => {
                panic!("Expected an error, but got None");
            }
        }

        // Remove from bucket.
        let result = try_remove_from_bucket(&bucket, "foo".to_string());
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
        let result = try_is_key_contained_in_bucket(&bucket, "foo".to_string());
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

        ok!()
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
