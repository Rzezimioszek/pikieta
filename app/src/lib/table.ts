/** Kolumna tabeli danych w stylu tabeli atrybutów QGIS. */
export interface Column<R> {
  key: string
  label: string
  /** Początkowa szerokość w pikselach; użytkownik może ją zmienić przeciągając nagłówek. */
  width: number
  align?: 'left' | 'right'
  /** Rodzaj edycji; brak oznacza kolumnę tylko do odczytu. `point` pozwala wskazać punkt na mapie. */
  edit?: 'text' | 'number' | 'point'
  value: (row: R) => string
  set?: (row: R, value: string) => void
  placeholder?: (row: R) => string
  tone?: (row: R) => 'error' | 'muted' | 'ok' | undefined
}
