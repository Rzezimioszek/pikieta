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
- workflowy GitHub Actions: testy rdzenia, build web, desktop (Windows, Linux), APK debug, deploy na GitHub Pages; pierwsze uruchomienie przeszło dla wszystkich czterech celów;
- wersja przeglądarkowa jest opublikowana pod https://rzezimioszek.github.io/pikieta/.

Ustalenia z prototypów:

- WMS ortofotomapy Geoportalu (`PZGIK/ORTO/WMS/StandardResolution`) podaje obraz w EPSG:2176–2180 i wysyła `Access-Control-Allow-Origin: *`, więc wersja przeglądarkowa nie potrzebuje proxy.
- Zapytania idą w WMS 1.1.1, żeby uniknąć odwróconej kolejności osi z 1.3.0.

Do zamknięcia etapu:

- sprawdzić klawiaturę numeryczną i mapę na fizycznym telefonie;
- zebrać zestaw testów porównawczych z C-Geo/WinKalk na etap 1.

## Stan etapu 1: zamknięty

Etap zamknięty 15.11.2026 decyzją autora projektu, na podstawie zgodności z PROJ. Porównanie z C-Geo/WinKalk nie zostało wykonane.

Zrobione:

- `pikieta-crs`: odwzorowanie Gaussa-Krügera szeregami Krügera (do n⁴) na GRS80; układy PL-2000 (EPSG:2176–2179), PL-1992 (EPSG:2180) i UTM 33–35 (EPSG:25833–25835); przeliczenia płaskie ↔ geodezyjne i między układami;
- testy względem programu `proj` (PROJ): 24 punkty kontrolne, zgodność do 0,02 mm, także 9° od południka osiowego;
- `pikieta-calc`: azymut i długość, metoda biegunowa, domiary prostokątne w obie strony, przecięcie prostych i kierunków (wcięcie kątowe w przód), wcięcie liniowe, pole ze współrzędnych;
- skala odwzorowania i zbieżność południków w punkcie (zgodne z `proj -V` do 1e-8 i 1e-7°), średnia skala odcinka wzorem Simpsona, długość i pole zredukowane na elipsoidę (pole: skala w środku ciężkości, przybliżenie dla działek i obrębów);
- `pikieta-cli`: polecenie `pikieta` (transformuj, xy, bl, azymut, pole, skala, redukcja, pole-elipsoida);
- interfejs: zmiana układu przelicza wpisane punkty, pod kursorem widać B i L.

Przeniesione na później:

- porównanie wyników z C-Geo/WinKalk i punktami kontrolnymi GUGiK, gdy będą dostępne dane (testy do dopisania w dowolnym momencie);
- reguły zaokrągleń i formatowania wyników w `pikieta-core`: etap 2, razem z raportami;
- redukcja długości na poziom terenu: etap 4, razem z wysokościami.

Poza zakresem etapu 1 były od początku: układ 1965, wysokości, wcięcie wstecz, transformacje Helmerta, ciągi.

## Stan etapu 2

Zrobione:

- `pikieta-io`: odczyt i zapis tekstowych wykazów współrzędnych (numer, X, Y, opcjonalnie H i kod; kolejność XY albo YX; separatory: spacje, tabulatory, średniki, przecinki; przecinek dziesiętny; nagłówek i komentarze);
- tabela punktów: import i eksport TXT (także pliki w Windows-1250), ręczne dodawanie, dodawanie kliknięciem na mapie, wyszukiwanie, usuwanie;
- punkty na mapie z numerami, wskazywanie punktów kliknięciem, przeliczanie całego projektu przy zmianie układu;
- siedem zadań na punktach z projektu: azymut i długość, metoda biegunowa, rzutowanie na linię, punkt z domiarów, wcięcie liniowe, przecięcie prostych, pole ze współrzędnych;
- raport obliczeń z eksportem do TXT;
- projekt (układ, punkty, raport) zapamiętywany w pamięci przeglądarki.

Do zamknięcia etapu:

- zapis plików w aplikacji desktopowej i na Androidzie przez okna systemowe (teraz eksport działa jak pobieranie w przeglądarce i nie był sprawdzany w Tauri);
- reguły zaokrągleń i formatowania wyników w `pikieta-core` (teraz formatuje interfejs, stałą liczbą miejsc);
- składanie raportu w rdzeniu (`pikieta-report`) zamiast w interfejsie, eksport do HTML/PDF;
- zapis i odczyt projektu do pliku;
- edycja istniejącego punktu w tabeli;
- wygodniejszy układ na telefonie (panel zajmuje ponad połowę ekranu).

## Ryzyka

- **Zakres GML**: rozrośnie się bez twardych profili i prawdziwych plików testowych z ośrodków.
- **Wiarygodność wyników**: geodeta musi dostać to samo co w uznanym programie, stąd testy porównawcze od początku.
- **Inne usługi WMS/WMTS** (KIEG, GESUT): nagłówki CORS trzeba sprawdzić osobno dla każdej.
