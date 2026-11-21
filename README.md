# Pikieta

Otwarty kalkulator geodezyjny z mapą, pisany w Ruście. Działa w przeglądarce, na Windows, Linux i Androidzie z jednego kodu.

Projekt jest na wczesnym etapie. Działa mapa z ortofotomapą lub BDOT10k z Geoportalu, tabela punktów z importem i eksportem TXT, siedem zadań obliczeniowych z raportem oraz przeliczenia między PL-2000, PL-1992 i UTM. Dalsze kroki opisuje [PLAN.md](PLAN.md).

## Struktura

| Katalog | Zawartość | Licencja |
|---|---|---|
| `crates/pikieta-core` | typy podstawowe: punkty, kąty | MIT OR Apache-2.0 |
| `crates/pikieta-calc` | obliczenia geodezyjne | MIT OR Apache-2.0 |
| `crates/pikieta-crs` | układy PL-2000, PL-1992, UTM (Gauss-Krüger) | MIT OR Apache-2.0 |
| `crates/pikieta-io` | tekstowe wykazy współrzędnych | MIT OR Apache-2.0 |
| `crates/pikieta-gml` | odczyt i zapis GML (etap 3) | MIT OR Apache-2.0 |
| `crates/pikieta-cli` | polecenie `pikieta` | MIT OR Apache-2.0 |
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

Wiersz poleceń:

```bash
cargo run -p pikieta-cli -- transformuj 2178 2180 5788456.49 7500833.51
cargo run -p pikieta-cli -- pomoc
```

Android wymaga Android SDK i NDK oraz JDK 17: `npm run tauri android init`, potem `npm run tauri android dev`.

## Licencje

Crate'y w `crates/` są dostępne na licencji MIT ([LICENSE-MIT](LICENSE-MIT)) albo Apache 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), do wyboru. Aplikacja w `app/` jest na licencji GPL 3.0 lub nowszej ([app/LICENSE](app/LICENSE)).

Ortofotomapa pochodzi z usługi WMS Głównego Urzędu Geodezji i Kartografii (geoportal.gov.pl).
