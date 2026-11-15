# Plan projektu Pikieta

Kalkulator geodezyjny z mapą w Ruście: połączenie zadań obliczeniowych w stylu WinKalk i C-Geo z lekką przeglądarką mapową. Wczytuje i eksportuje GML. Działa na Windows, Linux, Androidzie i w przeglądarce. Open source.

## Decyzje

| Temat | Decyzja |
|---|---|
| Nazwa | Pikieta (pierwotnie „GeoRust", odrzucone przez kolizję z organizacją github.com/georust) |
| Licencja rdzenia (`crates/`) | MIT OR Apache-2.0 |
| Licencja aplikacji (`app/`) | GPL-3.0-or-later |
| Frontend | TypeScript + Svelte + OpenLayers |
| Powłoka natywna | Tauri 2 (desktop i Android) |
| „Przez sieć" | wersja przeglądarkowa bez serwera; dane zostają u użytkownika |
| Pierwszy profil GML | EGiB |

## Architektura

Cała logika jest w czystym Ruście bez zależności od C, więc ten sam kod kompiluje się natywnie i do `wasm32`. Interfejs jest jedną aplikacją webową, która woła rdzeń przez WebAssembly na każdej platformie. Tauri tylko ją opakowuje i daje dostęp do plików oraz (później) GNSS.

Crate'y rdzenia:

- `pikieta-core`: typy punktów, kąty w gradach i stopniach, reguły zaokrągleń, tolerancje.
- `pikieta-crs`: PL-2000 (EPSG:2176–2179), PL-1992, PL-UTM, ETRF2000 na własnej implementacji Gaussa-Krügera. Układ 1965 i wysokości (Kronsztad86 ↔ EVRF2007, geoida) później, bo wymagają siatek.
- `pikieta-calc`: azymut i długość, biegunowe, domiary, rzutowanie na prostą, wcięcia i przecięcia, pole ze współrzędnych z poprawką odwzorowawczą, transformacja Helmerta i afiniczna, ciągi poligonowe, niwelacja, tachimetria, tyczenie, podział działek; później wyrównanie metodą najmniejszych kwadratów.
- `pikieta-gml`: odczyt i zapis na `quick-xml`.
- `pikieta-io` (do dodania): TXT/CSV, DXF, GeoJSON; później formaty instrumentów.
- `pikieta-project` (do dodania): model projektu, warstwy, undo/redo, indeks przestrzenny, wersjonowany plik projektu.
- `pikieta-report` (do dodania): raporty obliczeń do operatu (HTML/PDF).
- `pikieta-wasm`: cienkie wiązania dla frontendu.

## Zakres GML

Pełny GML jest nierealny, więc obsługa idzie profilami:

1. Geometria GML 3.2: punkty, linie, poligony, łuki, multi-geometrie, `srsName`.
2. EGiB: działki, punkty graniczne, budynki, kontury, z powiązaniami `xlink:href`.
3. GESUT i BDOT500.

Nieznane atrybuty muszą przetrwać cykl import → eksport bez strat. Walidacja jest własna, regułowa.

## Etapy

Rozmiary to zgrubny szacunek dla jednej osoby na pełny etat.

| # | Etap | Kryterium ukończenia | Rozmiar |
|---|---|---|---|
| 0 | Fundament i prototypy ryzyka | CI buduje 4 cele; na każdym działa mapa z ortofotomapą w PL-2000 i pole liczbowe | 2–3 tyg. |
| 1 | `pikieta-crs` + podstawowe `pikieta-calc` + CLI | wyniki zgodne co do mm z punktami kontrolnymi GUGiK/PROJ oraz z C-Geo/WinKalk | 4–6 tyg. |
| 2 | MVP desktop + web | tabela punktów, import/eksport TXT, punkty na mapie, 5–6 zadań z raportem | 6–8 tyg. |
| 3 | GML | plik EGiB wczytany na mapę i wyeksportowany bez strat | 6–8 tyg. |
| 4 | Obliczenia zaawansowane + DXF | ciągi, transformacje, podziały, rysowanie ze snappingiem | 8+ tyg. |
| 5 | Android | interfejs terenowy, GNSS z telefonu, praca offline; później odbiorniki RTK po Bluetooth | 4–6 tyg. |
| 6 | Wydanie | instalator Windows, AppImage/deb, APK, PWA, dokumentacja PL/EN | 2–3 tyg. |

## Stan etapu 0

Zrobione:

- workspace Cargo z crate'ami rdzenia i pierwszym zadaniem (azymut i długość) z testami;
- frontend z mapą OpenLayers i rdzeniem wołanym przez WASM;
- powłoka Tauri 2;
- workflowy GitHub Actions: testy rdzenia, build web, desktop (Windows, Linux), APK debug, deploy na GitHub Pages.

Ustalenia z prototypów:

- WMS ortofotomapy Geoportalu (`PZGIK/ORTO/WMS/StandardResolution`) podaje obraz w EPSG:2176–2180 i wysyła `Access-Control-Allow-Origin: *`, więc wersja przeglądarkowa nie potrzebuje proxy.
- Zapytania idą w WMS 1.1.1, żeby uniknąć odwróconej kolejności osi z 1.3.0.

Do zamknięcia etapu:

- potwierdzić zielone CI dla wszystkich czterech celów (Android nie był budowany lokalnie);
- sprawdzić klawiaturę numeryczną i mapę na fizycznym telefonie;
- zebrać zestaw testów porównawczych z C-Geo/WinKalk na etap 1.

## Ryzyka

- **Zakres GML**: rozrośnie się bez twardych profili i prawdziwych plików testowych z ośrodków.
- **Wiarygodność wyników**: geodeta musi dostać to samo co w uznanym programie, stąd testy porównawcze od początku.
- **Inne usługi WMS/WMTS** (KIEG, GESUT): nagłówki CORS trzeba sprawdzić osobno dla każdej.
