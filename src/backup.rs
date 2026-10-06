use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn create(vault_path: &Path, backup_dir: &Path) -> Result<PathBuf, BackupError> {
    if !vault_path.is_file() {
        return Err(BackupError::MissingVault(vault_path.to_path_buf()));
    }

    if !backup_dir.exists() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(backup_dir)?;
    }

    let bytes = fs::read(vault_path)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| BackupError::Clock)?
        .as_secs();

    let destination = unique_destination(backup_dir, timestamp)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&destination)?;

    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    fs::set_permissions(&destination, fs::Permissions::from_mode(0o600))?;
    File::open(backup_dir)?.sync_all()?;

    Ok(destination)
}

fn unique_destination(dir: &Path, timestamp: u64) -> Result<PathBuf, BackupError> {
    for sequence in 0_u16..=999 {
        let name = if sequence == 0 {
            format!("otpick-{timestamp}.otpvault")
        } else {
            format!("otpick-{timestamp}-{sequence:03}.otpvault")
        };
        let candidate = dir.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(BackupError::NameExhausted)
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("vault does not exist at {0}")]
    MissingVault(PathBuf),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("could not allocate a unique backup filename")]
    NameExhausted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn snapshot_is_exact_and_private() {
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).unwrap();
        let suffix = u64::from_le_bytes(random);

        let root = std::env::temp_dir().join(format!(
            "otpick-backup-test-{}-{suffix:016x}",
            std::process::id()
        ));
        let source = root.join("vault.otpvault");
        let destination_dir = root.join("backups");

        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"already encrypted vault bytes").unwrap();

        let destination = create(&source, &destination_dir).unwrap();

        assert_eq!(
            fs::read(&destination).unwrap(),
            b"already encrypted vault bytes"
        );
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
            0o600
        );

        fs::remove_dir_all(root).unwrap();
    }
}
