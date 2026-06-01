<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import type { Post } from '../api'

// A rotating radar scope that sweeps for the word "blorp" across captured
// posts. Reports how long since the last sighting; the whole scope links to
// that post. Fills the left margin beside the global graph. `post` is null
// when nothing has ever mentioned blorp.
export default defineComponent({
  name: 'BlorpDetector',
  props: {
    post: { type: Object as PropType<Post | null>, default: null },
  },
  data() {
    return {
      now: Date.now(),
      ticker: undefined as ReturnType<typeof setInterval> | undefined,
    }
  },
  mounted() {
    // Tick once a second so the "time since" counter s live.
    this.ticker = setInterval(() => {
      this.now = Date.now()
    }, 1000)
  },
  beforeUnmount() {
    clearInterval(this.ticker)
  },
  computed: {
    detected(): boolean {
      return this.post != null
    },
    sinceText(): string {
      if (!this.post) return ''
      const t = new Date(this.post.posted_at).getTime()
      if (isNaN(t)) return ''
      let s = Math.max(0, Math.floor((this.now - t) / 1000))
      const d = Math.floor(s / 86400)
      s -= d * 86400
      const h = Math.floor(s / 3600)
      s -= h * 3600
      const m = Math.floor(s / 60)
      s -= m * 60
      const pad = (n: number) => String(n).padStart(2, '0')
      if (d > 0) return `${d}d ${pad(h)}h ${pad(m)}m ${pad(s)}s`
      if (h > 0) return `${h}h ${pad(m)}m ${pad(s)}s`
      if (m > 0) return `${m}m ${pad(s)}s`
      return `${s}s`
    },
  },
})
</script>

<template>
  <component
    :is="post ? 'a' : 'div'"
    class="radar"
    :class="{ 'radar--on': detected }"
    :href="post ? post.comments_url : null"
    target="_blank"
    rel="noopener"
  >
    <div class="radar-scope">
      <div class="radar-grid"></div>
      <div class="radar-sweep"></div>
      <span v-if="detected" class="radar-blip"></span>
    </div>

    <div class="radar-readout">
      <div class="radar-label">TIME SINCE LAST BLORP</div>
      <div v-if="post" class="radar-since">{{ sinceText }}</div>
      <div v-else class="radar-status radar-status--idle">
        NO BLORP DETECTED<span class="blink">_</span>
      </div>
    </div>
  </component>
</template>
