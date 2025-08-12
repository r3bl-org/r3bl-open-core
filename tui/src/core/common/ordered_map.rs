// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::collections::HashMap;

use smallvec::smallvec;

use crate::InlineVec;

#[derive(Debug)]
pub struct OrderedMap<K, V> {
    keys: InlineVec<K>,
    map: HashMap<K, V>,
}

impl<K: std::hash::Hash + Eq + Clone, V> OrderedMap<K, V> {
    #[must_use]
    pub fn new() -> Self {
        OrderedMap {
            keys: smallvec![],
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if !self.map.contains_key(&key) {
            self.keys.push(key.clone());
        }
        self.map.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> { self.map.get(key) }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys
            .iter()
            .filter_map(move |key| self.map.get(key).map(|value| (key, value)))
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Default for OrderedMap<K, V> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests_ordered_map {
    use super::*;

    #[test]
    fn test_ordered_map_insert() {
        let mut map = OrderedMap::new();
        map.insert("key2", "value2");
        map.insert("key1", "value1");
        map.insert("key3", "value3");

        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((&"key2", &"value2")));
        assert_eq!(iter.next(), Some((&"key1", &"value1")));
        assert_eq!(iter.next(), Some((&"key3", &"value3")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_ordered_map_delete() {
        let mut map = OrderedMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");

        // Delete a key and check if it is removed.
        map.map.remove("key2");
        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((&"key1", &"value1")));
        assert_eq!(iter.next(), Some((&"key3", &"value3")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_ordered_map_update() {
        let mut map = OrderedMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");

        // Update a value and check if it is updated.
        map.insert("key2", "new_value2");
        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((&"key1", &"value1")));
        assert_eq!(iter.next(), Some((&"key2", &"new_value2")));
        assert_eq!(iter.next(), Some((&"key3", &"value3")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_ordered_map_get() {
        let mut map = OrderedMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");

        assert_eq!(map.get(&"key1"), Some(&"value1"));
        assert_eq!(map.get(&"key2"), Some(&"value2"));
        assert_eq!(map.get(&"key3"), Some(&"value3"));
        assert_eq!(map.get(&"key4"), None);
    }

    #[test]
    fn test_ordered_map_iter() {
        let mut map = OrderedMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        map.insert("key3", "value3");

        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((&"key1", &"value1")));
        assert_eq!(iter.next(), Some((&"key2", &"value2")));
        assert_eq!(iter.next(), Some((&"key3", &"value3")));
        assert_eq!(iter.next(), None);
    }
}
