# Łatwość obsługi Pikiety: jedno wejście do zadań, jedna historia, panel warstw, ostrzeżenia, które wyjaśniają

* **Autor:** Rzezimioszek
* **Data:** 2026-11-27

## Przegląd

Pikieta jest dla geodetów w terenie i w biurze oraz dla gisowców. Dziś interfejs wymaga nauki. Obliczenia są dostępne w trzech miejscach: we wstążce, w lewym panelu i w prawym panelu. Użytkownik gubi się przy wyborze zadania, przy szukaniu historii obliczeń i przy wyborze podkładów i nakładek, które są rozbite na małe listy.

Propozycja ma cztery części:

- jedno miejsce wyboru zadania;
- jedna historia obliczeń;
- panel warstw z kolejnością i dodawaniem warstw;
- okna zadań, które wyjaśniają dane wejściowe i ostrzegają o wątpliwych wynikach, ale niczego nie ukrywają ani nie upraszczają.

Gisowiec ma mniejszą wiedzę geodezyjną, ale chce tego samego wyniku co geodeta.

**Kryterium sukcesu:** nowa osoba bez pomocy wykonuje dwa zadania:

- (a) tachimetrię od wczytania punktów do raportu z rysunkiem;
- (b) transformację płaską.

**Typowy ciąg pracy:** import punktów, potem obliczenia, potem kolejne obliczenia na punktach z poprzednich, potem raport z rysunkami i na końcu eksport (eksport jest poza zakresem).

**Priorytety:** biuro i komputer są pierwsze. Telefon jest drugi. Na nim liczą się podgląd danych na mapie, wpis aktualnej lokalizacji i dodanie zdjęcia. Obliczenia na telefonie muszą być, ale mogą zostać trudniejsze w obsłudze.

## Analiza

Przegląd siedmiu recenzentów (Goldfish) w kodzie potwierdza problem nawigacji. Pokazuje też, że **samo przemeblowanie interfejsu nie spełni kryterium sukcesu**: raport nie może dziś zawierać rysunku, a ostrzeżenie o punkcie odstającym w transformacji praktycznie nigdy się nie włączy. Dwie pułapki przy pierwszym kontakcie ważą dla zadania (a) więcej niż nawigacja. Pierwsza to ciche zamienienie kolumn X/Y przy imporcie. Druga to zmiana układu, która bez pytania przelicza wszystkie punkty.

### Blokery

**B1. Ostrzeżenie o odstającym punkcie w transformacji nie może zadziałać przy typowej liczbie punktów (błąd w kodzie, sprawdzony).**

- **Reguła:** punkt jest oznaczany, gdy `v > 3·√2·m0` (`Transform.svelte:79, 88`). Jednak m0 liczy się z tych samych odchyłek: `m0² = Σv² / r`, gdzie r = 2n − u.
- **Dlaczego nie działa:** pojedyncza odchyłka nie przekroczy sumy, więc `v_i² ≤ r·m0²`. Warunek `v_i² > 18·m0²` jest możliwy dopiero przy r > 18, czyli od 12 punktów dostosowania (Helmert) albo od 13 (afiniczna). Typowo jest ich 4–8.
- **Co to maskuje:** gruby błąd dodatkowo zawyża m0. Korekta Hausbrandta, domyślnie włączona, rozprowadza go na punkty transformowane.
- **Proponowana poprawka w Rust:** test „bez jednego punktu”. Dopasowanie liczy się bez punktu i, a odchyłkę punktu i porównuje się z m0 tego zredukowanego dopasowania.
- **Zmiany w interfejsie:**
  - flaga „użyj w dopasowaniu” w każdym wierszu;
  - informacja „brak kontroli”, gdy nie ma nadliczbowych obserwacji;
  - próg widoczny obok wyniku;
  - komunikat „sprawdź”, a nie „błąd”;
  - żadnego automatycznego wyłączania punktów.
- **Test:** 5 punktów Helmerta, jeden przesunięty o 0,30 m, musi zostać oznaczony. Dziś ten test nie przechodzi.
- **Stan (27.11.2026): naprawione.**
  - `check_control` w `pikieta-calc` liczy studentyzowaną odchyłkę bez punktu. Statystyka `d² / (2·(1+h)·m0²)` ma rozkład F(2, r), a kwantyl 1% ma postać zamkniętą. W m0 przyjmowana jest dolna granica 0,1 mm.
  - Kontrola działa od 4 punktów (Helmert) albo 5 (afiniczna). Przy mniejszej liczbie okno pokazuje „brak kontroli”.
  - W oknie: flaga „Użyj”, kolumna „Bez punktu / dop.”, komunikat „sprawdź” z wyjaśnieniem. Raport zawiera opis testu.
  - Test jednostkowy w Rust przechodzi. W przeglądarce punkt przesunięty o 30 cm daje 306,5 mm przy dopuszczalnych 17,7 mm, a po jego wyłączeniu m0 spada z 92 do 2 mm.

**B2. Raport nie mieści rysunku.**

- `ReportEntry` (`project.svelte.ts:8-16`) i `Entry` w `pikieta-report` nie mają pola na rysunek.
- Szkic (`Sketch`) trafia na mapę i znika.
- **Minimalna poprawka:**
  - „Zapisz w raporcie” zapisuje szkic we współrzędnych projektu;
  - `pikieta-report` rysuje go jako SVG z kierunkiem północy, podziałką, opisami i nazwą układu;
  - wersja TXT może rysunek pominąć.
- **Czego nie robić:** nie fotografować mapy z podkładem. Część usług WMS nie ma CORS, więc obraz byłby pusty albo zablokowany.
- **Uwaga na zapis:** projekt leży w localStorage, a nieudany zapis jest dziś cichy. Duże rysunki albo zdjęcia mogą przepaść bez komunikatu.
- To największy pojedynczy element i trzeba go zaplanować jako osobną funkcję.
- **Decyzja autora:** rysunek w raporcie wchodzi w zakres tej pracy.

**B3. Zadanie testowe (a) „od wczytania punktów” jest dla tachimetrii niedookreślone.**

- Import czyta tylko współrzędne.
- Tachimetria nie ma wklejania ani importu obserwacji, a transformacja ma okno wklejania.
- Import z rejestratorów jest w planie na później.
- **Minimalnie:** to samo okno wklejania w tachimetrii dla wierszy „Nr kod Hz V S hc”.
- **Test:** trzeba podać wprost pliki testowe, czyli TXT z punktami nawiązania i blok obserwacji. Inaczej test mierzy szybkość pisania, a nie czytelność.

**B4. Import i zmiana układu cicho dają złe współrzędne.**

- **Import:**
  - kolejność kolumn X/Y to osobna lista z dala od przycisku importu (`Ribbon.svelte:157-163`);
  - nie ma podglądu, a gisowiec zakładający X = wschód dostaje punkty o setki kilometrów obok;
  - punkty o tych samych numerach są nadpisywane bez pytania.
- **Zmiana układu:** `changeCrs` przelicza wszystkie punkty bez potwierdzenia.
- **Poprawki:**
  - podgląd pierwszych wierszy;
  - wykrywanie kolejności osi (w PL-2000 X ≈ 5,4–6,1 mln, a pierwsza cyfra Y to strefa);
  - zamiana X/Y jednym kliknięciem;
  - lista punktów do nadpisania;
  - dwie osobne akcje: „Przelicz punkty do układu…” i „Zmień układ bez przeliczania”, z potwierdzeniem liczby punktów;
  - opisy osi „X (północ)” i „Y (wschód)”.

**B5. Kryterium sukcesu nie da się dziś zmierzyć.**

- W aplikacji nie ma testów interfejsu (Playwright, Vitest).
- Kryterium nie określa liczby osób, ich doświadczenia, limitu czasu, warunku zaliczenia ani plików testowych.
- Protokół jest w części „Kluczowe decyzje”.

**B6. Sprzeczność: „jedno miejsce” a wstążka, z której korzysta doświadczony użytkownik.** Karta „Obliczenia” we wstążce jest domyślna i daje dostęp do każdego zadania jednym kliknięciem. Recenzenci proponują dwa wyjścia:

- **jedno okno wyboru, kilka sposobów otwarcia:** wstążka, „+” w panelu i Ctrl+K otwierają to samo okno z tymi samymi nazwami i opisami;
- **usunięcie karty ze wstążki:** szybkość dla doświadczonych dają wyszukiwanie, Ctrl+K i ostatnio używane zadania.

Zewnętrznych użytkowników jeszcze nie ma, więc koszt usunięcia jest mały.

**Decyzja autora:** wstążka zostaje jako skrót do wspólnego okna wyboru. Przyciski wstążki, „+” w panelu i Ctrl+K otwierają to samo okno z tymi samymi nazwami, opisami i wyszukiwaniem.

### Ważne

**M1. Trzy okna wyboru, każde inne, i słowa o kilku znaczeniach.**

- **Okna wyboru:**
  - Wstążka ma krótkie nazwy, a opis tylko w dymku, którego na dotyku nie widać.
  - Lewy panel ma skrócony launcher bez opisów.
  - Prawy panel ma pełny launcher z opisami, ale w domyślnym widoku mapy jest ukryty. Najlepsze okno wyboru jest więc przy pierwszym uruchomieniu niewidoczne.
- Wszystkie trzy wołają tę samą funkcję `startCalc`, więc połączenie ich to zmiana wyłącznie w interfejsie.
- **„Obliczenia”** to naraz karta wstążki, tryb widoku, przełącznik panelu, tytuł pustego panelu i etykieta historii.
- **„Podział”** to tryb widoku, podział działki i tryb w Konstrukcjach.
- **Inne powtórzenia:** przełącznik widoku występuje trzy razy, a przełącznik panelu dwa razy.
- **Poprawki:**
  - jeden launcher, zawsze z opisami i z wyszukiwaniem po nazwach, opisach i synonimach;
  - słownik pojęć: Zadanie (rodzaj), Obliczenie (jedna sesja), Historia obliczeń, Warstwy, Raport;
  - tryby widoku „Mapa”, „Mapa i obliczenie” i „Pełny ekran”.

**M2. Warstwy są w czterech osobnych miejscach i mają stałą kolejność rysowania.**

- **Gdzie są zapisane:** podkład i nakładki w ustawieniach urządzenia (localStorage), a warstwy GML w IndexedDB i w pliku projektu. Do tego dochodzą punkty projektu.
- **Kolejność rysowania:** podkład, nakładki według kolejności włączania, wszystkie GML w jednej warstwie WebGL, na końcu szkic i punkty.
- **Ukryty błąd:** przy 10 lub więcej włączonych nakładkach nakładka rysuje się nad GML.
- **Co da się zrobić od razu:** zmiana kolejności nakładek to tylko przestawienie tablicy, a kolejność GML między sobą wymaga jednego settera.
- **Czego nie robić bez przebudowy renderera:** przeplatanie WMS z GML. Ryzykuje utratę 60 kl./s przy 157 tys. obiektów.
- **Zalecenie:**
  - jeden panel ze stałymi, opisanymi pasmami: punkty i szkic, GML, nakładki, podkład (wybór jednego);
  - zmiana kolejności wewnątrz pasma;
  - przy usłudze niedostępnej w danym układzie widoczny powód;
  - stan ładowania i błędu przy każdej warstwie.
- **Do decyzji:** czy warstwy zapisywać w projekcie (zalecane), czy w urządzeniu, oraz jak przenieść dotychczasowe ustawienia.

**M3. Historia, raport i punkty nie są ze sobą powiązane.**

- Wpis raportu to zamrożona kopia bez odwołania do obliczenia. Dwukrotny zapis go dubluje, a po poprawce obliczenia raport jest nieaktualny i nic o tym nie mówi.
- Punkty nie wiedzą, z jakiego obliczenia pochodzą. Poprawione przeliczenie nie nadpisuje starych punktów, więc kolejne obliczenia po cichu biorą stare.
- **Poprawki:**
  - odwołania `calcId` w raporcie i `src` w punkcie;
  - w historii znaczniki „w raporcie” i „raport nieaktualny”;
  - ponowny zapis zastępuje wpis zamiast go dublować;
  - akcja „Zaktualizuj N punktów z tego obliczenia” z podglądem różnic.

**M4. Ciąg pracy rwie się na drugorzędnych przyciskach.**

- „Zapisz w raporcie” jest przyciskiem głównym, a „Dodaj do projektu” drugorzędnym. Kto naciśnie tylko główny, w następnym zadaniu zobaczy „brak punktu w projekcie”.
- W transformacji domyślny przedrostek jest pusty. Przy „Wszystkie punkty projektu” każdy numer jest zajęty, a przycisk dodawania jest wyłączony bez wyjaśnienia.
- **Poprawki:**
  - jedna akcja „Zapisz wynik” z zaznaczeniem „do projektu” i „do raportu”;
  - domyślny przedrostek albo wybór między zastąpieniem współrzędnych a nowymi numerami;
  - każdy wyłączony przycisk mówi, dlaczego jest wyłączony;
  - punkty dostosowania nie trafiają do wyników.

**M5. „Ostrzegaj o wątpliwych wynikach” nie jest określone poza transformacją.**

- **Stan dziś:**
  - W tachimetrii odchyłki orientacji nie mają progu, a jeden punkt nawiązania przechodzi bez słowa.
  - W ciągu odchyłki dopuszczalne działają tylko wtedy, gdy użytkownik je wpisze.
  - Wcięcia z wyrównaniem liczą m0, ale go nie sprawdzają.
- **Potrzebne są:**
  - jedna tabela progów (zadanie, statystyka, próg, wartość domyślna, komunikat, czy blokuje czy tylko ostrzega);
  - sprawdzenia liczone w Rust i pokryte testami;
  - jeden wspólny pasek ostrzeżeń w oknie zadania;
  - opis źródła każdego domyślnego progu (przepis albo wartość użytkownika) i możliwość jego edycji, żeby program nie udawał tolerancji prawnej.

**M6. Jeden interfejs dla geodety i gisowca, bez „trybu dla początkujących”.** Recenzenci są tu zgodni.

- **Co dziś przeszkadza:**
  - niewyjaśnione pojęcia: Stanowisko, Nawiązanie, Stała orientacji, krzywizna i refrakcja, redukcje;
  - dwie tabele tachimetrii bez nagłówków;
  - w transformacji „x pierwotny / X wtórny” i reguła pustych pól;
  - niewyjaśniony Hausbrandt;
  - kąty tylko w gradach;
  - krótkie nazwy zadań („Liniowe”, „Wstecz”), które poza grupą nic nie mówią.
- **Poprawki:**
  - fachowe nazwy zostają, bo idą też do raportu;
  - pod każdym polem jedno zdanie wyjaśnienia;
  - jednostki w etykietach;
  - numerowane sekcje („1. Orientacja”, „2. Pomierzone punkty”);
  - w transformacji przełącznik „punkty projektu są w układzie źródłowym/docelowym”.
- **Opcja dalej idąca:** pokazywać wpływ parametru na dane użytkownika, np. „wpływ redukcji: max 12 mm (pkt 1007)”.

**M7. Telefon to nowa praca, nie przeróbka.**

- Nie ma kodu lokalizacji, aparatu ani zdjęć.
- Zdjęcia w localStorage przekroczą limit, więc trzeba je trzymać w IndexedDB, tak jak GML, i ustalić, jak trafiają do pliku .pikieta.
- Zmiana nazwy, zamknięcie i usunięcie w historii są dostępne tylko po najechaniu myszą, więc na dotyku są niedostępne. Potrzebne jest widoczne menu ⋯.
- Jeden próg szerokości 900 px wrzuca małe laptopy w układ telefonu. Trzeba oddzielić telefon (około 600 px albo `pointer: coarse`) od wąskiego komputera.
- Cele telefonu nie mają jeszcze kryterium sukcesu.

**M8. Bezpieczeństwo zapisanych stanów przy zmianach.**

- Ustawienia interfejsu nie są sprawdzane przy wczytaniu. Po usunięciu karty zapisane `ribbonTab:'calc'` pokaże zły panel.
- Plik z nowszej wersji, który zawiera nieznany rodzaj obliczenia, wywraca starszą wersję. Filtr działa przy localStorage, ale nie przy otwieraniu pliku.
- Wersja webowa wdraża się na bieżąco, więc użytkownicy przechodzą między wersjami bez wyboru.
- **Potrzebne:** klucz `pikieta.ui.v2` z jednorazowym przeniesieniem z v1, sprawdzanie wartości i filtr obliczeń przy otwieraniu pliku.

**M9. Pusty projekt niczego nie uczy.** W pustym projekcie przydałby się pasek trzech kroków z przyciskami odhaczanymi w miarę postępu: „1 Wczytaj punkty → 2 Wybierz obliczenie → 3 Raport”.

### Drobne

- **Czasowniki wczytywania:** są trzy (Otwórz, Importuj, Wczytaj), a przycisk „Punkty” eksportuje GML. Lepiej mieć jedno „Wczytaj…” z wyborem rodzaju.
- **Błędy importu** pokazuje tylko znikający komunikat z dwoma pierwszymi wierszami.
- **„Dokładność wyników”** w rzeczywistości oznacza liczbę miejsc po przecinku.
- **Nieudany podkład** po cichu wraca do ortofotomapy.
- **Historia** dzieli się na otwarte i zamknięte, a to ukryte pojęcie. Lepsza jest jedna lista według daty ze znacznikiem aktywnego obliczenia.
- **Lokalny dziennik diagnostyczny** (bez telemetrii) dla błędów usług WMS i obliczeń.

### Sprzeczności i ich rozwiązania

| Sprzeczność | Rozwiązanie |
| --- | --- |
| Jedno wejście a szybkość doświadczonego | Jedno okno wyboru; dla doświadczonych wyszukiwanie, Ctrl+K, ostatnie zadania albo wstążka jako skrót do tego samego okna |
| Wszystkie parametry a czytelność dla gisowca | Bez trybu dla początkujących: fachowa nazwa i zdanie wyjaśnienia, jednostki, grupy pokazujące bieżące wartości |
| Automatyczne ostrzeżenie a rygor statystyczny | Test „bez jednego punktu” w Rust, komunikat „sprawdź”, bez automatycznego wyłączania, flaga „użyj w dopasowaniu” |
| Gęstość na komputerze a telefon | Te same komponenty, inny układ zależnie od szerokości; na telefonie mapa na pierwszym miejscu, zadanie na pełny ekran |
| Edytowalne obliczenie a stały raport | Wpisy raportu powiązane z obliczeniem; wykrywanie nieaktualności albo zamrożenie przy wydruku |
| Kolejność warstw a renderer GPU | Stałe pasma, kolejność wewnątrz pasma, bez przeplatania WMS z GML |

## Cele i poza zakresem

* **W zakresie:**
  - jedno okno wyboru zadania z opisami i wyszukiwaniem;
  - jedna historia obliczeń;
  - jeden słownik pojęć;
  - panel warstw ze stałymi pasmami i kolejnością wewnątrz pasma;
  - podgląd importu z wykrywaniem X/Y;
  - rozdzielona akcja zmiany układu;
  - ostrzeżenia liczone w Rust według jednej tabeli progów, w tym naprawa testu odstających w transformacji;
  - wyjaśnienia pól w oknach zadań;
  - „Zapisz wynik” do projektu i raportu;
  - powiązanie raportu i punktów z obliczeniem;
  - wklejanie obserwacji w tachimetrii;
  - rysunek w raporcie (SVG ze szkicu);
  - pasek startowy w pustym projekcie;
  - bezpieczna migracja zapisanych ustawień;
  - test ścieżki użytkownika i protokół testu z ludźmi.
* **Poza zakresem:**
  - eksport do nowych formatów SHP, GPKG, CAD i GML (osobny wątek w PLAN.md; istniejące eksporty TXT, HTML i GML zostają);
  - upraszczanie obliczeń przez ukrywanie parametrów;
  - tryb dla początkujących;
  - przeplatanie WMS z GML;
  - dodawanie usług po adresie URL;
  - na razie lokalizacja i zdjęcia na telefonie, ale wąski układ nie może się zepsuć.

## Rozważone alternatywy

- **Tryb dla początkujących (osobny, uproszczony interfejs).** Odrzucony przez autora: program ma ostrzegać i wyjaśniać, a nie upraszczać. Dwa interfejsy to też podwójne utrzymanie.
- **Ukrywanie parametrów obliczeń.** Odrzucone przez autora z tego samego powodu: gisowiec dostałby wynik, którego nie umie ocenić.
- **Eksport w tym samym temacie.** Odrzucony przez autora: to nowa funkcja, a nie obsługa. Przeniesiony do PLAN.md jako wątek na później.
- **Telefon na równi z komputerem.** Odrzucony przez autora: telefon jest na drugim miejscu, a w terenie liczą się podgląd, lokalizacja i zdjęcie.
- **Zdjęcie mapy z podkładem jako rysunek raportu.** Odrzucone w przeglądzie: usługi bez CORS dają pusty albo zablokowany obraz.
- **Dowolna kolejność warstw WMS i GML.** Odrzucona w przeglądzie: wymaga przebudowy renderera WebGL i grozi spadkiem płynności przy dużych plikach.
- **Do porównania przy projektowaniu:** sama wstążka, sam panel boczny, paleta poleceń, panele dokowane w stylu QGIS.

## Kluczowe decyzje i kolejne kroki

**Postanowione w rozmowie:**

- Użytkownicy to geodeci w terenie i w biurze oraz gisowcy.
- Wszystko w interfejsie można zmienić, a rygor obliczeń zostaje.
- Program ostrzega i wyjaśnia, nie upraszcza.
- Komputer jest pierwszy, telefon drugi.
- Eksport jest osobnym wątkiem.
- Zadania testowe to tachimetria do raportu z rysunkiem oraz transformacja.

- Wstążka zostaje jako skrót do wspólnego okna wyboru zadania.
- Rysunek w raporcie wchodzi w zakres.
- Test punktów odstających w transformacji jest naprawiony (B1).

**Do rozstrzygnięcia przed wdrożeniem:**

1. Postać rysunku w raporcie: elementy (północ, podziałka, opisy), skala i format wydruku.
2. Jak obserwacje tachimetryczne trafiają do testu (wklejanie, plik).
3. Tabela progów i czy ostrzeżenia blokują dodanie punktów.
4. Warstwy w projekcie czy w urządzeniu; pasma.
5. Czy punkty i wpisy raportu mają odwołanie do obliczenia.
6. Telefon w tej iteracji czy poza nią.
7. Protokół testu użyteczności.

**Proponowana kolejność (najmniejszy zakres, który spełnia kryterium):**

1. Test ścieżki w Playwright na wersji webowej:
   - pliki testowe: TXT z punktami, wariant z zamienionymi X/Y, blok obserwacji, zestaw do transformacji z jednym punktem przesuniętym o 0,30 m;
   - pierwsze uruchomienie na obecnym interfejsie jako punkt odniesienia.
2. Bezpieczna migracja zapisanych stanów (M8).
3. Wspólny pasek ostrzeżeń i tabela progów (M5); test odstających w transformacji już naprawiony (B1).
4. Podgląd importu z wykrywaniem X/Y i rozdzielona zmiana układu (B4).
5. Jedno okno wyboru, jedna historia, słownik i pasek startowy (B6, M1, M9).
6. „Zapisz wynik”, domyślny przedrostek, powody wyłączenia przycisków, powiązania `calcId` i `src` (M3, M4).
7. Wklejanie obserwacji w tachimetrii (B3).
8. Rysunek w raporcie jako SVG ze szkicu (B2).
9. Panel warstw z pasmami i migracją (M2).
10. Na później: lokalizacja i zdjęcia na telefonie, usługi po URL, przeplatanie warstw.

**Sprawdzenie kryterium sukcesu:**

- **Test skryptowy (Playwright, w CI):**
  - czysta przeglądarka, ekrany 1366×768 i 390×844;
  - lokatory tylko po widocznych polskich nazwach;
  - scenariusz (a): import, potem tachimetria, wklejenie obserwacji, zapis wyniku, pole z nowych punktów i raport z `<svg>`;
  - scenariusz (b): Helmert, w którym ostrzeżenie wskazuje dokładnie podstawiony punkt, a po jego wyłączeniu m0 spada;
  - warunki strukturalne: jeden przycisk startu obliczenia, brak tej samej nazwy dla różnych akcji, żaden krok nie zależy od dymku.
- **Lista pojęć:** każde pojęcie z listy ma wyjaśnienie. Lista: stanowisko, nawiązanie, stała orientacji, zenit, cc, grady, punkty dostosowania, pierwotny/wtórny, Hausbrandt, m₀, nadliczbowe, redukcja.
- **Test z ludźmi:**
  - 5–6 osób, w tym co najmniej 3 gisowców bez przygotowania geodezyjnego;
  - karta zadania bez słownictwa interfejsu, bez pomocy, 20 minut na zadanie;
  - zaliczenie: co najmniej 4 z 5 osób kończą oba zadania, nikt nie zapisuje wyniku z odstającym punktem bez zauważenia ostrzeżenia, każdy sam znajduje historię i zmienia kolejność warstwy;
  - najpierw test na obecnym interfejsie;
  - jedna sesja z doświadczonym użytkownikiem: nic nie może mu zniknąć, a częste ścieżki mogą się wydłużyć najwyżej o jedno kliknięcie.
