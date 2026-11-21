// Import i eksport plików tekstowych.
import { parsePoints, writePoints, type Pt } from './core'
import { addPoints, downloadText, notify, project, reportText, ui } from './project.svelte'

/** Okno wyboru pliku. Anulowanie zostawia obietnicę nierozstrzygniętą. */
export function pickFile(accept: string): Promise<File> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = accept
    input.onchange = () => input.files?.[0] && resolve(input.files[0])
    input.click()
  })
}

/** Pliki z programów geodezyjnych bywają w Windows-1250, nie w UTF-8. */
async function readText(file: File): Promise<string> {
  const bytes = await file.arrayBuffer()
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    return new TextDecoder('windows-1250').decode(bytes)
  }
}

/** Wczytuje wykaz współrzędnych do projektu; zwraca wczytane punkty. */
export async function importPoints(): Promise<Pt[]> {
  const file = await pickFile('.txt,.csv,.xyz,text/plain')
  const parsed = parsePoints(await readText(file), ui.yx)
  const replaced = addPoints(parsed.points)
  notify(
    `${file.name}: wczytano ${parsed.points.length} pkt` +
      (replaced ? `, zastąpiono ${replaced}` : '') +
      (parsed.errors.length ? `, pominięto ${parsed.errors.length} wierszy (${parsed.errors.slice(0, 2).join('; ')})` : '') +
      '.',
    parsed.points.length === 0 ? 'error' : 'info',
  )
  return parsed.points
}

export function exportPoints() {
  if (project.points.length === 0) return notify('Brak punktów do eksportu.', 'error')
  downloadText('punkty.txt', writePoints(project.points, ui.yx, 2))
}

export function exportReport(crsLabel: string) {
  if (project.report.length === 0) return notify('Raport jest pusty.', 'error')
  downloadText('raport.txt', reportText(crsLabel))
}
