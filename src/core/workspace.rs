//! Module: src/core/workspace.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::fs::{build_tree, FileEntry, FolderEntry};
use super::path::{ensure_within_root, to_relative};
use super::CoreError;

#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
    tree_cache: Arc<Mutex<TreeCache>>,
}

#[derive(Debug)]
struct TreeCache {
    entries: std::collections::HashMap<(PathBuf, usize), TreeCacheEntry>,
}

#[derive(Clone, Debug)]
struct TreeCacheEntry {
    tree: FolderEntry,
    cached_at: Instant,
}

const TREE_CACHE_TTL: Duration = Duration::from_millis(300);
const TREE_CACHE_MAX_ENTRIES: usize = 64;

impl Workspace {
    // Buka gerbang workspace: canonical path biar kita gak nyasar ke dimensi lain.
    pub fn new(root: PathBuf) -> Result<Self, CoreError> {
        let root = root.canonicalize()?;
        Ok(Self {
            root,
            tree_cache: Arc::new(Mutex::new(TreeCache {
                entries: std::collections::HashMap::new(),
            })),
        })
    }

    // Kasih tahu markas utama kita ada di mana.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // Pindah markas, tapi tetap lewat satpam canonicalize.
    pub fn set_root(&mut self, root: PathBuf) -> Result<(), CoreError> {
        let root = root.canonicalize()?;
        self.root = root;
        self.invalidate_tree_cache();
        Ok(())
    }

    // Minta pohon file dari root dengan batas kedalaman biar tidak jadi hutan belantara.
    pub fn list_tree(&self, depth: usize) -> Result<FolderEntry, CoreError> {
        self.list_tree_at(&self.root, depth)
    }

    // Versi "lihat pohon di titik tertentu", tetap lewat resolve biar aman.
    pub fn list_tree_at(
        &self,
        path: impl AsRef<Path>,
        depth: usize,
    ) -> Result<FolderEntry, CoreError> {
        let target = self.resolve(path)?;
        if let Some(cached) = self.lookup_cached_tree(&target, depth) {
            return Ok(cached);
        }
        let built = build_tree(&target, depth)?;
        self.store_cached_tree(&target, depth, built.clone());
        Ok(built)
    }

    // Baca file UTF-8; kalau isinya alien encoding, kita tolak dengan sopan.
    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<String, CoreError> {
        let target = self.resolve(path)?;
        let bytes = fs::read(&target)?;
        String::from_utf8(bytes).map_err(|_| CoreError::NonUtf8(target))
    }

    // Tulis file dan pastikan folder induk sudah ada, jadi gak "save ke kehampaan".
    pub fn write_file(&self, path: impl AsRef<Path>, content: &str) -> Result<(), CoreError> {
        let target = self.resolve(path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
        self.invalidate_tree_cache();
        Ok(())
    }

    // Bikin file baru plus metadata size, biar langsung siap dipajang di explorer.
    pub fn create_file(
        &self,
        path: impl AsRef<Path>,
        content: &str,
    ) -> Result<FileEntry, CoreError> {
        let target = self.resolve(path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
        let metadata = fs::metadata(&target)?;
        self.invalidate_tree_cache();
        Ok(FileEntry::new(target, metadata.len()))
    }

    // Bikin folder baru (rekursif), karena hidup terlalu singkat untuk mkdir satu-satu.
    pub fn create_folder(&self, path: impl AsRef<Path>) -> Result<FolderEntry, CoreError> {
        let target = self.resolve(path)?;
        fs::create_dir_all(&target)?;
        self.invalidate_tree_cache();
        Ok(FolderEntry::new(target, Vec::new()))
    }

    // Hapus path; kalau folder, sikat sekalian isinya.
    pub fn delete_path(&self, path: impl AsRef<Path>) -> Result<(), CoreError> {
        let target = self.resolve(path)?;
        let metadata = fs::metadata(&target)?;
        if metadata.is_dir() {
            fs::remove_dir_all(&target)?;
        } else {
            fs::remove_file(&target)?;
        }
        self.invalidate_tree_cache();
        Ok(())
    }

    // Rename/move path dengan jaminan parent target ada duluan.
    pub fn rename_path(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<(), CoreError> {
        let source = self.resolve(from)?;
        let target = self.resolve(to)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(source, target)?;
        self.invalidate_tree_cache();
        Ok(())
    }

    // Resolver resmi: relative -> root, absolute -> diverifikasi tetap di wilayah kekuasaan.
    pub fn resolve(&self, path: impl AsRef<Path>) -> Result<PathBuf, CoreError> {
        let path = path.as_ref();
        let target = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        ensure_within_root(&self.root, &target)
    }

    // Ubah absolute path jadi path relatif ke root supaya UI gak kepanjangan curhat.
    pub fn relative(&self, path: impl AsRef<Path>) -> PathBuf {
        to_relative(&self.root, path.as_ref())
    }

    fn lookup_cached_tree(&self, path: &Path, depth: usize) -> Option<FolderEntry> {
        let mut cache = self.tree_cache.lock().ok()?;
        let key = (path.to_path_buf(), depth);
        let Some(entry) = cache.entries.get(&key).cloned() else {
            return None;
        };
        if entry.cached_at.elapsed() > TREE_CACHE_TTL {
            cache.entries.remove(&key);
            return None;
        }
        Some(entry.tree)
    }

    fn store_cached_tree(&self, path: &Path, depth: usize, tree: FolderEntry) {
        let Ok(mut cache) = self.tree_cache.lock() else {
            return;
        };
        if cache.entries.len() >= TREE_CACHE_MAX_ENTRIES {
            cache.entries.clear();
        }
        cache.entries.insert(
            (path.to_path_buf(), depth),
            TreeCacheEntry {
                tree,
                cached_at: Instant::now(),
            },
        );
    }

    fn invalidate_tree_cache(&self) {
        if let Ok(mut cache) = self.tree_cache.lock() {
            cache.entries.clear();
        }
    }
}
