//! Bounded standard I/O of the existing immutable native record closure.
use super::*;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, PathBuf},
};

pub(super) fn references(checkpoint: &PreparationCheckpoint) -> Result<Vec<RecordRef>> {
    let mut records = BTreeMap::new();
    for record in [
        &checkpoint.configuration,
        &checkpoint.evidence.request.record,
        &checkpoint.evidence.record,
    ] {
        if records
            .insert(record.path.clone(), record.clone())
            .is_some_and(|old| old != *record)
        {
            return Err(Error::Association(
                "retry record path has conflicting identities",
            ));
        }
    }
    Ok(records.into_values().collect())
}

pub(super) fn checked_source(root: &Path, record: &RecordRef) -> Result<PathBuf> {
    let path = record.resolve(root)?;
    let mut current = root.to_path_buf();
    for component in Path::new(&record.path).components() {
        let Component::Normal(name) = component else {
            return Err(Error::Association("retry source record path"));
        };
        current.push(name);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(Error::Association(
                "retry source record cannot be a symlink",
            ));
        }
    }
    if !path.canonicalize()?.starts_with(root) || !path.is_file() {
        return Err(Error::Association("retry source record escaped its root"));
    }
    Ok(path)
}

pub(super) fn destination(source: &Path, destination: &Path) -> Result<PathBuf> {
    let name = destination
        .file_name()
        .ok_or(Error::Association("retry destination name"))?;
    if !matches!(
        Path::new(name).components().collect::<Vec<_>>().as_slice(),
        [Component::Normal(_)]
    ) {
        return Err(Error::Association(
            "retry destination must be a normal directory name",
        ));
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let target = parent.canonicalize()?.join(name);
    if target.starts_with(source) || source.starts_with(&target) {
        return Err(Error::Association(
            "retry destination must be disjoint from original evidence",
        ));
    }
    match fs::symlink_metadata(&target) {
        Ok(_) => return Err(Error::Association("retry destination already exists")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(target)
}

pub(super) fn import(
    source: &Path,
    target: &Path,
    records: &[RecordRef],
    maximum: u64,
    mut check: impl FnMut() -> Result<()>,
) -> Result<()> {
    let mut total = 0u64;
    for record in records {
        total = total
            .checked_add(record.bytes)
            .ok_or(Error::Association("retry transport overflow"))?;
        if total > maximum {
            return Err(Error::Association("retry transport allowance"));
        }
        checked_source(source, record)?;
        // The caller authenticated the complete closure before native decoding.
        // Authentication of the copied bytes below detects concurrent changes.
        check()?;
    }
    fs::create_dir(target)?;
    for record in records {
        let src = checked_source(source, record)?;
        let dst = record.resolve(target)?;
        fs::create_dir_all(
            dst.parent()
                .ok_or(Error::Association("retry record parent"))?,
        )?;
        let mut input = File::open(src)?;
        let mut output = OpenOptions::new().write(true).create_new(true).open(dst)?;
        let mut copied = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            check()?;
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            copied = copied
                .checked_add(count as u64)
                .ok_or(Error::Association("retry copy overflow"))?;
            if copied > record.bytes {
                return Err(Error::Association("retry source grew during import"));
            }
            output.write_all(&buffer[..count])?;
        }
        output.sync_all()?;
        if copied != record.bytes {
            return Err(Error::Association("retry source shortened during import"));
        }
        record.verify(target)?;
    }
    File::open(target)?.sync_all()?;
    Ok(())
}

pub(super) fn publish_checkpoint(root: &Path, checkpoint: &EndpointRetryCheckpoint) -> Result<()> {
    let temporary = root.join(".endpoint-retry-checkpoint.tmp");
    let destination = root.join("endpoint-retry-checkpoint.json");
    if destination.try_exists()? {
        return Err(Error::Association("retry checkpoint already exists"));
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    serde_json::to_writer(&mut file, checkpoint)
        .map_err(|_| Error::Association("retry checkpoint serialization"))?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(temporary, destination)?;
    File::open(root)?.sync_all()?;
    Ok(())
}
