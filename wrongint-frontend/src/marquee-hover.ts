// Smoothly ramp a marquee's speed instead of the hard play/pause that
// vue3-marquee's `pause-on-hover` does. The marquee scrolls via CSS animations,
// which are real Web Animations — so we ease their `playbackRate` toward a
// target (0 = stopped, 1 = full speed) over a short window with rAF. Hovering
// decelerates quickly to a stop; leaving accelerates back.

const pending = new WeakMap<Element, number>()

function ramp(el: Element, target: number, ms = 280): void {
  const anims = (el as Element & {
    getAnimations?: (opts?: { subtree?: boolean }) => Animation[]
  }).getAnimations?.({ subtree: true })
  if (!anims || !anims.length) return

  const from = anims[0].playbackRate ?? 1
  const start = performance.now()

  const queued = pending.get(el)
  if (queued) cancelAnimationFrame(queued)

  const step = (now: number): void => {
    // Ease-out so the change is quick up front then settles.
    const t = Math.min(1, (now - start) / ms)
    const eased = 1 - (1 - t) * (1 - t)
    const rate = from + (target - from) * eased
    for (const a of anims) a.playbackRate = rate
    if (t < 1) pending.set(el, requestAnimationFrame(step))
  }
  pending.set(el, requestAnimationFrame(step))
}

// Bind to a marquee root's native @mouseenter / @mouseleave. currentTarget is
// the marquee element; its subtree holds the scrolling (cloned) tracks.
export function slow(e: Event): void {
  if (e.currentTarget instanceof Element) ramp(e.currentTarget, 0, 280)
}

export function resume(e: Event): void {
  // Accelerate back more gently than the decel.
  if (e.currentTarget instanceof Element) ramp(e.currentTarget, 1, 900)
}
