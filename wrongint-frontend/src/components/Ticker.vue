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
  methods: {
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
  },
})
</script>

<template>
  <div class="ticker">
    <div class="ticker-track">
      <div v-for="g in 2" :key="g" class="ticker-group">
        <span v-for="(it, i) in items" :key="i" class="tick">
          <span class="tick-label">{{ it.label }}</span>
          <span class="tick-value" :class="{ dead: it.value == null }">{{ fmt(it.value) }}</span>
          <span class="tick-sep">◆</span>
        </span>
      </div>
    </div>
  </div>
</template>
