# src/app/ui Folder

## Tujuan
Merender semua panel dan komponen tampilan aplikasi.

## Pencapaian Saat Ini
- Layout dinamis untuk Explorer/AI Chat/Editor/Terminal/Settings.
- Panel AI Chat mengambil area kiri (50 persen saat aktif).
- Shortcut dan status ditampilkan adaptif di footer.

## Cara Pemakaian
1. Fungsi `draw` sebagai root render.
2. Update komponen panel lewat fungsi `draw_*`.
3. Ikuti tema dari `style` agar konsisten.
