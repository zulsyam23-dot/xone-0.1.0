# .xone/ai Folder

## Tujuan
Menyimpan konfigurasi AI terpusat untuk panel AI Chat.

## Fitur
- Dukungan provider:
  - `ollama`
  - `openai_compatible` (alias: `openai`, `chatgpt`, `groq`, `together`, `openrouter`, `mistral`)
- Quick profile switch dari Settings:
  - `1` = ollama-local
  - `2` = openai
  - `3` = groq
- Validasi key: jika provider butuh API key dan kosong, user diberi link daftar key.

## Kelebihan
- Satu format config untuk banyak provider.
- Bisa mulai tanpa cloud (Ollama lokal).
- Mudah migrasi ke provider cloud saat dibutuhkan.

## Struktur `ai.conf`
- `provider`
- `base_url`
- `model`
- `api_key`
- plus opsi lanjutan (template): `timeout_ms`, `temperature`, `max_tokens`, `stream`, `system_prompt`

## Cara Pakai
1. Buka `ai.conf` dan isi profile aktif.
2. Untuk provider cloud, isi `api_key` valid.
3. Buka AI Chat (`F6`) dan tes prompt.
4. Gunakan Settings untuk switch profile cepat.
