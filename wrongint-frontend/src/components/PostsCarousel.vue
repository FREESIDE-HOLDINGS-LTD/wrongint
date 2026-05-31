<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { sourceLabel, type Post } from '../api'
import { rangeOf, heat, accent, isHot, type Range } from '../heat'

export interface CarouselPost extends Post {
  source: string
}

export default defineComponent({
  name: 'PostsCarousel',
  props: {
    posts: { type: Array as PropType<CarouselPost[]>, default: () => [] },
  },
  computed: {
    // Index range per source, so each source gets its own green->red scale.
    ranges(): Record<string, Range | null> {
      const bySource: Record<string, (number | null)[]> = {}
      for (const p of this.posts) (bySource[p.source] ??= []).push(p.index)
      const out: Record<string, Range | null> = {}
      for (const src in bySource) out[src] = rangeOf(bySource[src])
      return out
    },
  },
  methods: {
    sourceLabel,
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    accent(p: CarouselPost): string {
      return accent(heat(p.index, this.ranges[p.source] ?? null))
    },
    hot(p: CarouselPost): boolean {
      return isHot(heat(p.index, this.ranges[p.source] ?? null))
    },
    track(): Animation | undefined {
      const el = this.$refs.track as HTMLElement | undefined
      return el?.getAnimations?.()[0]
    },
    // Ease the marquee's playback rate toward `target` over ~700ms.
    ramp(target: number) {
      const anim = this.track()
      if (!anim) return
      const start = anim.playbackRate
      const t0 = performance.now()
      const dur = 700
      const step = (now: number) => {
        const k = Math.min((now - t0) / dur, 1)
        anim.playbackRate = start + (target - start) * k
        if (k < 1) requestAnimationFrame(step)
      }
      requestAnimationFrame(step)
    },
  },
})
</script>

<template>
  <div v-if="posts.length" class="carousel" @mouseenter="ramp(0)" @mouseleave="ramp(1)">
    <div ref="track" class="carousel-track">
      <div v-for="g in 2" :key="g" class="carousel-group">
        <a
          v-for="(p, i) in posts"
          :key="i"
          class="card"
          :class="{ hot: hot(p) }"
          :style="{ '--accent': accent(p) }"
          :href="p.comments_url"
          target="_blank"
          rel="noopener noreferrer"
        >
          <span class="card-src">{{ sourceLabel(p.source) }}</span>
          <span class="card-title">{{ p.title }}</span>
          <span class="card-meta">
            <span class="card-idx">idx {{ fmt(p.index) }}</span>
            <span class="card-cs">{{ p.comments }} comments / {{ p.score }} points</span>
          </span>
        </a>
      </div>
    </div>
  </div>
</template>
