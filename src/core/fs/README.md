# src/core/fs Folder

## Tujuan
Implementasi model dan traversal filesystem untuk explorer.

## Pencapaian Saat Ini
- Build tree folder/file terurut.
- Skip folder internal (`.git`, `target`) saat listing.
- Struktur node file/folder reusable untuk UI.

## Cara Pemakaian
1. Gunakan `build_tree` untuk membangun struktur explorer.
2. Pakai model `Node`, `FileEntry`, `FolderEntry` di layer atas.
3. Pastikan depth listing sesuai kebutuhan performa.
