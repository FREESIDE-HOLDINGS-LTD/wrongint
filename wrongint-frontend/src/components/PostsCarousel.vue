<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { Vue3Marquee } from 'vue3-marquee'
import PostCard from './PostCard.vue'
import { sourceLabel, type Post } from '../api'
import { rangeOf, heat, accent, isHot, emoji, type Range } from '../heat'

export interface CarouselPost extends Post {
  source: string
}

export default defineComponent({
  name: 'PostsCarousel',
  components: { Vue3Marquee, PostCard },
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
    accent(p: CarouselPost): string {
      return accent(heat(p.index, this.ranges[p.source] ?? null))
    },
    hot(p: CarouselPost): boolean {
      return isHot(heat(p.index, this.ranges[p.source] ?? null))
    },
    emoji(p: CarouselPost): string {
      return emoji(heat(p.index, this.ranges[p.source] ?? null))
    },
  },
})
</script>

<template>
  <Vue3Marquee
    v-if="posts.length"
    class="carousel"
    :duration="120"
    :clone="true"
    :pause-on-hover="true"
  >
    <PostCard
      v-for="p in posts"
      :key="p.id"
      kind="carousel"
      :post="p"
      :accent="accent(p)"
      :hot="hot(p)"
      :emoji="emoji(p)"
      :label="sourceLabel(p.source)"
    />
  </Vue3Marquee>
</template>
