<script lang="ts">
import { defineComponent } from 'vue'

// Elevator-style direction indicator. Both ▲/▼ slots always occupy space so the
// lit arrow never shifts. When moving, only the arrow for the current direction
// shows (colored by the readout) and the other is hidden-but-reserved. When idle
// (dir === 0) both arrows show, greyed.
export default defineComponent({
  name: 'DirArrows',
  props: {
    dir: { type: Number, required: true },
  },
})
</script>

<template>
  <span class="dir-arrows" aria-hidden="true">
    <span class="dir-arrow" :class="{ active: dir > 0, hidden: dir < 0 }">▲</span>
    <span class="dir-arrow" :class="{ active: dir < 0, hidden: dir > 0 }">▼</span>
  </span>
</template>
