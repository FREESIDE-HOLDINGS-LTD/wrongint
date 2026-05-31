<script lang="ts">
import { defineComponent, type PropType } from 'vue'

export interface TickerItem {
  label: string
  value: number | null
}

export default defineComponent({
  name: 'Ticker',
  props: {
    items: { type: Array as PropType<TickerItem[]>, default: () => [] },
  },
  computed: {
    // Duplicate the list so the marquee can loop seamlessly.
    loop(): TickerItem[] {
      return [...this.items, ...this.items, ...this.items, ...this.items]
    },
  },
  methods: {
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(3)
    },
  },
})
</script>

<template>
  <div class="ticker">
    <div class="ticker-track">
      <span v-for="(it, i) in loop" :key="i" class="tick">
        <span class="tick-label">{{ it.label }}</span>
        <span class="tick-value" :class="{ dead: it.value == null }">{{ fmt(it.value) }}</span>
        <span class="tick-sep">◆</span>
      </span>
    </div>
  </div>
</template>
