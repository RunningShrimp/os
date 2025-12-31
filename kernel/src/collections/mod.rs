// Collections module for kernel
// Provides HashMap and VecDeque implementations

extern crate alloc;
extern crate hashbrown;

// Use alloc's default hasher for collections
pub type HashMap<K, V, H = hashbrown::DefaultHashBuilder> = hashbrown::HashMap<K, V, H>;
pub type HashSet<K, H = hashbrown::DefaultHashBuilder> = hashbrown::HashSet<K, H>;
pub use alloc::collections::VecDeque;
