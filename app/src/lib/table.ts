/** Kolumna tabeli danych w stylu tabeli atrybutów QGIS. */
export interface Column<R> {
  key: string
  label: string
  /** Początkowa szerokość w pikselach; użytkownik może ją zmienić przeciągając nagłówek. */
  width: number
  align?: 'left' | 'right'
  /** Rodzaj edycji; brak oznacza kolumnę tylko do odczytu. `point` pozwala wskazać punkt na mapie,
   *  `check` to pole wyboru: `value` zwraca '1' dla zaznaczonego, `set` dostaje '1' albo ''. */
  edit?: 'text' | 'number' | 'point' | 'check'
  /** Ukrywa pole edycji w danym wierszu (np. pole wyboru w pustym wierszu na końcu). */
  hidden?: (row: R) => boolean
  value: (row: R) => string
  set?: (row: R, value: string) => void
  placeholder?: (row: R) => string
  tone?: (row: R) => 'error' | 'muted' | 'ok' | undefined
}
