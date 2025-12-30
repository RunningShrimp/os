//! Checkpointing Module
//!
//! Provides system checkpoint and restoration capabilities for fault tolerance

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use nos_api::Result;
use nos_api::Error;

/// Checkpoint type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointType {
    /// Regular checkpoint
    Regular,
    /// Emergency checkpoint
    Emergency,
    /// Recovery checkpoint
    Recovery,
    /// Incremental checkpoint
    Incremental,
}

impl CheckpointType {
    /// Convert checkpoint type to string
    pub fn to_string(&self) -> String {
        match self {
            CheckpointType::Regular => String::from("Regular"),
            CheckpointType::Emergency => String::from("Emergency"),
            CheckpointType::Recovery => String::from("Recovery"),
            CheckpointType::Incremental => String::from("Incremental"),
        }
    }

    /// Parse checkpoint type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Regular" => Some(CheckpointType::Regular),
            "Emergency" => Some(CheckpointType::Emergency),
            "Recovery" => Some(CheckpointType::Recovery),
            "Incremental" => Some(CheckpointType::Incremental),
            _ => None,
        }
    }
}

/// Checkpoint ID type
pub type CheckpointId = u64;

/// Checkpoint metadata
#[derive(Debug, Clone)]
pub struct CheckpointMetadata {
    pub id: CheckpointId,
    pub timestamp: u64,
    pub checkpoint_type: CheckpointType,
    pub description: String,
    pub creator: String,
    pub tags: Vec<String>,
}

impl CheckpointMetadata {
    /// Create new checkpoint metadata
    pub fn new(
        id: CheckpointId,
        checkpoint_type: CheckpointType,
        description: String,
        creator: String,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id,
            timestamp: 0, // Would use actual timestamp in real implementation
            checkpoint_type,
            description,
            creator,
            tags,
        }
    }

    /// Get checkpoint type as string
    pub fn get_type_string(&self) -> String {
        self.checkpoint_type.to_string()
    }
}

/// Checkpoint data
#[derive(Debug, Clone)]
struct Checkpoint {
    id: CheckpointId,
    timestamp: u64,
    checkpoint_type: CheckpointType,
    description: String,
    creator: String,
    tags: Vec<String>,
    data: Vec<u8>,
}

impl Checkpoint {
    /// Create a new checkpoint
    fn new(
        id: CheckpointId,
        checkpoint_type: CheckpointType,
        description: String,
        creator: String,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id,
            timestamp: 0, // Would use actual timestamp in real implementation
            checkpoint_type,
            description,
            creator,
            tags,
            data: Vec::new(),
        }
    }

    /// Convert checkpoint to metadata
    fn to_metadata(&self) -> CheckpointMetadata {
        CheckpointMetadata {
            id: self.id,
            timestamp: self.timestamp,
            checkpoint_type: self.checkpoint_type,
            description: self.description.clone(),
            creator: self.creator.clone(),
            tags: self.tags.clone(),
        }
    }
}

/// Checkpoint manager
pub struct CheckpointManager {
    checkpoints: Vec<Checkpoint>,
    next_id: CheckpointId,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub const fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a checkpoint
    pub fn create_checkpoint(
        &mut self,
        checkpoint_type: CheckpointType,
        description: String,
        creator: String,
        tags: Vec<String>,
    ) -> Result<CheckpointId> {
        let id = self.next_id;
        self.next_id += 1;

        let checkpoint = Checkpoint::new(
            id,
            checkpoint_type,
            description,
            creator,
            tags,
        );

        self.checkpoints.push(checkpoint);
        Ok(id)
    }

    /// Restore from a checkpoint
    pub fn restore_checkpoint(&self, checkpoint_id: CheckpointId) -> Result<()> {
        if checkpoint_id as usize >= self.checkpoints.len() {
            return Err(Error::InvalidArgument(String::from("Checkpoint ID out of range")));
        }
        // In a real implementation, this would restore system state
        Ok(())
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&mut self, checkpoint_id: CheckpointId) -> Result<()> {
        if checkpoint_id as usize >= self.checkpoints.len() {
            return Err(Error::InvalidArgument(String::from("Checkpoint ID out of range")));
        }
        self.checkpoints.remove(checkpoint_id as usize);
        Ok(())
    }

    /// Get all checkpoints metadata
    pub fn get_all_checkpoints(&self) -> Vec<CheckpointMetadata> {
        self.checkpoints
            .iter()
            .map(|cp| cp.to_metadata())
            .collect()
    }

    /// Get checkpoint by ID
    pub fn get_checkpoint(&self, checkpoint_id: CheckpointId) -> Option<CheckpointMetadata> {
        self.checkpoints
            .get(checkpoint_id as usize)
            .map(|cp| cp.to_metadata())
    }

    /// Get checkpoints by type
    pub fn get_checkpoints_by_type(&self, checkpoint_type: CheckpointType) -> Vec<CheckpointMetadata> {
        self.checkpoints
            .iter()
            .filter(|cp| cp.checkpoint_type == checkpoint_type)
            .map(|cp| cp.to_metadata())
            .collect()
    }

    /// Get checkpoint count
    pub fn get_checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Clear all checkpoints
    pub fn clear_all(&mut self) {
        self.checkpoints.clear();
        self.next_id = 0;
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get global checkpoint manager
pub fn get_checkpoint_manager() -> &'static CheckpointManager {
    static MANAGER: CheckpointManager = CheckpointManager::new();
    &MANAGER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_type_to_string() {
        assert_eq!(CheckpointType::Regular.to_string(), "Regular");
        assert_eq!(CheckpointType::Emergency.to_string(), "Emergency");
        assert_eq!(CheckpointType::Recovery.to_string(), "Recovery");
        assert_eq!(CheckpointType::Incremental.to_string(), "Incremental");
    }

    #[test]
    fn test_checkpoint_type_from_str() {
        assert_eq!(CheckpointType::from_str("Regular"), Some(CheckpointType::Regular));
        assert_eq!(CheckpointType::from_str("Emergency"), Some(CheckpointType::Emergency));
        assert_eq!(CheckpointType::from_str("Invalid"), None);
    }

    #[test]
    fn test_checkpoint_manager() {
        let mut manager = CheckpointManager::new();

        // Test creating checkpoint
        let id = manager
            .create_checkpoint(
                CheckpointType::Regular,
                String::from("Test checkpoint"),
                String::from("test"),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(id, 0);

        // Test getting checkpoint
        let checkpoint = manager.get_checkpoint(id);
        assert!(checkpoint.is_some());
        assert_eq!(checkpoint.unwrap().id, id);

        // Test getting all checkpoints
        let all = manager.get_all_checkpoints();
        assert_eq!(all.len(), 1);

        // Test checkpoint count
        assert_eq!(manager.get_checkpoint_count(), 1);
    }

    #[test]
    fn test_restore_invalid_checkpoint() {
        let manager = CheckpointManager::new();
        let result = manager.restore_checkpoint(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_checkpoint() {
        let mut manager = CheckpointManager::new();

        let id = manager
            .create_checkpoint(
                CheckpointType::Regular,
                String::from("Test"),
                String::from("test"),
                Vec::new(),
            )
            .unwrap();

        assert!(manager.delete_checkpoint(id).is_ok());
        assert_eq!(manager.get_checkpoint_count(), 0);
    }

    #[test]
    fn test_get_checkpoints_by_type() {
        let mut manager = CheckpointManager::new();

        manager
            .create_checkpoint(
                CheckpointType::Regular,
                String::from("Test1"),
                String::from("test"),
                Vec::new(),
            )
            .unwrap();

        manager
            .create_checkpoint(
                CheckpointType::Emergency,
                String::from("Test2"),
                String::from("test"),
                Vec::new(),
            )
            .unwrap();

        let regular_checkpoints = manager.get_checkpoints_by_type(CheckpointType::Regular);
        assert_eq!(regular_checkpoints.len(), 1);

        let emergency_checkpoints = manager.get_checkpoints_by_type(CheckpointType::Emergency);
        assert_eq!(emergency_checkpoints.len(), 1);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = CheckpointManager::new();

        manager
            .create_checkpoint(
                CheckpointType::Regular,
                String::from("Test"),
                String::from("test"),
                Vec::new(),
            )
            .unwrap();

        manager.clear_all();
        assert_eq!(manager.get_checkpoint_count(), 0);
    }
}
