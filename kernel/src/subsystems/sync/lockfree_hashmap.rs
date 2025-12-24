//! Lock-free Hashmap
//!
//! A concurrent, lock-free hashmap implementation using open addressing and atomic operations.
//! Based on FASTER (Fast Atomic Shippable Tries) concept.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::hash_map::DefaultHasher;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

const LOAD_FACTOR: usize = 4;
const MAX_PROBE: usize = 64;

/// Hash table entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum EntryState {
    Empty = 0,
    Inserting = 1,
    Occupied = 2,
    Tombstone = 3,
}

impl EntryState {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => EntryState::Empty,
            1 => EntryState::Inserting,
            2 => EntryState::Occupied,
            3 => EntryState::Tombstone,
            _ => EntryState::Empty,
        }
    }
}

/// Hash table entry
#[repr(C)]
struct Entry<K, V> {
    state: AtomicU8,
    key: MaybeUninit<K>,
    value: MaybeUninit<V>,
    next: AtomicPtr<Entry<K, V>>,
}

impl<K, V> Entry<K, V> {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EntryState::Empty as u8),
            key: MaybeUninit::uninit(),
            value: MaybeUninit::uninit(),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

struct AtomicU8 {
    value: core::sync::atomic::AtomicU8,
}

impl AtomicU8 {
    const fn new(value: u8) -> Self {
        Self {
            value: core::sync::atomic::AtomicU8::new(value),
        }
    }

    fn load(&self, ordering: Ordering) -> u8 {
        self.value.load(ordering)
    }

    fn compare_exchange_weak(
        &self,
        current: u8,
        new: u8,
        success: Ordering,
        failure: Ordering,
    ) -> Result<u8, u8> {
        self.value.compare_exchange_weak(current, new, success, failure)
    }

    fn store(&self, value: u8, ordering: Ordering) {
        self.value.store(value, ordering);
    }
}

/// Lock-free hash table
pub struct LockFreeHashMap<K, V> {
    table: Vec<AtomicPtr<Entry<K, V>>>,
    capacity: AtomicU64,
    size: AtomicU64,
    _marker: PhantomData<(K, V)>,
}

unsafe impl<K: Send, V: Send> Send for LockFreeHashMap<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for LockFreeHashMap<K, V> {}

impl<K, V> LockFreeHashMap<K, V> {
    /// Create a new lock-free hash map with default capacity (16)
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    /// Create a new lock-free hash map with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let actual_capacity = capacity.next_power_of_two();
        let table: Vec<AtomicPtr<Entry<K, V>>> = (0..actual_capacity)
            .map(|_| AtomicPtr::new(core::ptr::null_mut()))
            .collect();

        Self {
            table,
            capacity: AtomicU64::new(actual_capacity as u64),
            size: AtomicU64::new(0),
            _marker: PhantomData,
        }
    }

    /// Get the current capacity
    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Acquire) as usize
    }

    /// Get the current number of entries
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Acquire) as usize
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value by key
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let capacity = self.capacity();
        let hash = Self::hash(key);
        let mut index = hash as usize % capacity;

        for _ in 0..MAX_PROBE {
            let entry_ptr = self.table[index].load(Ordering::Acquire);

            if entry_ptr.is_null() {
                return None;
            }

            let entry = unsafe { &*entry_ptr };
            let state = EntryState::from_u8(entry.state.load(Ordering::Acquire));

            match state {
                EntryState::Empty => return None,
                EntryState::Occupied => {
                    let entry_key = unsafe { entry.key.assume_init_ref() };
                    if entry_key.borrow() == key {
                        return Some(unsafe { entry.value.assume_init_read() });
                    }
                }
                EntryState::Tombstone => {}
                EntryState::Inserting => {}
            }

            index = (index + 1) % capacity;
        }

        None
    }

    /// Insert or update a key-value pair
    pub fn insert(&self, key: K, value: V) -> Result<(), V> {
        loop {
            let capacity = self.capacity();
            let hash = Self::hash(&key);
            let mut index = hash as usize % capacity;

            let mut tombstone_index: Option<usize> = None;

            for _ in 0..MAX_PROBE {
                let entry_ptr = self.table[index].load(Ordering::Acquire);

                if entry_ptr.is_null() {
                    let entry = Box::leak(Box::new(Entry::new()));
                    entry.state.store(EntryState::Inserting as u8, Ordering::Release);
                    unsafe {
                        entry.key.write(key);
                        entry.value.write(value);
                    }
                    entry.state.store(EntryState::Occupied as u8, Ordering::Release);

                    if self.table[index].compare_exchange_weak(
                        core::ptr::null_mut(),
                        entry,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ).is_ok() {
                        self.size.fetch_add(1, Ordering::Relaxed);
                        return Ok(());
                    } else {
                        unsafe {
                            let _ = unsafe { entry.key.assume_init_read() };
                            let _ = unsafe { entry.value.assume_init_read() };
                            drop(Box::from_raw(entry));
                        }
                        continue;
                    }
                }

                let entry = unsafe { &*entry_ptr };
                let state = EntryState::from_u8(entry.state.load(Ordering::Acquire));

                match state {
                    EntryState::Empty => {
                        if let Some(tombstone_idx) = tombstone_index {
                            index = tombstone_idx;
                        }

                        let entry = Box::leak(Box::new(Entry::new()));
                        entry.state.store(EntryState::Inserting as u8, Ordering::Release);
                        unsafe {
                            entry.key.write(key);
                            entry.value.write(value);
                        }
                        entry.state.store(EntryState::Occupied as u8, Ordering::Release);

                        if self.table[index].compare_exchange_weak(
                            entry_ptr,
                            entry,
                            Ordering::AcqRel,
                            Ordering::Relaxed,
                        ).is_ok() {
                            self.size.fetch_add(1, Ordering::Relaxed);
                            return Ok(());
                        } else {
                            unsafe {
                                let _ = unsafe { entry.key.assume_init_read() };
                                let _ = unsafe { entry.value.assume_init_read() };
                                drop(Box::from_raw(entry));
                            }
                            continue;
                        }
                    }
                    EntryState::Occupied => {
                        let entry_key = unsafe { entry.key.assume_init_ref() };
                        if entry_key == &key {
                            let old_value = unsafe { entry.value.assume_init_read() };
                            unsafe {
                                entry.value.write(value);
                            }
                            return Err(old_value);
                        }
                    }
                    EntryState::Tombstone => {
                        if tombstone_index.is_none() {
                            tombstone_index = Some(index);
                        }
                    }
                    EntryState::Inserting => {}
                }

                index = (index + 1) % capacity;
            }

            self.rehash();
        }
    }

    /// Remove a key-value pair
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let capacity = self.capacity();
        let hash = Self::hash(key);
        let mut index = hash as usize % capacity;

        for _ in 0..MAX_PROBE {
            let entry_ptr = self.table[index].load(Ordering::Acquire);

            if entry_ptr.is_null() {
                return None;
            }

            let entry = unsafe { &*entry_ptr };
            let state = EntryState::from_u8(entry.state.load(Ordering::Acquire));

            match state {
                EntryState::Empty => return None,
                EntryState::Occupied => {
                    let entry_key = unsafe { entry.key.assume_init_ref() };
                    if entry_key.borrow() == key {
                        entry.state.store(EntryState::Tombstone as u8, Ordering::Release);
                        self.size.fetch_sub(1, Ordering::Relaxed);
                        return Some(unsafe { entry.value.assume_init_read() });
                    }
                }
                EntryState::Tombstone => {}
                EntryState::Inserting => {}
            }

            index = (index + 1) % capacity;
        }

        None
    }

    /// Check if the map contains a key
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Clear all entries
    pub fn clear(&self) {
        let capacity = self.capacity();

        for i in 0..capacity {
            let entry_ptr = self.table[i].load(Ordering::Acquire);

            if !entry_ptr.is_null() {
                let entry = unsafe { Box::from_raw(entry_ptr) };
                drop(entry);
                self.table[i].store(core::ptr::null_mut(), Ordering::Release);
            }
        }

        self.size.store(0, Ordering::Release);
    }

    /// Compute hash for a key
    fn hash<Q>(key: &Q) -> u64
    where
        Q: Hash + ?Sized,
    {
        use core::hash::Hasher;
        use alloc::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Rehash the table (expand capacity)
    fn rehash(&self) {
        let old_capacity = self.capacity();
        let new_capacity = old_capacity * 2;
        let old_table: Vec<AtomicPtr<Entry<K, V>>> = self.table.clone();

        let new_table: Vec<AtomicPtr<Entry<K, V>>> = (0..new_capacity)
            .map(|_| AtomicPtr::new(core::ptr::null_mut()))
            .collect();

        for i in 0..old_capacity {
            let entry_ptr = old_table[i].load(Ordering::Acquire);

            if !entry_ptr.is_null() {
                let entry = unsafe { &*entry_ptr };
                let state = EntryState::from_u8(entry.state.load(Ordering::Acquire));

                if state == EntryState::Occupied {
                    let key = unsafe { entry.key.assume_init_read() };
                    let hash = Self::hash(&key);
                    let mut new_index = hash as usize % new_capacity;

                    loop {
                        let entry_ptr = new_table[new_index].load(Ordering::Acquire);

                        if entry_ptr.is_null() {
                            new_table[new_index].store(entry_ptr, Ordering::Release);
                            break;
                        }

                        new_index = (new_index + 1) % new_capacity;
                    }
                }
            }
        }

        self.capacity.store(new_capacity as u64, Ordering::Release);
    }
}

impl<K, V> Drop for LockFreeHashMap<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Borrow trait for hash lookups
pub trait Borrow<Borrowed: ?Sized> {
    fn borrow(&self) -> &Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    fn borrow(&self) -> &T {
        self
    }
}

impl<'a, T: ?Sized> Borrow<T> for &'a T {
    fn borrow(&self) -> &T {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert_get() {
        let map = LockFreeHashMap::new();
        map.insert(1, 10).ok();
        assert_eq!(map.get(&1), Some(10));
    }

    #[test]
    fn test_update() {
        let map = LockFreeHashMap::new();
        map.insert(1, 10).ok();
        let old = map.insert(1, 20);
        assert_eq!(old, Err(10));
        assert_eq!(map.get(&1), Some(20));
    }

    #[test]
    fn test_remove() {
        let map = LockFreeHashMap::new();
        map.insert(1, 10).ok();
        assert_eq!(map.remove(&1), Some(10));
        assert_eq!(map.get(&1), None);
    }
}
