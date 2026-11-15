# Pikieta

Otwarty kalkulator geodezyjny z mapą, pisany w Ruście. Działa w przeglądarce, na Windows, Linux i Androidzie z jednego kodu.

Projekt jest na wczesnym etapie: działa mapa z ortofotomapą Geoportalu w układach PL-2000 i PL-1992 oraz jedno zadanie (azymut i długość). Dalsze kroki opisuje [PLAN.md](PLAN.md).

## Struktura

| Katalog | Zawartość | Licencja |
|---|---|---|
| `crates/pikieta-core` | typy podstawowe: punkty, kąty | MIT OR Apache-2.0 |
| `crates/pikieta-calc` | obliczenia geodezyjne | MIT OR Apache-2.0 |
| `crates/pikieta-crs` | układy współrzędnych (etap 1) | MIT OR Apache-2.0 |
| `crates/pikieta-gml` | odczyt i zapis GML (etap 3) | MIT OR Apache-2.0 |
| `crates/pikieta-wasm` | wiązania WebAssembly rdzenia | MIT OR Apache-2.0 |
| `app` | interfejs (TypeScript, Svelte, OpenLayers) | GPL-3.0-or-later |
| `app/src-tauri` | powłoka Tauri 2 (desktop, Android) | GPL-3.0-or-later |

## Uruchomienie

Wymagane: Rust (stable) z celem `wasm32-unknown-unknown` oraz Node.js 22.

```bash
rustup target add wasm32-unknown-unknown
cd app
npm install
npm run dev          # wersja przeglądarkowa: http://localhost:5173
npm run tauri dev    # okno desktopowe
```

Testy rdzenia: `cargo test` w katalogu głównym.

Android wymaga Android SDK i NDK oraz JDK 17: `npm run tauri android init`, potem `npm run tauri android dev`.

## Licencje

Crate'y w `crates/` są dostępne na licencji MIT ([LICENSE-MIT](LICENSE-MIT)) albo Apache 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), do wyboru. Aplikacja w `app/` jest na licencji GPL 3.0 lub nowszej ([app/LICENSE](app/LICENSE)).

Ortofotomapa pochodzi z usługi WMS Głównego Urzędu Geodezji i Kartografii (geoportal.gov.pl).
