#[allow(unused_imports)]
use super::*;

pub(super) fn read_generation(ticket_path: &Path) -> Result<u64, StorageError> {
    Ok(read_generation_marker(ticket_path)?.unwrap_or(0))
}

pub(super) fn read_generation_marker(ticket_path: &Path) -> Result<Option<u64>, StorageError> {
    if file_sha256_if_present(ticket_path, GENERATION_FILE)?.is_none() {
        return Ok(None);
    }
    read_generation_file(&ticket_path.join(GENERATION_FILE)).map(Some)
}

pub(super) fn read_generation_file(path: &Path) -> Result<u64, StorageError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(StorageError::Io(error));
        }
        Err(error) => return Err(StorageError::Io(error)),
    };
    contents.trim().parse::<u64>().map_err(|error| {
        transaction_error(
            path.parent().unwrap_or_else(|| Path::new(".")),
            format!(
                "generation marker is malformed at {}: {error}",
                path.display()
            ),
        )
    })
}

pub(super) fn ensure_absent(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    match fs::symlink_metadata(ticket_path.join(relative)) {
        Ok(_) => Err(transaction_error(
            ticket_path,
            format!("transaction-owned path already exists: {relative}"),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StorageError::Io(error)),
    }
}

pub(super) fn create_parent_directories(
    ticket_path: &Path,
    relative: &str,
) -> Result<(), StorageError> {
    let parent = Path::new(relative)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut current = ticket_path.to_path_buf();
    for component in parent.components() {
        let Component::Normal(name) = component else {
            return Err(transaction_error(
                ticket_path,
                format!("unsafe parent in transaction path: {relative}"),
            ));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(transaction_error(
                    ticket_path,
                    format!(
                        "transaction parent is not a real directory: {}",
                        current.display()
                    ),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
                sync_parent(&current)?;
            }
            Err(error) => return Err(StorageError::Io(error)),
        }
    }
    Ok(())
}

pub(super) fn transaction_error(
    ticket_path: &Path,
    detail: impl std::fmt::Display,
) -> StorageError {
    StorageError::Other(format!(
        "ticket transaction at {}: {detail}",
        ticket_path.display()
    ))
}

pub(super) fn sync_parent(path: &Path) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    sync_directory(parent)
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> Result<(), StorageError> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(windows)]
pub(super) fn sync_directory(path: &Path) -> Result<(), StorageError> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, FlushFileBuffers, OPEN_EXISTING,
        },
    };

    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(StorageError::Io(std::io::Error::last_os_error()));
    }
    let flush_result = unsafe { FlushFileBuffers(handle) };
    let flush_error = (flush_result == 0).then(std::io::Error::last_os_error);
    unsafe {
        CloseHandle(handle);
    }
    match flush_error {
        Some(error) => Err(StorageError::Io(error)),
        None => Ok(()),
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn sync_directory(path: &Path) -> Result<(), StorageError> {
    Err(transaction_error(
        path,
        "durable directory synchronization is unsupported on this platform",
    ))
}

#[cfg(unix)]
pub(super) fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
pub(super) fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(StorageError::Io(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(test)]
pub(in super::super) fn set_failpoint(failpoint: Option<FailPoint>) {
    FAIL_AT.with(|current| current.set(failpoint));
}

#[cfg(test)]
pub(super) fn maybe_fail(failpoint: FailPoint) -> Result<(), StorageError> {
    if FAIL_AT.with(|current| current.get() == Some(failpoint)) {
        FAIL_AT.with(|current| current.set(None));
        Err(StorageError::Other(format!(
            "injected failure at transaction boundary {failpoint:?}"
        )))
    } else {
        Ok(())
    }
}

#[cfg(not(test))]
pub(super) fn maybe_fail(_failpoint: FailPoint) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(test)]
pub(super) fn is_injected_failure(error: &StorageError) -> bool {
    error
        .to_string()
        .starts_with("injected failure at transaction boundary")
}

#[cfg(not(test))]
pub(super) fn is_injected_failure(_error: &StorageError) -> bool {
    false
}
