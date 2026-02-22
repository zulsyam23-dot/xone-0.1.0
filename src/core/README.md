# src/core Folder

## Tujuan
Menyediakan fondasi workspace, path safety, error type, dan filesystem layer.

## Pencapaian Saat Ini
- Workspace resolve aman dalam root.
- Operasi file/folder: create, read, write, delete, rename.
- Tree cache untuk performa listing explorer.

## Cara Pemakaian
1. Gunakan `Workspace` sebagai gateway operasi file.
2. Hindari akses fs langsung dari lapisan UI.
3. Tangani error melalui `CoreError`.
