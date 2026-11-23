// Katalog usług WMS (GUGiK i instytucje publikujące przez geoportal.gov.pl): podkłady i nakładki.
// Nazwy warstw, obsługiwane układy i nagłówki CORS sprawdzone w GetCapabilities usług (23.11.2026).

export interface WmsService {
  id: string
  label: string
  group: string
  url: string
  /** Warstwy WMS od spodniej; `*` oznacza wszystkie warstwy z GetCapabilities. */
  layers: string
  format?: 'image/png' | 'image/jpeg'
  /** Układy obsługiwane przez usługę; brak oznacza wszystkie strefy PL-2000 i PL-1992. */
  crs?: string[]
  /**
   * Usługa zezwala na odczyt obrazu z innej domeny (CORS). Bez tego obrazu nie da się przeliczyć
   * do innego układu, więc usługa działa tylko w układach, które sama obsługuje.
   */
  cors: boolean
  opacity?: number
  attribution: string
}

const GEO = 'https://mapy.geoportal.gov.pl/wss'
// Usługi integracyjne przekierowują do serwerów powiatów, które zwykle nie wysyłają nagłówków CORS;
// obsługują wszystkie strefy, więc obraz nie wymaga przeliczania.
const INT = 'https://integracja.gugik.gov.pl/cgi-bin'
const GUGIK = '© GUGiK (geoportal.gov.pl)'
const PL1992 = ['EPSG:2180']

/** Podkłady: zawsze jeden pod treścią mapy. */
export const BASEMAPS: WmsService[] = [
  {
    id: 'orto',
    group: 'Ortofotomapa',
    label: 'Ortofotomapa standardowa',
    url: `${GEO}/service/PZGIK/ORTO/WMS/StandardResolution`,
    layers: 'Raster',
    format: 'image/jpeg',
    cors: true,
    attribution: `Ortofotomapa ${GUGIK}`,
  },
  {
    id: 'orto-hd',
    group: 'Ortofotomapa',
    label: 'Ortofotomapa wysokiej rozdzielczości',
    url: `${GEO}/service/PZGIK/ORTO/WMS/HighResolution`,
    layers: 'Raster',
    format: 'image/jpeg',
    crs: PL1992,
    cors: true,
    attribution: `Ortofotomapa ${GUGIK}`,
  },
  {
    id: 'orto-true',
    group: 'Ortofotomapa',
    label: 'Ortofotomapa prawdziwa',
    url: `${GEO}/service/PZGIK/ORTO/WMS/TrueOrtho`,
    layers: 'PrawdziwaOrtofotomapa',
    format: 'image/jpeg',
    crs: PL1992,
    cors: true,
    attribution: `Ortofotomapa prawdziwa ${GUGIK}`,
  },
  {
    id: 'bdot10k',
    group: 'Mapy',
    label: 'BDOT10k',
    url: `${GEO}/service/pub/guest/kompozycja_BDOT10k_WMS/MapServer/WMSServer`,
    layers: '*',
    cors: true,
    attribution: `BDOT10k ${GUGIK}`,
  },
  {
    id: 'bdoo',
    group: 'Mapy',
    label: 'BDOO (mapa ogólnogeograficzna)',
    url: `${GEO}/service/PZGIK/BDOO/WMS/aktualne`,
    layers: '*',
    crs: PL1992,
    cors: true,
    attribution: `BDOO ${GUGIK}`,
  },
  {
    id: 'topo',
    group: 'Mapy',
    label: 'Mapa topograficzna (skany)',
    url: `${GEO}/service/img/guest/TOPO/MapServer/WMSServer`,
    layers: 'Raster',
    format: 'image/jpeg',
    cors: true,
    attribution: `Mapa topograficzna ${GUGIK}`,
  },
  {
    id: 'osm',
    group: 'Mapy',
    label: 'OpenStreetMap',
    url: `${GEO}/ext/OSM/BaseMapWMS`,
    layers: 'MS',
    cors: false,
    attribution: '© autorzy OpenStreetMap',
  },
  {
    id: 'cien',
    group: 'Rzeźba terenu',
    label: 'Cieniowanie NMT',
    url: `${GEO}/service/PZGIK/NMT/GRID1/WMS/ShadedRelief`,
    layers: 'Raster',
    format: 'image/jpeg',
    cors: true,
    attribution: `NMT ${GUGIK}`,
  },
  {
    id: 'hipso',
    group: 'Rzeźba terenu',
    label: 'Hipsometria NMT',
    url: `${GEO}/service/PZGIK/NMT/GRID1/WMS/Hypsometry`,
    layers: 'Raster',
    format: 'image/jpeg',
    cors: true,
    attribution: `NMT ${GUGIK}`,
  },
]

/** Nakładki: dowolnie wiele nad podkładem, pod warstwami GML i punktami. */
export const OVERLAYS: WmsService[] = [
  {
    id: 'kieg',
    group: 'Ewidencja gruntów',
    label: 'Działki, numery, budynki (KIEG)',
    url: `${INT}/KrajowaIntegracjaEwidencjiGruntow`,
    layers: 'dzialki,numery_dzialek,budynki',
    cors: false,
    attribution: `KIEG ${GUGIK}`,
  },
  {
    id: 'kieg-uzytki',
    group: 'Ewidencja gruntów',
    label: 'Użytki i kontury klasyfikacyjne (KIEG)',
    url: `${INT}/KrajowaIntegracjaEwidencjiGruntow`,
    layers: 'uzytki,kontury',
    cors: false,
    attribution: `KIEG ${GUGIK}`,
  },
  {
    id: 'kieg-obreby',
    group: 'Ewidencja gruntów',
    label: 'Granice obrębów (KIEG)',
    url: `${INT}/KrajowaIntegracjaEwidencjiGruntow`,
    layers: 'obreby',
    cors: false,
    attribution: `KIEG ${GUGIK}`,
  },
  {
    id: 'kiut',
    group: 'Uzbrojenie terenu',
    label: 'Sieci uzbrojenia terenu (KIUT)',
    url: `${INT}/KrajowaIntegracjaUzbrojeniaTerenu`,
    layers:
      'przewod_niezidentyfikowany,przewod_specjalny,przewod_telekomunikacyjny,przewod_cieplowniczy,przewod_gazowy,przewod_elektroenergetyczny,przewod_kanalizacyjny,przewod_wodociagowy,przewod_urzadzenia',
    cors: false,
    attribution: `KIUT ${GUGIK}`,
  },
  {
    id: 'kibdot',
    group: 'Obiekty topograficzne',
    label: 'Obiekty topograficzne BDOT500 (KIBDOT)',
    url: `${INT}/KrajowaIntegracjaBazDanychObiektowTopograficznych`,
    layers: 'bdot',
    cors: false,
    attribution: `KIBDOT ${GUGIK}`,
  },
  {
    id: 'osnowa-szcz',
    group: 'Osnowa geodezyjna',
    label: 'Szczegółowa osnowa geodezyjna (KISOG)',
    url: `${INT}/KrajowaIntegracjaSzczegolowychOsnowGeodezyjnych`,
    layers: 'osnowa-XY,osnowa-H,osnowa-XYH,osnowa-',
    cors: false,
    attribution: `KISOG ${GUGIK}`,
  },
  {
    id: 'osnowa-poz',
    group: 'Osnowa geodezyjna',
    label: 'Podstawowa osnowa pozioma',
    url: `${GEO}/service/PZGIK/PodstawowaOsnowaGeodezyjnaPozioma`,
    layers: 'Fundamentalna_pozioma,Bazowa_pozioma,Fudnamentalna_pozioma_etykiety,Bazowa_pozioma_etykiety',
    cors: true,
    attribution: `Osnowa ${GUGIK}`,
  },
  {
    id: 'asg',
    group: 'Osnowa geodezyjna',
    label: 'Stacje ASG-EUPOS',
    url: `${GEO}/service/pub/guest/G2_ASG_EUPOS_WMS/MapServer/WMSServer`,
    layers: '0,1',
    cors: true,
    attribution: `ASG-EUPOS ${GUGIK}`,
  },
  {
    id: 'prg',
    group: 'Granice i adresy',
    label: 'Granice administracyjne (PRG)',
    url: `${GEO}/service/PZGIK/PRG/WMS/AdministrativeBoundaries`,
    layers:
      'A06_Granice_obrebow_ewidencyjnych,A05_Granice_jednostek_ewidencyjnych,A04_Granice_miast,A03_Granice_gmin,A02_Granice_powiatow,A01_Granice_wojewodztw,A00_Granice_panstwa',
    cors: true,
    attribution: `PRG ${GUGIK}`,
  },
  {
    id: 'adresy',
    group: 'Granice i adresy',
    label: 'Ulice i punkty adresowe (KIN)',
    url: `${GEO}/ext/KrajowaIntegracjaNumeracjiAdresowej`,
    layers: 'A08_Ulice,A07_Punkty_adresowe',
    cors: true,
    attribution: `KIN ${GUGIK}`,
  },
  {
    id: 'kimp',
    group: 'Planowanie przestrzenne',
    label: 'Miejscowe plany (KIMP)',
    url: `${GEO}/ext/KrajowaIntegracjaMiejscowychPlanowZagospodarowaniaPrzestrzennego`,
    layers: 'raster,wektor-str,wektor-pow,wektor-lin,wektor-lzb,wektor-pkt,plany_granice',
    crs: PL1992,
    cors: true,
    opacity: 0.75,
    attribution: `KIMP ${GUGIK}`,
  },
  {
    id: 'gdos',
    group: 'Środowisko',
    label: 'Formy ochrony przyrody (GDOŚ)',
    url: 'https://sdi.gdos.gov.pl/wms',
    layers:
      'GDOS:ObszaryChronionegoKrajobrazu,GDOS:ParkiKrajobrazowe,GDOS:ObszarySpecjalnejOchrony,GDOS:SpecjalneObszaryOchrony,GDOS:ParkiNarodowe,GDOS:Rezerwaty,GDOS:ZespolyPrzyrodniczoKrajobrazowe,GDOS:UzytkiEkologiczne,GDOS:StanowiskaDokumentacyjne,GDOS:PomnikiPrzyrody',
    cors: true,
    opacity: 0.7,
    attribution: '© GDOŚ',
  },
  {
    id: 'lpis',
    group: 'Środowisko',
    label: 'Działki referencyjne LPIS (ARiMR)',
    url: `${GEO}/ext/arimr_lpis`,
    layers: '02,04,06,08,10,12,14,16,18,20,22,24,26,28,30,32',
    cors: false,
    attribution: '© ARiMR',
  },
  {
    id: 'rozn-wys',
    group: 'Układy i siatki',
    label: 'Różnice wysokości Kronsztad86 – EVRF2007',
    url: `${GEO}/service/RozniceWysokosci_Kronsztad86_EVRF2007`,
    layers: 'raster,punkty',
    cors: false,
    opacity: 0.6,
    attribution: GUGIK,
  },
  {
    id: 'sekcje2000',
    group: 'Układy i siatki',
    label: 'Sekcje mapy w układzie 2000',
    url: `${GEO}/service/PUWG2000`,
    layers: 'sekcje10000,sekcje5000,sekcje2000,sekcje1000,sekcje500',
    crs: ['EPSG:2178', 'EPSG:2180'],
    cors: false,
    attribution: GUGIK,
  },
  {
    id: 'sekcje1992',
    group: 'Układy i siatki',
    label: 'Sekcje mapy w układzie 1992',
    url: `${GEO}/service/PUWG1992`,
    layers: 'sekcje10000,sekcje5000',
    crs: ['EPSG:2178', 'EPSG:2180'],
    cors: false,
    attribution: GUGIK,
  },
  {
    id: 'sekcje1965',
    group: 'Układy i siatki',
    label: 'Sekcje mapy w układzie 1965',
    url: `${GEO}/service/PUWG1965`,
    layers: 'sekcje10000,sekcje5000,sekcje2000,sekcje1000,sekcje500',
    crs: ['EPSG:2178', 'EPSG:2180'],
    cors: false,
    attribution: GUGIK,
  },
  {
    id: 'strefy2000',
    group: 'Układy i siatki',
    label: 'Strefy układu 2000',
    url: `${GEO}/service/strefy2000`,
    layers: 'strefy2000',
    crs: ['EPSG:2178', 'EPSG:2180'],
    cors: false,
    attribution: GUGIK,
  },
]

export const DEFAULT_BASEMAP = 'orto'

export const findService = (id: string) => BASEMAPS.find((s) => s.id === id) ?? OVERLAYS.find((s) => s.id === id)

/**
 * W jakim układzie pobierać obraz: w układzie mapy, gdy usługa go obsługuje; w innym (z przeliczeniem
 * obrazu), gdy usługa zezwala na CORS; `null`, gdy w tym układzie mapy usługi nie da się pokazać.
 */
export function requestCrs(s: WmsService, mapCrs: string): string | null {
  if (!s.crs || s.crs.includes(mapCrs)) return mapCrs
  return s.cors ? s.crs[0] : null
}

/** Grupy w kolejności katalogu. */
export function grouped(list: WmsService[]): [string, WmsService[]][] {
  const out = new Map<string, WmsService[]>()
  for (const s of list) out.set(s.group, [...(out.get(s.group) ?? []), s])
  return [...out]
}
