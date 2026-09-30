<script lang="ts">
  // The CapSnap hanko: a gold seal, pressed slightly off-true like a real stamp.
  let { size = 28 }: { size?: number } = $props();
</script>

<svg class="seal" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <rect x="1.5" y="1.5" width="29" height="29" rx="5" fill="var(--accent)" />
  <rect x="4" y="4" width="24" height="24" rx="3" fill="none" stroke="var(--on-accent)" stroke-width="0.9" opacity="0.7" />
  <text x="16" y="20.5" text-anchor="middle" font-family="var(--serif)" font-size="12" font-weight="700" fill="var(--on-accent)" letter-spacing="0.5">CS</text>
</svg>

<style>
  .seal {
    transform: rotate(-3deg);
    flex: none;
  }
</style>
