<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import type { Post } from '../api'

// One post, rendered identically in the carousel and in the battle flanks.
// `kind` only switches the CSS class family so each context keeps its own look;
// the markup and the date/index formatting live here, in one place.
export default defineComponent({
  name: 'PostCard',
  props: {
    post: { type: Object as PropType<Post>, required: true },
    kind: {
      type: String as PropType<'carousel' | 'contender' | 'champion'>,
      default: 'contender',
    },
    accent: { type: String, default: '#5a6470' },
    hot: { type: Boolean, default: false },
    emoji: { type: String, default: '' },
    // Small label above the title: source name in the carousel, WIRED/TIRED on a
    // champion. Empty hides it.
    label: { type: String, default: '' },
    // Plain challengers use a smaller emoji than carousel cards / champions.
    sm: { type: Boolean, default: false },
  },
  computed: {
    // Class-name family for the inner pieces: carousel keeps `card-*`, the
    // battle keeps `contender-*`, so each context's existing CSS still applies.
    pre(): string {
      return this.kind === 'carousel' ? 'card' : 'contender'
    },
    rootClass(): Record<string, boolean> {
      return {
        card: this.kind === 'carousel',
        contender: this.kind !== 'carousel',
        champion: this.kind === 'champion',
        hot: this.hot,
      }
    },
    emojiClass(): Record<string, boolean> {
      return {
        'card-emoji': this.kind === 'carousel',
        'contender-emoji': this.kind !== 'carousel',
        'contender-emoji--sm': this.sm,
      }
    },
    labelClass(): string {
      return this.kind === 'carousel' ? 'card-src' : 'contender-label'
    },
  },
  methods: {
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    fmtDate(iso: string): string {
      const d = new Date(iso)
      if (isNaN(d.getTime())) return ''
      const secs = (d.getTime() - Date.now()) / 1000
      const units: [Intl.RelativeTimeFormatUnit, number][] = [
        ['year', 31536000],
        ['month', 2592000],
        ['day', 86400],
        ['hour', 3600],
        ['minute', 60],
        ['second', 1],
      ]
      const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
      for (const [unit, s] of units) {
        if (Math.abs(secs) >= s || unit === 'second') {
          return rtf.format(Math.round(secs / s), unit)
        }
      }
      return ''
    },
  },
})
</script>

<template>
  <a
    :class="rootClass"
    :style="{ '--accent': accent }"
    :href="post.comments_url"
    :title="post.title"
    target="_blank"
    rel="noopener noreferrer"
  >
    <span :class="emojiClass" aria-hidden="true">{{ emoji }}</span>
    <span :class="pre + '-body'">
      <span v-if="label" :class="labelClass">{{ label }}</span>
      <span :class="pre + '-title'">{{ post.title }}</span>
      <span :class="pre + '-meta'">
        <span :class="pre + '-idx'">idx {{ fmt(post.index) }}</span>
        <span :class="pre + '-cs'">{{ post.comments }} comments / {{ post.score }} points</span>
      </span>
      <span :class="pre + '-date'">{{ fmtDate(post.posted_at) }}</span>
    </span>
  </a>
</template>
