// Dostępność: obsługa klawiatury w grupach przycisków (karty, przełączniki) według wzorców WAI-ARIA.

/**
 * Akcja dla elementu z `role="tablist"` albo `role="radiogroup"`: strzałki, Home i End przenoszą
 * fokus między kartami (opcjami) i od razu je wybierają. Karty muszą mieć `tabindex` 0 (wybrana)
 * albo -1 (pozostałe), żeby Tab wchodził do grupy raz.
 */
export function rovingKeys(node: HTMLElement) {
  const items = () =>
    [...node.querySelectorAll<HTMLElement>('[role="tab"], [role="radio"]')].filter((el) => !el.hasAttribute('disabled'))
  function onKey(e: KeyboardEvent) {
    const list = items()
    const i = list.indexOf(document.activeElement as HTMLElement)
    if (i < 0) return
    const next = {
      ArrowRight: i + 1,
      ArrowDown: i + 1,
      ArrowLeft: i - 1,
      ArrowUp: i - 1,
      Home: 0,
      End: list.length - 1,
    }[e.key]
    if (next === undefined) return
    e.preventDefault()
    const el = list[(next + list.length) % list.length]
    el.focus()
    el.click()
  }
  node.addEventListener('keydown', onKey)
  return { destroy: () => node.removeEventListener('keydown', onKey) }
}
