<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { Vue3Marquee } from 'vue3-marquee'
import { sourceLabel, type Post } from '../api'
import { rangeOf, heat, accent, isHot, type Range } from '../heat'

export interface CarouselPost extends Post {
  source: string
}

export default defineComponent({
  name: 'PostsCarousel',
  components: { Vue3Marquee },
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
  },
})
</script>

<template>
  <Vue3Marquee
    v-if="posts.length"
    class="carousel"
    :duration="95"
    :clone="true"
    :pause-on-hover="true"
  >
    <a
      v-for="p in posts"
      :key="p.id"
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
  </Vue3Marquee>
</template>
