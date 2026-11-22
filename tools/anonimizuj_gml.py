#!/usr/bin/env python3
"""Anonimizator plików GML (EGiB, GESUT, BDOT500) do testów i zgłaszania błędów.

Zamienia dane osobowe i identyfikujące na stałe zastępniki, a resztę pliku zostawia bajt w bajt:
geometria, identyfikatory obiektów (gml:id, idDzialki itp.), atrybuty i układ pliku zostają.
Ta sama wartość dostaje zawsze ten sam zastępnik, więc powiązania (np. ta sama osoba
w kilku udziałach) się zachowują.

Zamieniane są:
- osoby fizyczne: imiona, nazwiska, imiona rodziców, PESEL, informacja o śmierci;
- instytucje, podmioty grupowe, wspólnoty: nazwy i REGON;
- adresy podmiotów (zameldowania, pobytu, siedziby): miejscowość, kod, ulica, numery;
- księgi wieczyste i dokumenty: numer KW, dokument własności, sygnatury, oznaczenia, opisy,
  twórcy dokumentów i operatów;
- teksty swobodne: dodatkowe informacje, opisy zmian, informacja dodatkowa (GESUT, BDOT500);
- GESUT: władający siecią.

Adresy nieruchomości (budynków, działek) i współrzędne nie są danymi osobowymi i zostają.
Komentarze XML są usuwane razem z treścią (mogą zawierać nazwiska wykonawców).

Użycie:
    python anonimizuj_gml.py plik.gml                 # zapisuje plik_anon.gml
    python anonimizuj_gml.py plik.gml -o wynik.gml
    python anonimizuj_gml.py plik.gml --mapa mapa.csv  # słownik zamian (zostaje u Ciebie!)

Wymaga tylko Pythona 3.8+ (bez dodatkowych pakietów).
"""

import argparse
import csv
import re
import sys
from collections import Counter
from pathlib import Path

# Pola zamieniane zawsze, w każdej klasie: nazwa elementu → rodzaj zastępnika.
FIELDS = {
    # osoby fizyczne
    "pierwszeImie": "imie",
    "drugieImie": "imie",
    "imieOjca": "imie",
    "imieMatki": "imie",
    "pierwszyCzlonNazwiska": "nazwisko",
    "drugiCzlonNazwiska": "nazwisko",
    "pesel": "pesel",
    "informacjaOSmierci": "tekst",
    # instytucje, podmioty grupowe, wspólnoty gruntowe
    "nazwaPelna": "nazwa",
    "nazwaSkrocona": "nazwa",
    "regon": "regon",
    # księgi wieczyste, dokumenty, operaty
    "numerKW": "kw",
    "dokumentWlasnosci": "tekst",
    "sygnaturaDokumentu": "sygnatura",
    "oznKancelaryjneDokumentu": "sygnatura",
    "opisDokumentu": "tekst",
    "nazwaTworcyDokumentu": "nazwa",
    "nazwaTworcy": "nazwa",
    "opisOperatu": "tekst",
    "opisZmiany": "tekst",
    "tytul": "tekst",
    # teksty swobodne
    "dodatkoweInformacje": "tekst",
    "informacjaDodatkowa": "tekst",
    # GESUT
    "wladajacy": "nazwa",
}

# Pola zamieniane tylko wewnątrz wskazanych klas (te same nazwy pól występują też w adresach
# nieruchomości, które zostają).
SCOPED = {
    "EGB_AdresPodmiotu": {
        "miejscowosc": "miejscowosc",
        "kodPocztowy": "kod",
        "ulica": "ulica",
        "numerPorzadkowy": "numer",
        "numerLokalu": "numer",
    },
    "EGB_WspolnotaGruntowa": {"nazwa": "nazwa"},
}

NAME = r"(?:[\w.-]+:)?"


class Replacer:
    """Stałe zastępniki: ta sama wartość danego rodzaju → ten sam zastępnik."""

    def __init__(self):
        self.seen = {}
        self.counter = Counter()
        self.stats = Counter()

    def __call__(self, kind, field, value):
        if value.strip() == "":
            return value
        self.stats[field] += 1
        key = (kind, value.strip())
        if key not in self.seen:
            self.counter[kind] += 1
            self.seen[key] = self.make(kind, self.counter[kind], value.strip())
        return self.seen[key]

    @staticmethod
    def make(kind, n, value):
        if kind == "pesel":
            return f"{n:011d}"
        if kind == "regon":
            # Zachowuje długość (9 albo 14 cyfr).
            return f"{n:0{max(len(value), 9)}d}"
        if kind == "kw":
            return f"XX1X/{n:08d}/0"
        if kind == "kod":
            return "00-000"
        if kind == "numer":
            return str(n)
        if kind == "tekst":
            return f"TEKST_{n}"
        return f"{kind.upper()}_{n}"


def anonymize(text, rep):
    # Komentarze mogą zawierać nazwiska wykonawców; zostaje pusty komentarz.
    text = re.sub(r"<!--.*?-->", "<!-- -->", text, flags=re.S)

    def field_sub(fields):
        names = "|".join(sorted(fields, key=len, reverse=True))
        pattern = re.compile(rf"<({NAME}({names}))(\s[^<>]*)?>([^<]*)</\1>")

        def sub(m):
            field = m.group(2)
            value = rep(fields[field], field, m.group(4))
            # Zastępniki nie zawierają znaków specjalnych XML, więc nie trzeba ich kodować.
            return f"<{m.group(1)}{m.group(3) or ''}>{value}</{m.group(1)}>"

        return pattern, sub

    for cls, fields in SCOPED.items():
        block = re.compile(rf"(<({NAME}{cls})(?:\s[^<>]*)?>)(.*?)(</\2>)", re.S)
        pattern, sub = field_sub(fields)
        text = block.sub(lambda m: m.group(1) + pattern.sub(sub, m.group(3)) + m.group(4), text)

    pattern, sub = field_sub(FIELDS)
    return pattern.sub(sub, text)


def encoding_of(head):
    m = re.search(rb'encoding\s*=\s*["\']([\w.-]+)["\']', head[:200])
    return m.group(1).decode("ascii") if m else "utf-8"


def leftovers(text, placeholders):
    """Ciągi 11 cyfr w treści elementów (poza zastępnikami): możliwe PESEL-e, które zostały."""
    found = Counter()
    for m in re.finditer(r">([^<]*)<", text):
        for p in re.findall(r"(?<![\d.])\d{11}(?![\d.])", m.group(1)):
            if p not in placeholders:
                found[p] += 1
    return found


def main():
    ap = argparse.ArgumentParser(description="Anonimizacja danych osobowych w plikach GML (EGiB, GESUT, BDOT500).")
    ap.add_argument("plik", type=Path, help="plik GML do anonimizacji")
    ap.add_argument("-o", "--wynik", type=Path, help="plik wynikowy (domyślnie <nazwa>_anon.gml)")
    ap.add_argument("--mapa", type=Path, help="zapisz słownik zamian do CSV (zawiera dane osobowe, nie udostępniaj)")
    args = ap.parse_args()

    raw = args.plik.read_bytes()
    enc = encoding_of(raw)
    # Znacznik BOM zostaje w tekście jako znak U+FEFF i wraca przy zapisie.
    text = raw.decode(enc)

    rep = Replacer()
    out = anonymize(text, rep)

    target = args.wynik or args.plik.with_name(args.plik.stem + "_anon" + args.plik.suffix)
    if target.resolve() == args.plik.resolve():
        sys.exit("Plik wynikowy nie może nadpisać źródłowego.")
    # Zapis bajtów zachowuje końce linii i kodowanie oryginału.
    target.write_bytes(out.encode(enc))

    print(f"Zapisano: {target}")
    if rep.stats:
        print("Zamienione wartości:")
        for field, n in sorted(rep.stats.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"  {field:28} {n}")
    else:
        print("Nie znaleziono pól z danymi osobowymi.")

    rest = leftovers(out, set(rep.seen.values()))
    if rest:
        print(
            f"UWAGA: w treści zostało {sum(rest.values())} ciągów 11 cyfr (możliwe PESEL-e), np. "
            + ", ".join(list(rest)[:3])
            + ". Sprawdź plik przed udostępnieniem.",
            file=sys.stderr,
        )

    if args.mapa:
        with args.mapa.open("w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, delimiter=";")
            w.writerow(["rodzaj", "oryginał", "zastępnik"])
            for (kind, value), placeholder in sorted(rep.seen.items()):
                w.writerow([kind, value, placeholder])
        print(f"Słownik zamian: {args.mapa} (zawiera dane osobowe, zostaw go u siebie)")


if __name__ == "__main__":
    main()
