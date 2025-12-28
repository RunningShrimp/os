// Collections module for kernel
// Provides HashMap and VecDeque implementations

extern crate alloc;
extern crate hashbrown;

// Use our compatibility DefaultHasherBuilder as the default hasher for
// collections across the kernel so derived impls (e.g. Clone) require the
// hasher to implement Clone / Default consistently.
// Support both `HashMap<K, V>` and `HashMap<K, V, H>` by providing a default
// type parameter for the hasher. This keeps prior callsites (which sometimes
// passed the hasher explicitly) working while allowing shorter forms.
pub type HashMap<K, V, H = crate::compat::DefaultHasherBuilder> = hashbrown::HashMap<K, V, H>;
pub type HashSet<K, H = crate::compat::DefaultHasherBuilder> = hashbrown::HashSet<K, H>;
pub use alloc::collections::VecDeque;

use crate::compat::DefaultHasherBuilder;

/// Create a new HashMap with default hasher
pub fn new_hashmap<K, V>() -> HashMap<K, V> {
    HashMap::with_hasher(DefaultHasherBuilder)
}



