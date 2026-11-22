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
- `pikieta-calc`: azymut i długość, biegunowe, domiary, rzutowanie na prostą, wcięcia i przecięcia, pole ze współrzędnych z poprawką odwzorowawczą, transformacja Helmerta i afiniczna, ciągi poligonowe, niwelacja, tachimetria, tyczenie, podział działek; później wyrównanie metodą najmniejszych kwadratów, NMT i trasy (pełna lista w „Obliczenia do uzupełnienia”).
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
| 4a | Obliczenia terenowe + DXF | ciągi, tachimetria, tyczenie, wcięcia, transformacje płaskie, podziały, wyrównanie pól, rysowanie ze snappingiem | 8+ tyg. |
| 4b | Wysokości, wyrównanie, instrumenty | niwelacja, transformacje wysokości i 3D, wyrównanie MNK, dzienniki pomiarów | 8+ tyg. |
| 5 | Android | interfejs terenowy, GNSS z telefonu, praca offline; później odbiorniki RTK po Bluetooth | 4–6 tyg. |
| 6 | Wydanie | instalator Windows, AppImage/deb, APK, PWA, dokumentacja PL/EN | 2–3 tyg. |
| 7 | NMT i obliczenia specjalistyczne | model TIN, objętości, warstwice, trasy, przekroje, suwnice | 8+ tyg. |

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

## Stan etapu 2: zamknięty

Zrobione:

- `pikieta-io`: odczyt i zapis tekstowych wykazów współrzędnych (numer, X, Y, opcjonalnie H i kod; kolejność XY albo YX; separatory: spacje, tabulatory, średniki, przecinki; przecinek dziesiętny; nagłówek i komentarze);
- tabela punktów: import i eksport TXT (także pliki w Windows-1250), ręczne dodawanie, dodawanie kliknięciem na mapie, wyszukiwanie, usuwanie;
- punkty na mapie z numerami, wskazywanie punktów kliknięciem, przeliczanie całego projektu przy zmianie układu;
- siedem zadań na punktach z projektu: azymut i długość, metoda biegunowa, rzutowanie na linię, punkt z domiarów, wcięcie liniowe, przecięcie prostych, pole ze współrzędnych;
- raport obliczeń z eksportem do TXT;
- projekt (układ, punkty, obliczenia, raport) zapamiętywany w pamięci przeglądarki.

Układ interfejsu (przebudowa 21.11.2026):

- wstążka u góry z kartami Projekt, Obliczenia, Mapa, Widok; zwijana;
- lista obliczeń po lewej jak lista rozmów w czacie: „Nowe obliczenie”, otwarte, historia pogrupowana po dniach, wyszukiwanie, zmiana nazwy, zamykanie i usuwanie;
- panel otwartego narzędzia obok mapy; przełącznik w rogu zmienia tryb: mapa z panelem, podział 50/50, obliczenia na pełnym ekranie; na telefonie podział jest pionowy, a lista wysuwa się jako szuflada;
- tabela punktów zadokowana pod mapą jak tabela atrybutów w QGIS: sortowanie, zmiana szerokości kolumn, zaznaczanie (Ctrl, Shift) wspólne z mapą, tryb edycji komórek, filtr zaznaczonych, przybliżanie, przewijanie wirtualne dużych zbiorów, zmiana wysokości panelu;
- obliczenia wsadowe w tabelach (azymuty, biegunowe, rzutowanie, domiary, wierzchołki pola) z nawigacją Enter/strzałki i wskazywaniem punktów kliknięciem na mapie; szkic obliczenia na mapie.

Domknięcie etapu (21.11.2026):

- zaokrąglanie w `pikieta-core` regułą „do parzystej” na zapisie dziesiętnym (12,345 → 12,34; 12,355 → 12,36); dokładność współrzędnych, długości, kątów i pól ustawiana w projekcie; hektary zawsze 0,0001 ha; ten sam sposób zaokrąglania w interfejsie, raporcie i eksporcie TXT;
- crate `pikieta-report`: raport jako dane (parametry, tabela, wyniki, uwagi) składany w Ruście do TXT i do HTML przygotowanego do druku na A4; w aplikacji podgląd, zapis TXT/HTML i druk z zapisem do PDF; stare wpisy raportu przechodzą jako tekst;
- zapis i odczyt projektu do pliku `.pikieta` (JSON: układ, punkty, obliczenia, raport, dokładność), skróty Ctrl+S i Ctrl+O;
- okna systemowe: w Tauri wtyczki `dialog` i `fs` z dostępem tylko do plików wskazanych w oknie; w przeglądarkach Chromium okno zapisu File System Access; w pozostałych pobieranie;
- zmiana kolejności wierszy w tabelach obliczeń przeciąganiem za kolumnę numerów (mysz i dotyk) oraz Alt+↑/↓;
- ostrzeżenie o samoprzecinającym się wieloboku w obliczeniu pola i w raporcie;
- okno Tauri na Windows sprawdzone przez zdalne debugowanie WebView2: interfejs i rdzeń działają, odczyt pliku spoza wskazanych jest blokowany.

Niesprawdzone, przeniesione na etap 5 (Android): systemowe okna zapisu i odczytu na Androidzie oraz w ręcznie obsługiwanym oknie Tauri (testy zrobione skryptem nie klikają w natywne okna), układ na fizycznym telefonie.

## Stan etapu 3

Zrobione (21.11.2026):

- `pikieta-gml`, bezstratne drzewo XML: zachowuje dokładny tekst znaczników, białe znaki, komentarze, CDATA i deklaracje; eksport bez zmian jest identyczny bajt w bajt z plikiem źródłowym, a edycja atrybutu zmienia tylko jego element;
- geometria GML 3.2: Point, LineString, LinearRing, Curve (LineStringSegment, Arc, ArcString, Circle zagęszczane do łamanej), Ring z curveMember, CompositeCurve, OrientableCurve, Polygon, Surface/PolygonPatch, Multi*, MultiGeometry; `posList` 2D i 3D, seria `pos`, `coordinates` z GML 2; przestrzeń nazw GML rozpoznawana z deklaracji `xmlns`;
- obiekty i atrybuty: kolekcje z `featureMember`/`featureMembers`/`member`, atrybuty proste, odwołania `xlink:href`, atrybuty zagnieżdżone spłaszczane do ścieżek (np. `idIIP.lokalnyId`); etykiety EGiB z identyfikatorów (numer działki, punktu, budynku);
- walidacja: powtórzone `gml:id`, odwołania lokalne bez celu, brak lub różne `srsName`, pierścienie niezamknięte, za krótkie i samoprzecinające się, wymagana geometria i `idDzialki`, powtórzone działki, punkty graniczne poza wierzchołkami działki (tolerancja 1 cm);
- zapis punktów projektu do GML 3.2 (schemat `urn:pikieta:punkty:1.0`);
- aplikacja: warstwy GML na mapie ze stylami klas EGiB i przeliczeniem do układu projektu, zaznaczanie kliknięciem, tabela obiektów jak tabela atrybutów (wybór klasy, wyszukiwanie, edycja prostych atrybutów), panel warstwy z klasami, widocznością i listą uwag walidacji (kliknięcie wskazuje obiekt), punkty graniczne do punktów projektu, warstwy zapamiętywane w IndexedDB i w pliku projektu.

Sprawdzone na syntetycznym pliku EGiB (`crates/pikieta-gml/tests/data/egib-przyklad.gml`, dane wymyślone).

Wydajność dużych plików (21.11.2026), po zgłoszeniu, że plik 150 MB blokował mapę:

- rdzeń: nazwy i białe znaki w drzewie XML współdzielone, znaczniki kanoniczne nieprzechowywane, atrybuty obiektów liczone na żądanie; wczytanie 150 MB (157 tys. obiektów) natywnie 5,0 s → 2,9 s, zapis dalej bajt w bajt;
- do mapy trafia jeden bufor binarny z geometrią już w układzie projektu (11 MB, 0,45 s), zamiast JSON z zagnieżdżonymi tablicami; do interfejsu lekki indeks (3 MB), a atrybuty tylko dla pokazanej klasy albo zaznaczonego obiektu;
- przeglądarka: dokument w Web Workerze z WASM, interfejs płynny w trakcie wczytywania (150 MB w 9,7 s, ponowne wczytanie z IndexedDB 3,7 s);
- aplikacja desktopowa: plik czyta i przetwarza natywny Rust (komendy Tauri), bufor geometrii przez binarną odpowiedź IPC; 150 MB w 3,9 s; rdzeń kompilowany z optymalizacją także w `tauri dev`;
- mapa: geometria GML rysowana przez WebGL (w WebView2 na sprzętowym GPU przez Direct3D 11), etykiety i zaznaczenie w lekkich warstwach canvas, wybór kliknięciem przez indeks przestrzenny; punkty graniczne rysowane dopiero poniżej 1,5 m/px;
- edycje atrybutów zapisywane jako dziennik i odtwarzane po ponownym wczytaniu źródła (plik w IndexedDB albo ścieżka na dysku).

Pomiary płynności przesuwania mapy z 157 tys. obiektów: przy przybliżeniu ok. 60 klatek/s; z całą warstwą w widoku mediana 17 ms na klatkę, ale pojedyncze klatki 100–300 ms przy przebudowie buforów WebGL.

Druga runda (21.11.2026), po uwagach z testu na prawdziwym pliku (127 tys. obiektów):

- własny renderer WebGL2 (`glrender.ts`) zamiast warstwy wektorowej OpenLayers: bufory budowane raz po wczytaniu, przy przesuwaniu i przybliżaniu zmienia się tylko macierz; linie jako instancjonowane prostokąty (grubość, przerywanie, biała obwódka), punkty jako koła w shaderze, wypełnienia z triangulacji `earcut`;
- style EGiB, GESUT i BDOT500 według katalogu znaków umownych mapy zasadniczej (D20211385, zał. 4): rdzeń wylicza kod znaku z klasy i atrybutu rodzaju (np. `EGDE`, `SUKS`, `OTKJ`, pierwszeństwo ma `kodObiektu` z pliku), mapa bierze z katalogu kolor RGB, grubość linii w mm (1 mm = 96/25,4 px, co najmniej 1 px), przerywanie (także kreska z kropką) i kolejność prezentacji; warianty: punkt graniczny stabilizowany (okrąg) lub nie (kropka), budynek podziemny przerywany, przewód nadziemny i projektowany (3 mm na 1 mm);
- opisy jak w katalogu: budynek (rodzaj KŚT i kondygnacje), użytek (OFU), kontur (OZU+OZK), przewód (sieć, typ, średnica, „proj.”, „niecz.”; nadziemne elektroenergetyczne i telekomunikacyjne bez opisu), pikieta (wysokość); znaki złożone (kreski skarp, szrafy, symbole drzew i urządzeń) uproszczone do linii, przerywania i koła, biała obwódka 0,5 px dla czytelności na ortofotomapie;
- etykiety tworzone partiami w tle, style z pamięci podręcznej na klasę i wariant, indeks przestrzenny bez obiektów pośrednich (0,46 s → 0,3 s), atrybuty tabeli pobierane po geometrii (pierwsza klasa 2,5–3 s → ok. 1 s);
- interfejs: aktywne zakładki z pełną ramką, tabela punktów z wierszem wpisu na końcu (Enter dodaje punkt, przejście do kolejnego wiersza), edycja włączona domyślnie;
- pomiary włączane przez `localStorage['pikieta.debug'] = '1'` (konsola).

Znane ograniczenia: pamięć procesu desktopowego to ok. 0,9 GB na dokument 150 MB (drzewo XML w pamięci); duże warstwy nie trafiają do pliku projektu w przeglądarce (powyżej 30 MB), na desktopie zapisuje się ścieżka; na Androidzie działa wątek roboczy z WASM (niesprawdzone na urządzeniu).

Do zamknięcia etapu:

- test na prawdziwym pliku EGiB z ośrodka dokumentacji (zanonimizowanym), w tym wydajność dla dużych plików;
- profile GESUT i BDOT500: style są, do sprawdzenia na prawdziwych plikach (etykiety, rozpoznawanie sieci);
- walidacja względem schematów XSD tylko w zakresie reguł, bez pełnego walidatora.

## Stan etapu 4a

Etap 3 pozostaje otwarty do sprawdzenia na prawdziwych plikach EGiB, GESUT i BDOT500.

Zrobione (22.11.2026):

- ciąg poligonowy w `pikieta-calc`: dwustronnie nawiązany, zamknięty i wiszący, kąty lewe lub prawe; odchyłka kątowa i odchyłki przyrostów, wyrównanie przybliżone (poprawki kątów po równo, przyrostów proporcjonalnie do długości boków); testy na danych bez błędów, z błędem kąta i boku, kątach prawych, ciągu zamkniętym i wiszącym;
- zadanie „Ciąg” w grupie Osnowa: wiersze od punktu początkowego (kąt na punkcie, bok do następnego), opcjonalna redukcja odwzorowawcza długości (skala boku z położenia przybliżonego, drugi przebieg), odchyłka kątowa dopuszczalna 2·mα·√n i względna liniowa 1 : N wpisywane przez użytkownika, nowe punkty do projektu, raport z poprawkami i przyrostami.

Następne w etapie 4a: tachimetria, tyczenie, wcięcie wstecz.

## Obliczenia do uzupełnienia

Porównanie z zakresem obliczeń C-Geo (instrukcja użytkownika, tomy I i II, oraz wykaz zmian 2013–2022; pliki są tylko lokalnie, w folderze `inspiracje/` wyłączonym z repozytorium). Pikieta ma dziś: azymut i długość, metodę biegunową, rzutowanie na prostą i punkty z domiarów, wcięcie liniowe, przecięcie prostych i kierunków, pole ze współrzędnych z redukcją na elipsoidę oraz przeliczenia PL-2000/1992/UTM.

Etap 4a, obliczenia terenowe (kolejność według potrzeb w codziennej pracy):

- ciąg poligonowy: dwustronnie nawiązany, zamknięty, wiszący; odchyłki kątowa i liniowa z dopuszczalnymi, wyrównanie przybliżone, redukcje odwzorowawcze długości;
- tachimetria: stanowisko z orientacją na jeden lub kilka punktów, XYH pikiet, redukcje (odwzorowawcza, na poziom odniesienia, krzywizna i refrakcja), średnia z dwóch położeń lunety;
- tyczenie: elementy biegunowe (kąt lub azymut i odległość) ze stanowiska z orientacją, także dla wielu punktów naraz, z raportem;
- wcięcia: wstecz (z trzech punktów), kątowe w przód z kątów pomierzonych na stanowiskach, kombinowane, wcięcie z wyrównaniem przy nadliczbowych obserwacjach, przestrzenne wcięcie w przód (XYH);
- konstrukcje: prosta prostopadła, równoległa, pod zadanym kątem lub azymutem, przecięcie prostej z okręgiem, punkt ukryty (pomiar mimośrodowy), punkty na łuku kołowym (ze spadkiem), podział odcinka;
- transformacje płaskie: Helmerta (podobieństwa), afiniczna, z korektą Hausbrandta, raport odchyłek na punktach dostosowania;
- pola: wyrównanie pól (działka i użytki, rozrzucenie odchyłki), wykaz zmian danych ewidencyjnych na podstawie GML EGiB;
- podział działki na zadaną powierzchnię: równolegle do boku, z punktu, na równe części, z powtarzaniem kryterium;
- uśrednianie współrzędnych i porównanie dwóch zbiorów punktów (odchyłki, także z weryfikacją atrybutów, np. kodu);
- redukcja długości: odwzorowawcza i na poziom terenu jako osobne zadanie (rdzeń już to liczy).

Etap 4b, wysokości, wyrównanie i dane z instrumentów:

- niwelacja techniczna: ciąg nawiązany i zamknięty, punkty pośrednie, odchyłka dopuszczalna; później niwelacja precyzyjna (łaty, poprawki temperaturowe);
- transformacje wysokości (Kronsztadt86 ↔ EVRF2007, geoida PL-geoid) i przestrzenna 3D (7 parametrów, PL-2000 ↔ XYZ ETRF); wymagają siatek, razem z układem 1965;
- wyrównanie ścisłe metodą najmniejszych kwadratów: sieci poziome, wysokościowe i 3D/GNSS, wyrównanie swobodne, błędy średnie, elipsy błędów, wykrywanie błędów grubych;
- dziennik kątów i boków: obserwacje z instrumentów (średnie z serii, redukcje), import do ciągów, tachimetrii i wyrównania;
- dziennik pomiarów RTK/RTN: uśrednianie powtórzeń, przyrosty, kontrola na punktach osnowy, dopasowanie do osnowy lokalnej.

Etap 7, numeryczny model terenu i zastosowania specjalistyczne (po wydaniu 1.0):

- model TIN z punktów XYH i linii nieciągłości, pole powierzchni 3D, objętości (mas ziemnych, między dwoma modelami, do płaszczyzny), warstwice, przekrój pionowy, rzutowanie punktów na model, mapa spadków;
- trasy: proste, łuki kołowe, klotoidy, łuki koszowe, łuki pionowe, kilometraż, punkty dodatkowe i kontrolowane, przekroje poprzeczne;
- rzutowanie pomiarów liniowych na dane projektowe (kontrola realizacji, inwentaryzacja);
- pomiary torów suwnicowych (wpasowanie, odchyłki, wykresy);
- kalibracja (wpasowanie) rastra na mapie.

Poza zakresem obliczeń, ale z tych samych instrukcji: kontrola spójności i topologii mapy, transformacja całej mapy, przecięcie warstw.

## Ryzyka

- **Zakres GML**: rozrośnie się bez twardych profili i prawdziwych plików testowych z ośrodków.
- **Wiarygodność wyników**: geodeta musi dostać to samo co w uznanym programie, stąd testy porównawcze od początku.
- **Inne usługi WMS/WMTS** (KIEG, GESUT): nagłówki CORS trzeba sprawdzić osobno dla każdej.
