use std::process::Command;
use std::path::Path;
use nix::unistd::{getuid, getgid};

use crate::core::error::{DotmanError, Result};

/// Privilege management utility
pub struct PrivilegeUtility;

impl PrivilegeUtility {
    /// Check if currently running as root
    pub fn is_root() -> bool {
        getuid().is_root()
    }

    /// Get current user ID
    pub fn current_uid() -> u32 {
        getuid().as_raw()
    }

    /// Get current group ID
    pub fn current_gid() -> u32 {
        getgid().as_raw()
    }

    /// Check if sudo is available
    pub fn has_sudo() -> bool {
        Command::new("which")
            .arg("sudo")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if we can run commands with sudo without password
    pub fn can_sudo_nopasswd() -> bool {
        Command::new("sudo")
            .args(["-n", "true"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Execute command with sudo
    pub async fn sudo_command(command: &str, args: &[&str]) -> Result<std::process::Output> {
        if !Self::has_sudo() {
            return Err(DotmanError::privilege("sudo not available".to_string()));
        }

        let mut cmd = tokio::process::Command::new("sudo");
        cmd.arg(command);
        cmd.args(args);
        
        let output = cmd.output().await
            .map_err(|e| DotmanError::privilege(format!("Failed to execute sudo command: {}", e)))?;
        
        Ok(output)
    }

    /// Copy file with sudo if needed
    pub async fn sudo_copy(src: &Path, dst: &Path) -> Result<()> {
        let output = Self::sudo_command("cp", &[
            src.to_str().ok_or_else(|| DotmanError::path("Invalid source path".to_string()))?,
            dst.to_str().ok_or_else(|| DotmanError::path("Invalid destination path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo cp failed: {}", stderr)));
        }

        Ok(())
    }

    /// Create directory with sudo if needed
    pub async fn sudo_mkdir(path: &Path) -> Result<()> {
        let output = Self::sudo_command("mkdir", &[
            "-p",
            path.to_str().ok_or_else(|| DotmanError::path("Invalid path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo mkdir failed: {}", stderr)));
        }

        Ok(())
    }

    /// Remove file/directory with sudo if needed
    pub async fn sudo_remove(path: &Path) -> Result<()> {
        let output = Self::sudo_command("rm", &[
            "-rf",
            path.to_str().ok_or_else(|| DotmanError::path("Invalid path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo rm failed: {}", stderr)));
        }

        Ok(())
    }

    /// Change ownership with sudo
    pub async fn sudo_chown(path: &Path, uid: u32, gid: u32) -> Result<()> {
        let ownership = format!("{}:{}", uid, gid);
        let output = Self::sudo_command("chown", &[
            &ownership,
            path.to_str().ok_or_else(|| DotmanError::path("Invalid path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo chown failed: {}", stderr)));
        }

        Ok(())
    }

    /// Change permissions with sudo
    pub async fn sudo_chmod(path: &Path, mode: u32) -> Result<()> {
        let mode_str = format!("{:o}", mode);
        let output = Self::sudo_command("chmod", &[
            &mode_str,
            path.to_str().ok_or_else(|| DotmanError::path("Invalid path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo chmod failed: {}", stderr)));
        }

        Ok(())
    }

    /// Create symlink with sudo
    pub async fn sudo_symlink(target: &Path, link: &Path) -> Result<()> {
        let output = Self::sudo_command("ln", &[
            "-sf",
            target.to_str().ok_or_else(|| DotmanError::path("Invalid target path".to_string()))?,
            link.to_str().ok_or_else(|| DotmanError::path("Invalid link path".to_string()))?,
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DotmanError::privilege(format!("sudo ln failed: {}", stderr)));
        }

        Ok(())
    }

    /// Request password prompt for sudo operations
    pub async fn request_sudo_password() -> Result<()> {
        println!("Elevated privileges required for system file operations.");
        println!("You may be prompted for your password.");
        
        let output = Self::sudo_command("true", &[]).await?;
        
        if !output.status.success() {
            return Err(DotmanError::privilege("Failed to obtain sudo privileges".to_string()));
        }
        
        Ok(())
    }

    /// Check if path requires elevated privileges to modify
    pub fn path_requires_sudo(path: &Path) -> bool {
        crate::utils::path::PathUtility::requires_privileges(path)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_detection() {
        // These tests may vary based on the system configuration
        let is_root = PrivilegeUtility::is_root();
        let uid = PrivilegeUtility::current_uid();
        let gid = PrivilegeUtility::current_gid();
        
        // Basic sanity checks
        if is_root {
            assert_eq!(uid, 0);
        }
        
        // Check that we can at least detect the current IDs
        assert!(uid >= 0);
        assert!(gid >= 0);
    }

    #[test]
    fn test_sudo_availability() {
        // Test if sudo is available (may vary by system)
        let has_sudo = PrivilegeUtility::has_sudo();
        println!("Sudo available: {}", has_sudo);
        
        if has_sudo {
            let can_nopasswd = PrivilegeUtility::can_sudo_nopasswd();
            println!("Can sudo without password: {}", can_nopasswd);
        }
    }

    #[test]
    fn test_path_privilege_requirements() {
        assert!(PrivilegeUtility::path_requires_sudo(Path::new("/etc/passwd")));
        assert!(PrivilegeUtility::path_requires_sudo(Path::new("/usr/bin/test")));
        assert!(!PrivilegeUtility::path_requires_sudo(Path::new("/home/user/file.txt")));
        assert!(!PrivilegeUtility::path_requires_sudo(Path::new("./local/file.txt")));
    }
} 