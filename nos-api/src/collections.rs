//! Collections module for no-alloc environment

#[cfg(feature = "alloc")]
pub use hashbrown::HashMap;

#[cfg(feature = "alloc")]
pub use alloc::collections::BTreeMap;

#[cfg(not(feature = "alloc"))]
pub struct HashMap<K, V> {
    // Simple placeholder implementation for no-alloc environment
    // In a real implementation, this would use static memory or custom allocator
    _phantom: core::marker::PhantomData<(K, V)>,
}

#[cfg(not(feature = "alloc"))]
impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
    
    pub fn insert(&mut self, _key: K, _value: V) -> Option<V> {
        // Placeholder implementation
        None
    }
    
    pub fn get(&self, _key: &K) -> Option<&V> {
        // Placeholder implementation
        None
    }
    
    pub fn contains_key(&self, _key: &K) -> bool {
        // Placeholder implementation
        false
    }
    
    pub fn remove(&mut self, _key: &K) -> Option<V> {
        // Placeholder implementation
        None
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        // Placeholder implementation
        core::iter::empty()
    }
    
    pub fn values(&self) -> impl Iterator<Item = &V> {
        // Placeholder implementation
        core::iter::empty()
    }
}

#[cfg(not(feature = "alloc"))]
impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
impl<K, V> Clone for HashMap<K, V> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
/// Simple BTreeMap implementation for no-alloc environment
#[derive(Debug)]
pub struct BTreeMap<K, V> {
    // Simple placeholder implementation for no-alloc environment
    _phantom: core::marker::PhantomData<(K, V)>,
}

#[cfg(not(feature = "alloc"))]
impl<K, V> BTreeMap<K, V> {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
    
    pub fn insert(&mut self, _key: K, _value: V) -> Option<V> {
        // Placeholder implementation
        None
    }
    
    pub fn get(&self, _key: &K) -> Option<&V> {
        // Placeholder implementation
        None
    }
    
    pub fn get_mut(&mut self, _key: &K) -> Option<&mut V> {
        // Placeholder implementation
        None
    }
    
    pub fn remove(&mut self, _key: &K) -> Option<V> {
        // Placeholder implementation
        None
    }
    
    pub fn values(&self) -> impl Iterator<Item = &V> {
        // Placeholder implementation
        core::iter::empty()
    }
    
    pub fn len(&self) -> usize {
        // Placeholder implementation
        0
    }
    
    pub fn contains_key(&self, _key: &K) -> bool {
        // Placeholder implementation
        false
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        // Placeholder implementation
        core::iter::empty()
    }
    
    pub fn cloned(&self) -> impl Iterator<Item = (K, V)> where K: Clone, V: Clone {
        // Placeholder implementation
        core::iter::empty()
    }
}

#[cfg(not(feature = "alloc"))]
impl<K, V> Default for BTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
impl<K, V> Clone for BTreeMap<K, V> {
    fn clone(&self) -> Self {
        Self::new()
    }
}