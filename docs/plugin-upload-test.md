# Plugin Upload Test Fixture

Repo ini sekarang punya fixture plugin sederhana untuk uji coba alur upload:

- Source plugin: `plugin_samples/hello-hook`
- Pack script: `scripts/package-plugin.ps1`
- Output zip: `plugin_samples/dist/hello-hook-0.1.0.zip`

## Build paket zip

```powershell
./scripts/package-plugin.ps1
```

## Simulasi instal di mekanisme saat ini

Karena installer plugin resmi belum ada, pasang plugin dengan menyalin isi `hooks.conf` ke:

`<workspace>/.xone/hooks.conf`

Lalu restart app.

## Shortcut dari plugin contoh

- `Ctrl+Alt+H` -> tampilkan message
- `Ctrl+Alt+R` -> kirim command ke terminal
- `Ctrl+Alt+G` -> buka `Cargo.toml`
