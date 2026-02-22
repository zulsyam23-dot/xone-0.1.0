# .xone Folder

## Tujuan
Menyimpan konfigurasi lokal per-workspace untuk Xone.

## File Penting
- `ui.conf`: tema, density, dan preferensi UI.
- `hooks.conf`: binding shortcut ke action hook.
- `ai/ai.conf`: konfigurasi provider AI.

## Fitur Konfigurasi
- Isolasi konfigurasi per project.
- Mudah dibackup dan dipindah antar mesin.
- Bisa dikontrol lewat Settings tanpa edit manual untuk item utama AI.

## Kelebihan
- Perubahan setting tidak mengganggu project lain.
- Mudah audit perubahan konfigurasi.
- Fleksibel untuk profile lokal vs cloud.

## Cara Pakai
1. Ubah via UI Settings jika tersedia.
2. Untuk fine-tuning, edit file `.conf` langsung.
3. Restart app jika perubahan belum terbaca otomatis.
