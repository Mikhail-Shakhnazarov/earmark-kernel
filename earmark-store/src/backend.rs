use std::path::Path;

use crate::StoreError;
use gix::bstr::{BStr, ByteSlice};

pub(crate) trait GitBackend {
    fn ensure_repo(&self, root: &Path) -> Result<(), StoreError>;
    fn commit_paths(&self, root: &Path, message: &str) -> Result<(), StoreError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GixBackend;

impl GixBackend {
    fn resolve_signature(
        &self,
        repo: &gix::Repository,
    ) -> Result<gix::actor::Signature, StoreError> {
        let config = repo.config_snapshot();

        let name = std::env::var("EARMARK_GIT_NAME")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                config
                    .string("user.name")
                    .map(|v| v.to_string())
                    .filter(|v| !v.trim().is_empty())
            })
            .unwrap_or_else(|| "earmark".to_string());

        let email = std::env::var("EARMARK_GIT_EMAIL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                config
                    .string("user.email")
                    .map(|v| v.to_string())
                    .filter(|v| !v.trim().is_empty())
            })
            .unwrap_or_else(|| "earmark@local".to_string());

        Ok(gix::actor::Signature {
            name: name.into(),
            email: email.into(),
            time: gix::date::Time::now_local_or_utc(),
        })
    }

    fn has_scope_prefix(path: &BStr, scope: &str) -> bool {
        let scope = scope.as_bytes();
        path.as_bytes().starts_with(scope)
            && (path.len() == scope.len() || path.as_bytes().get(scope.len()) == Some(&b'/'))
    }

    fn mode_from_metadata(meta: &gix::index::fs::Metadata) -> gix::index::entry::Mode {
        if meta.is_executable() {
            gix::index::entry::Mode::FILE_EXECUTABLE
        } else {
            gix::index::entry::Mode::FILE
        }
    }

    fn stage_index_and_write_tree<'a>(
        &self,
        repo: &'a gix::Repository,
        root: &Path,
    ) -> Result<gix::Id<'a>, StoreError> {
        let mut index = repo
            .index_or_load_from_head_or_empty()
            .map_err(|e| StoreError::GitBackend(e.to_string()))?
            .into_owned();

        index.remove_entries(|_, path, _| {
            !Self::has_scope_prefix(path, ".git")
        });

        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let rel = entry
                .path()
                .strip_prefix(root)
                .map_err(|e| StoreError::Invariant(e.to_string()))?;
            
            if rel.starts_with(".git") {
                continue;
            }

            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let path = rel_str.as_bytes().as_bstr();

            let data = std::fs::read(entry.path())?;
            let blob = repo
                .write_blob(data)
                .map_err(|e| StoreError::GitBackend(e.to_string()))?;
            let meta = gix::index::fs::Metadata::from_path_no_follow(entry.path())?;
            let stat = gix::index::entry::Stat::from_fs(&meta)
                .map_err(|e| StoreError::GitBackend(e.to_string()))?;
            let mode = Self::mode_from_metadata(&meta);

            index.dangerously_push_entry(
                stat,
                blob.detach(),
                gix::index::entry::Flags::empty(),
                mode,
                path,
            );
        }

        index.sort_entries();
        index
            .write(Default::default())
            .map_err(|e| StoreError::GitBackend(e.to_string()))?;

        let mut editor = repo
            .edit_tree(gix::hash::ObjectId::empty_tree(repo.object_hash()))
            .map_err(|e| StoreError::GitBackend(e.to_string()))?;

        for (entry, path) in index.entries_mut_with_paths() {
            let kind = match entry.mode {
                gix::index::entry::Mode::FILE | gix::index::entry::Mode::FILE_EXECUTABLE => {
                    gix::object::tree::EntryKind::Blob
                }
                gix::index::entry::Mode::SYMLINK => gix::object::tree::EntryKind::Link,
                gix::index::entry::Mode::COMMIT => gix::object::tree::EntryKind::Commit,
                gix::index::entry::Mode::DIR => continue,
                _ => continue,
            };

            editor
                .upsert(path.to_str_lossy().to_string(), kind, entry.id)
                .map_err(|e| StoreError::GitBackend(e.to_string()))?;
        }

        editor
            .write()
            .map_err(|e| StoreError::GitBackend(e.to_string()))
    }
}

impl GitBackend for GixBackend {
    fn ensure_repo(&self, root: &Path) -> Result<(), StoreError> {
        if !root.join(".git").exists() {
            gix::init(root).map_err(|e| StoreError::GitBackend(e.to_string()))?;
        }
        Ok(())
    }

    fn commit_paths(&self, root: &Path, message: &str) -> Result<(), StoreError> {
        let repo = gix::open_opts(root, gix::open::Options::isolated())
            .map_err(|e| StoreError::GitBackend(e.to_string()))?;
        
        // Ensure we haven't discovered the workspace root through upward traversal
        let git_dir = repo.git_dir().canonicalize().map_err(|e| StoreError::GitBackend(e.to_string()))?;
        let expected_git_dir = root.join(".git").canonicalize().map_err(|e| StoreError::GitBackend(e.to_string()))?;
        if git_dir != expected_git_dir {
            return Err(StoreError::GitBackend("discovered wrong repository through upward traversal".to_string()));
        }

        // Snapshot current index to allow rollback on failure
        let index_path = repo.index_path();
        let index_snapshot = if index_path.exists() {
            Some(std::fs::read(&index_path)?)
        } else {
            None
        };

        let commit_result: Result<(), StoreError> = (|| {
            let signature = self.resolve_signature(&repo)?;
            let tree_id = self.stage_index_and_write_tree(&repo, root)?;
            let parent_ids: Vec<_> = repo
                .head_id()
                .ok()
                .map(|id| vec![id.detach()])
                .unwrap_or_default();

            let mut time_buf = gix::date::parse::TimeBuf::default();
            let sig_ref = signature.to_ref(&mut time_buf);

            repo.commit_as(
                sig_ref,
                sig_ref,
                "HEAD",
                message,
                tree_id.detach(),
                parent_ids,
            )
            .map_err(|e| StoreError::GitBackend(e.to_string()))?;

            Ok(())
        })();

        if let Err(err) = commit_result {
            // Restore the index to its previous state
            if let Some(bytes) = index_snapshot {
                std::fs::write(&index_path, bytes)?;
            } else if index_path.exists() {
                let _ = std::fs::remove_file(&index_path);
            }
            return Err(err);
        }

        Ok(())
    }
}
