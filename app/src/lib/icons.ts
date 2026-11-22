// Ikony liniowe na siatce 24×24, każda jako jedna ścieżka SVG.

const circle = (cx: number, cy: number, r: number) =>
  `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`

export const ICONS = {
  menu: 'M4 6h16M4 12h16M4 18h16',
  plus: 'M12 5v14M5 12h14',
  x: 'M6 6l12 12M18 6 6 18',
  search: `${circle(11, 11, 6)}M20 20l-4.5-4.5`,
  upload: 'M12 15V4M7 9l5-5 5 5M5 20h14',
  download: 'M12 4v11M7 10l5 5 5-5M5 20h14',
  file: 'M7 3h7l4 4v14H7zM14 3v4h4',
  report: 'M7 3h7l4 4v14H7zM14 3v4h4M10 12h5M10 16h5',
  save: 'M5 4h11l3 3v13H5zM8 4v5h7V4M8 20v-6h8v6',
  folder: 'M3 6.5V5h6l2 2h10v12H3zM3 9h18',
  printer: 'M7 9V4h10v5M7 17H4v-8h16v8h-3M7 14h10v6H7z',
  grip: 'M9 6h.01M9 12h.01M9 18h.01M15 6h.01M15 12h.01M15 18h.01',
  trash: 'M5 7h14M9 7V4h6v3M7 7l1 13h8l1-13',
  pencil: 'M4 20l4-1L19 8l-3-3L5 16zM14 7l3 3',
  map: 'M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2zM9 4v14M15 6v14',
  split: 'M3 5h18v14H3zM12 5v14',
  panel: 'M3 5h18v14H3zM3 10h18M9 10v9',
  table: 'M3 5h18v14H3zM3 10h18M3 15h18M9 5v14',
  sidebar: 'M3 5h18v14H3zM9 5v14',
  layers: 'M12 4l9 5-9 5-9-5zM3 14l9 5 9-5',
  globe: `${circle(12, 12, 9)}M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18`,
  pointAdd: `${circle(10, 10, 3)}M10 3v2M10 15v2M3 10h2M15 10h2M18 15v6M15 18h6`,
  fit: 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5',
  selectAll: 'M4 4h16v16H4zM8 12l3 3 5-6',
  invert: `${circle(12, 12, 8)}M12 4v16M12 8h4M12 12h6M12 16h4`,
  deselect: 'M4 4h16v16H4zM9 9l6 6M15 9l-6 6',
  filter: 'M4 5h16l-6 8v6l-4-2v-4z',
  chevronUp: 'M6 15l6-6 6 6',
  chevronDown: 'M6 9l6 6 6-6',
  clock: `${circle(12, 12, 9)}M12 7v5l3 2`,
  azimuth: `${circle(6, 18, 2)}${circle(18, 6, 2)}M7.5 16.5l9-9M5 9V3M3 5l2-2 2 2`,
  polar: `${circle(6, 18, 2)}M8 18h13M7.5 16.5 18 6M12 18a6 6 0 0 0-1.8-4.2`,
  offset: `M3 19 21 7${circle(16, 18, 2)}M12.5 12.7 15 16.4`,
  fromOffset: 'M3 19 21 7M12.5 12.7 15 16.4M16 15.5v5M13.5 18h5',
  distInt: `${circle(9, 12, 5)}${circle(15, 12, 5)}`,
  lineInt: `M4 4l16 16M20 4 4 20${circle(12, 12, 2)}`,
  area: 'M5 18 7 6l9-2 4 9-6 7z',
  traverse: `${circle(4, 18, 1.6)}${circle(10, 9, 1.6)}${circle(16, 15, 1.6)}${circle(20, 5, 1.6)}M5 16.6 9 10.4M11.2 10.2l3.6 3.6M16.8 13.5 19.4 6.5`,
  point: circle(12, 12, 3),
  list: 'M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01',
} satisfies Record<string, string>

export type IconName = keyof typeof ICONS
