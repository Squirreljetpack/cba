//! A map backed by a `Vec` of key-value pairs.
//!
//! Lookup is linear, so this suits small maps. Unlike `HashMap` it has a
//! `const` constructor (usable in statics) and a stable iteration order
//! (insertion order).

use std::borrow::Borrow;

/// A map backed by a `Vec` of key-value pairs, preserving insertion order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VecMap<K, V> {
    vec: Vec<(K, V)>,
}

impl<K, V> VecMap<K, V> {
    /// An empty map.
    pub const fn new() -> Self {
        Self { vec: Vec::new() }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// Iterate over the entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.vec.iter().map(|(k, v)| (k, v))
    }

    /// Iterate over the entries in insertion order, mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.vec.iter_mut().map(|(k, v)| (&*k, &mut *v))
    }

    /// Iterate over the keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.vec.iter().map(|(k, _)| k)
    }

    /// Iterate over the values in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.vec.iter().map(|(_, v)| v)
    }
}

impl<K: Eq, V> VecMap<K, V> {
    /// Whether the map contains `key`.
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.get(key).is_some()
    }

    /// The value of `key`, if present.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.vec
            .iter()
            .find(|(k, _)| k.borrow() == key)
            .map(|(_, v)| v)
    }

    /// Mutable reference to the value of `key`, if present.
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.vec
            .iter_mut()
            .find(|(k, _)| k.borrow() == key)
            .map(|(_, v)| v)
    }

    /// Mutable reference to the value of `key`, inserting `default` first
    /// when the key is absent.
    pub fn get_or_insert_mut(&mut self, key: K, default: V) -> &mut V {
        if let Some(i) = self.vec.iter().position(|(k, _)| *k == key) {
            return &mut self.vec[i].1;
        }
        self.vec.push((key, default));
        &mut self.vec.last_mut().expect("just pushed").1
    }

    /// Insert `value` for `key`, returning the previous value when present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some((_, v)) = self.vec.iter_mut().find(|(k, _)| *k == key) {
            return Some(std::mem::replace(v, value));
        }
        self.vec.push((key, value));
        None
    }

    /// Remove the entry of `key`, returning its value when present.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.vec
            .iter()
            .position(|(k, _)| k.borrow() == key)
            .map(|i| self.vec.remove(i).1)
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
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
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
    fn get_or_insert_mut() {
        let mut map = VecMap::new();
        let v = map.get_or_insert_mut("k", Vec::new());
        v.push(1);
        map.get_or_insert_mut("k", Vec::new()).push(2);
        assert_eq!(map.get("k"), Some(&vec![1, 2]));
        assert_eq!(*map.get_mut("k").unwrap(), vec![1, 2]);
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
    }
}

