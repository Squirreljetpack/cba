//! A map backed by separate `Vec`s of keys and values.
//!
//! Lookup is linear over the contiguous keys array, so this suits small maps
//! and benefits from cache locality. Unlike `HashMap` it has a `const` constructor
//! (usable in statics) and a stable iteration order (insertion order).

use std::borrow::Borrow;

/// A map backed by separate `Vec`s of keys and values, preserving insertion order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VecMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K, V> VecMap<K, V> {
    /// An empty map.
    pub const fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }

    /// Iterate over the entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    /// Iterate over the entries in insertion order, mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    /// Iterate over the keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    /// Iterate over the values in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }

    /// Iterate over the values mutably in insertion order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut()
    }
}

impl<K: Eq, V> VecMap<K, V> {
    /// Whether the map contains `key`.
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.keys.iter().any(|k| k.borrow() == key)
    }

    /// The value of `key`, if present.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.keys
            .iter()
            .position(|k| k.borrow() == key)
            .map(|i| &self.values[i])
    }

    /// Mutable reference to the value of `key`, if present.
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.keys
            .iter()
            .position(|k| k.borrow() == key)
            .map(|i| &mut self.values[i])
    }

    /// Reference to the value of `key`, inserting `default` first
    /// when the key is absent.
    pub fn get_or_insert(&mut self, key: K, default: V) -> &V {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            return &self.values[i];
        }
        self.keys.push(key);
        self.values.push(default);
        self.values.last().expect("just pushed")
    }

    /// Mutable reference to the value of `key`, inserting `default` first
    /// when the key is absent.
    pub fn get_or_insert_mut(&mut self, key: K, default: V) -> &mut V {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            return &mut self.values[i];
        }
        self.keys.push(key);
        self.values.push(default);
        self.values.last_mut().expect("just pushed")
    }

    /// Insert `value` for `key`, returning the previous value when present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            return Some(std::mem::replace(&mut self.values[i], value));
        }
        self.keys.push(key);
        self.values.push(value);
        None
    }

    /// Remove the entry of `key`, returning its value when present.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        if let Some(i) = self.keys.iter().position(|k| k.borrow() == key) {
            self.keys.remove(i);
            Some(self.values.remove(i))
        } else {
            None
        }
    }
}

impl<K: Eq, V, Q: ?Sized + Eq> std::ops::Index<&Q> for VecMap<K, V>
where
    K: Borrow<Q>,
{
    type Output = V;

    fn index(&self, index: &Q) -> &V {
        self.get(index).expect("VecMap: key not found")
    }
}

impl<K: Eq, V> FromIterator<(K, V)> for VecMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: Eq, V> Extend<(K, V)> for VecMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.values.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove() {
        let mut map = VecMap::new();
        assert!(map.is_empty());
        assert_eq!(map.insert("a", 1), None);
        assert_eq!(map.insert("b", 2), None);
        assert_eq!(map.insert("a", 3), Some(1));
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&3));
        assert_eq!(map.get("c"), None);
        assert!(map.contains_key("b"));
        assert_eq!(map.remove("b"), Some(2));
        assert_eq!(map.remove("b"), None);
        assert_eq!(map.len(), 1);
        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn get_or_insert_and_get_or_insert_mut() {
        let mut map = VecMap::new();
        let val = map.get_or_insert("a", 10);
        assert_eq!(*val, 10);
        let val = map.get_or_insert("a", 20);
        assert_eq!(*val, 10);

        let mut vec_map = VecMap::new();
        let v = vec_map.get_or_insert_mut("k", Vec::new());
        v.push(1);
        vec_map.get_or_insert_mut("k", Vec::new()).push(2);
        assert_eq!(vec_map.get("k"), Some(&vec![1, 2]));
        assert_eq!(*vec_map.get_mut("k").unwrap(), vec![1, 2]);
    }

    #[test]
    fn iteration_preserves_insertion_order() {
        let map: VecMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into_iter().collect();
        assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(map.values().copied().collect::<Vec<_>>(), vec!["a", "b", "c"]);
        assert_eq!(
            map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            vec![(1, "a"), (2, "b"), (3, "c")]
        );
        let mut iter = map.into_iter();
        assert_eq!(iter.next(), Some((1, "a")));
        assert_eq!(iter.next(), Some((2, "b")));
        assert_eq!(iter.next(), Some((3, "c")));
        assert_eq!(iter.next(), None);
    }
}
