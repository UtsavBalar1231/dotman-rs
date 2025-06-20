use std::path::{Path, PathBuf};
use std::collections::HashMap;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error, debug, instrument};
use std::sync::Mutex;

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, OperationResult, OperationType},
    traits::{TransactionManager, FileSystem},
};

/// Transaction state for tracking operations
#[derive(Debug, Clone)]
pub enum TransactionState {
    Active,
    Committed,
    RolledBack,
    Failed,
}

/// Individual operation within a transaction
#[derive(Debug, Clone)]
pub struct TransactionOperation {
    pub id: Uuid,
    pub operation_type: OperationType,
    pub source_path: PathBuf,
    pub target_path: Option<PathBuf>,
    pub backup_path: Option<PathBuf>,
    pub original_metadata: Option<FileMetadata>,
    pub executed: bool,
}

/// Transaction for atomic file operations
#[derive(Debug)]
pub struct Transaction {
    pub id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub operations: Vec<TransactionOperation>,
    pub state: TransactionState,
    pub temp_dir: PathBuf,
}

/// Transaction manager implementation
pub struct DefaultTransactionManager<F>
where
    F: FileSystem + Send + Sync,
{
    filesystem: F,
    active_transactions: Mutex<HashMap<Uuid, Transaction>>,
    temp_root: PathBuf,
}

impl<F> DefaultTransactionManager<F>
where
    F: FileSystem + Send + Sync,
{
    /// Create a new transaction manager
    pub fn new(filesystem: F, temp_root: PathBuf) -> Self {
        Self {
            filesystem,
            active_transactions: Mutex::new(HashMap::new()),
            temp_root,
        }
    }

    /// Create temporary directory for transaction
    async fn create_temp_dir(&self, transaction_id: Uuid) -> Result<PathBuf> {
        let temp_dir = self.temp_root.join(format!("transaction-{}", transaction_id));
        self.filesystem.create_dir_all(&temp_dir).await?;
        Ok(temp_dir)
    }

    /// Create backup of file before modification
    async fn create_backup(&self, file_path: &Path, temp_dir: &Path) -> Result<PathBuf> {
        if !self.filesystem.exists(file_path).await? {
            return Err(DotmanError::file_not_found(file_path.to_path_buf()));
        }

        let backup_name = format!(
            "backup-{}-{}",
            file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            Uuid::new_v4()
        );
        let backup_path = temp_dir.join(backup_name);

        self.filesystem.copy_file(file_path, &backup_path).await?;
        Ok(backup_path)
    }

    /// Execute a single operation within transaction
    async fn execute_operation(
        &self,
        operation: &mut TransactionOperation,
    ) -> Result<()> {
        match operation.operation_type {
            OperationType::Copy => {
                if let Some(ref target_path) = operation.target_path {
                    // Create backup if target exists
                    if self.filesystem.exists(target_path).await? {
                        let temp_dir = self.temp_root.join(format!("transaction-{}", Uuid::new_v4()));
                        operation.backup_path = Some(self.create_backup(target_path, &temp_dir).await?);
                    }

                    // Perform copy
                    self.filesystem.copy_file(&operation.source_path, target_path).await?;
                    operation.executed = true;
                }
            }
            OperationType::Move => {
                if let Some(ref target_path) = operation.target_path {
                    // Create backup if target exists
                    if self.filesystem.exists(target_path).await? {
                        let temp_dir = self.temp_root.join(format!("transaction-{}", Uuid::new_v4()));
                        operation.backup_path = Some(self.create_backup(target_path, &temp_dir).await?);
                    }

                    // Perform move
                    self.filesystem.move_file(&operation.source_path, target_path).await?;
                    operation.executed = true;
                }
            }
            OperationType::Delete => {
                // Create backup before delete
                let temp_dir = self.temp_root.join(format!("transaction-{}", Uuid::new_v4()));
                operation.backup_path = Some(self.create_backup(&operation.source_path, &temp_dir).await?);

                // Perform delete
                self.filesystem.remove_file(&operation.source_path).await?;
                operation.executed = true;
            }
            OperationType::CreateSymlink => {
                if let Some(ref target_path) = operation.target_path {
                    // Create backup if target exists
                    if self.filesystem.exists(target_path).await? {
                        let temp_dir = self.temp_root.join(format!("transaction-{}", Uuid::new_v4()));
                        operation.backup_path = Some(self.create_backup(target_path, &temp_dir).await?);
                    }

                    // Create symlink
                    self.filesystem.create_symlink(&operation.source_path, target_path).await?;
                    operation.executed = true;
                }
            }
            _ => {
                return Err(DotmanError::transaction("Unsupported operation type in transaction".to_string()));
            }
        }

        Ok(())
    }

    /// Rollback a single operation
    async fn rollback_operation(&self, operation: &TransactionOperation) -> Result<()> {
        if !operation.executed {
            return Ok(()); // Nothing to rollback
        }

        match operation.operation_type {
            OperationType::Copy | OperationType::CreateSymlink => {
                // Remove the target file that was created
                if let Some(ref target_path) = operation.target_path {
                    if self.filesystem.exists(target_path).await? {
                        self.filesystem.remove_file(target_path).await?;
                    }

                    // Restore backup if it exists
                    if let Some(ref backup_path) = operation.backup_path {
                        if self.filesystem.exists(backup_path).await? {
                            self.filesystem.copy_file(backup_path, target_path).await?;
                        }
                    }
                }
            }
            OperationType::Move => {
                // Move the file back to original location
                if let Some(ref target_path) = operation.target_path {
                    if self.filesystem.exists(target_path).await? {
                        self.filesystem.move_file(target_path, &operation.source_path).await?;
                    }
                }

                // Restore backup if target existed before
                if let (Some(ref target_path), Some(ref backup_path)) = (&operation.target_path, &operation.backup_path) {
                    if self.filesystem.exists(backup_path).await? {
                        self.filesystem.copy_file(backup_path, target_path).await?;
                    }
                }
            }
            OperationType::Delete => {
                // Restore the deleted file from backup
                if let Some(ref backup_path) = operation.backup_path {
                    if self.filesystem.exists(backup_path).await? {
                        self.filesystem.copy_file(backup_path, &operation.source_path).await?;
                    }
                }
            }
            _ => {
                warn!("Cannot rollback unsupported operation type: {:?}", operation.operation_type);
            }
        }

        Ok(())
    }

    /// Clean up transaction temporary files
    async fn cleanup_transaction(&self, transaction: &Transaction) -> Result<()> {
        if self.filesystem.exists(&transaction.temp_dir).await? {
            // Remove temp directory and all its contents
            self.filesystem.remove(&transaction.temp_dir).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<F> TransactionManager for DefaultTransactionManager<F>
where
    F: FileSystem + Send + Sync,
{
    #[instrument(skip(self))]
    async fn begin_transaction(&self) -> Result<Uuid> {
        let transaction_id = Uuid::new_v4();
        let temp_dir = self.create_temp_dir(transaction_id).await?;

        let transaction = Transaction {
            id: transaction_id,
            started_at: Utc::now(),
            operations: Vec::new(),
            state: TransactionState::Active,
            temp_dir,
        };

        self.active_transactions.lock().unwrap().insert(transaction_id, transaction);

        info!(transaction_id = %transaction_id, "Started new transaction");
        Ok(transaction_id)
    }

    #[instrument(skip(self))]
    async fn add_operation(
        &self,
        transaction_id: Uuid,
        operation: OperationResult,
    ) -> Result<()> {
        // Get original metadata if source exists (before acquiring lock)
        let original_metadata = if self.filesystem.exists(&operation.path).await? {
            Some(self.filesystem.metadata(&operation.path).await?)
        } else {
            None
        };

        let mut transactions = self.active_transactions.lock().unwrap();
        let transaction = transactions.get_mut(&transaction_id)
            .ok_or_else(|| DotmanError::transaction("Transaction not found".to_string()))?;

        if !matches!(transaction.state, TransactionState::Active) {
            return Err(DotmanError::transaction("Transaction is not active".to_string()));
        }

        let transaction_operation = TransactionOperation {
            id: Uuid::new_v4(),
            operation_type: operation.operation_type,
            source_path: operation.path.clone(),
            target_path: None, // This needs to be set based on the operation type
            backup_path: None,
            original_metadata,
            executed: false,
        };

        let operation_id = transaction_operation.id;
        transaction.operations.push(transaction_operation);

        debug!(
            transaction_id = %transaction_id,
            operation_id = %operation_id,
            "Added operation to transaction"
        );

        Ok(())
    }

    #[instrument(skip(self))]
    async fn commit_transaction(&self, transaction_id: Uuid) -> Result<()> {
        let mut transaction = {
            let mut transactions = self.active_transactions.lock().unwrap();
            transactions.remove(&transaction_id)
                .ok_or_else(|| DotmanError::transaction("Transaction not found".to_string()))?
        };

        if !matches!(transaction.state, TransactionState::Active) {
            return Err(DotmanError::transaction("Transaction is not active".to_string()));
        }

        info!(
            transaction_id = %transaction_id,
            operations_count = transaction.operations.len(),
            "Committing transaction"
        );

        let mut results = Vec::new();
        let mut executed_operations = Vec::new();

        // Execute all operations
        for operation in &mut transaction.operations {
            match self.execute_operation(operation).await {
                Ok(_) => {
                    executed_operations.push(operation.clone());
                    
                    results.push(OperationResult {
                        operation_type: operation.operation_type.clone(),
                        path: operation.source_path.clone(),
                        success: true,
                        error: None,
                        details: None,
                        required_privileges: false,
                        duration: None,
                        bytes_processed: None,
                    });

                    debug!(
                        transaction_id = %transaction_id,
                        operation_id = %operation.id,
                        "Operation executed successfully"
                    );
                }
                Err(e) => {
                    error!(
                        transaction_id = %transaction_id,
                        operation_id = %operation.id,
                        error = %e,
                        "Operation failed, rolling back transaction"
                    );

                    // Rollback all executed operations
                    for executed_op in executed_operations.iter().rev() {
                        if let Err(rollback_err) = self.rollback_operation(executed_op).await {
                            error!(
                                transaction_id = %transaction_id,
                                operation_id = %executed_op.id,
                                error = %rollback_err,
                                "Failed to rollback operation"
                            );
                        }
                    }

                    transaction.state = TransactionState::Failed;
                    self.cleanup_transaction(&transaction).await?;

                    return Err(DotmanError::transaction(format!("Transaction failed: {}", e)));
                }
            }
        }

        transaction.state = TransactionState::Committed;
        self.cleanup_transaction(&transaction).await?;

        info!(
            transaction_id = %transaction_id,
            successful_operations = results.len(),
            "Transaction committed successfully"
        );

        Ok(())
    }

    #[instrument(skip(self))]
    async fn rollback_transaction(&self, transaction_id: Uuid) -> Result<()> {
        let mut transaction = {
            let mut transactions = self.active_transactions.lock().unwrap();
            transactions.remove(&transaction_id)
                .ok_or_else(|| DotmanError::transaction("Transaction not found".to_string()))?
        };

        info!(
            transaction_id = %transaction_id,
            operations_count = transaction.operations.len(),
            "Rolling back transaction"
        );

        // Rollback all executed operations in reverse order
        for operation in transaction.operations.iter().rev() {
            if operation.executed {
                if let Err(e) = self.rollback_operation(operation).await {
                    error!(
                        transaction_id = %transaction_id,
                        operation_id = %operation.id,
                        error = %e,
                        "Failed to rollback operation"
                    );
                }
            }
        }

        transaction.state = TransactionState::RolledBack;
        self.cleanup_transaction(&transaction).await?;

        info!(transaction_id = %transaction_id, "Transaction rolled back");
        Ok(())
    }

    async fn get_transaction_status(&self, transaction_id: Uuid) -> Result<crate::core::traits::TransactionStatus> {
        let transactions = self.active_transactions.lock().unwrap();
        let transaction = transactions.get(&transaction_id)
            .ok_or_else(|| DotmanError::transaction("Transaction not found".to_string()))?;
        
        Ok(match transaction.state {
            TransactionState::Active => crate::core::traits::TransactionStatus::Active,
            TransactionState::Committed => crate::core::traits::TransactionStatus::Committed,
            TransactionState::RolledBack => crate::core::traits::TransactionStatus::RolledBack,
            TransactionState::Failed => crate::core::traits::TransactionStatus::Failed,
        })
    }

    async fn list_active_transactions(&self) -> Result<Vec<Uuid>> {
        let transactions = self.active_transactions.lock().unwrap();
        Ok(transactions.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::FileSystemImpl;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transaction_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let temp_root = temp_dir.path().to_path_buf();

        let mut manager = DefaultTransactionManager::new(filesystem, temp_root);
        
        // Test creating a transaction
        let tx_id = manager.begin_transaction().await.unwrap();
        assert!(!tx_id.is_nil());
        
        let active_txs = manager.list_active_transactions().await.unwrap();
        assert_eq!(active_txs.len(), 1);
        assert_eq!(active_txs[0], tx_id);
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let temp_root = temp_dir.path().join("temp");
        tokio::fs::create_dir_all(&temp_root).await.unwrap();

        let mut manager = DefaultTransactionManager::new(filesystem, temp_root);

        // Create test file
        let source_file = temp_dir.path().join("source.txt");
        tokio::fs::write(&source_file, "test content").await.unwrap();

        // Start transaction
        let tx_id = manager.begin_transaction().await.unwrap();
        
        // Add copy operation
        let operation = OperationResult {
            operation_type: OperationType::Copy,
            path: source_file.clone(),
            success: true,
            error: None,
            details: None,
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        };
        manager.add_operation(tx_id, operation).await.unwrap();

        // Commit transaction - this should not fail even if the actual file operations are mock
        manager.commit_transaction(tx_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let temp_root = temp_dir.path().join("temp");
        tokio::fs::create_dir_all(&temp_root).await.unwrap();

        let mut manager = DefaultTransactionManager::new(filesystem, temp_root);

        // Create test file
        let source_file = temp_dir.path().join("source.txt");
        tokio::fs::write(&source_file, "test content").await.unwrap();

        // Start transaction
        let tx_id = manager.begin_transaction().await.unwrap();
        
        // Add delete operation
        let operation = OperationResult {
            operation_type: OperationType::Delete,
            path: source_file.clone(),
            success: true,
            error: None,
            details: None,
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        };
        manager.add_operation(tx_id, operation).await.unwrap();

        // Rollback transaction before commit - should not fail
        manager.rollback_transaction(tx_id).await.unwrap();

        // Original file should still exist (transaction never executed)
        assert!(source_file.exists());
        let content = tokio::fs::read_to_string(&source_file).await.unwrap();
        assert_eq!(content, "test content");
    }
} 
