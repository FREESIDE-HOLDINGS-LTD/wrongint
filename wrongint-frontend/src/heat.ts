// Per-source "argument heat": map an index onto a green->red spectrum,
// scaled to the min/max of whatever source the post belongs to.

export interface Range {
  min: number
  max: number
}

// Smallest range covering all the given values, or null when there are none.
export function rangeOf(values: Iterable<number | null | undefined>): Range | null {
  let min = Infinity
  let max = -Infinity
  for (const v of values) {
    if (v == null) continue
    min = Math.min(min, v)
    max = Math.max(max, v)
  }
  return min <= max ? { min, max } : null
}

// 0 (range min) .. 1 (range max). null when the value or range is missing.
export function heat(value: number | null | undefined, range: Range | null): number | null {
  if (value == null || !range) return null
  if (range.max === range.min) return 0
  return (value - range.min) / (range.max - range.min)
}

// Green (cool) through red (hot); muted grey when there is no heat.
export function accent(t: number | null): string {
  if (t == null) return '#5a6470'
  return `hsl(${120 - 120 * t}, 100%, 55%)`
}

// Hottest third of a range pulses.
export function isHot(t: number | null): boolean {
  return t != null && t >= 0.66
}

// Face graded the same way as the color: sleepy (cool) through angry (hot).
// Neutral face when there is no heat.
export const FACES = ['😴', '🥱', '😑', '😐', '😤', '😠', '😡']
export function emoji(t: number | null): string {
  if (t == null) return '😶'
  const i = Math.min(FACES.length - 1, Math.floor(t * FACES.length))
  return FACES[i]
}
