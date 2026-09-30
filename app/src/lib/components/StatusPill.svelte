<script lang="ts">
  import type { SyncState } from "$lib/bridge";
  // Wording is the spec's own (§3): staff must never mistake a pending capture for a ready QR.
  let { state }: { state: SyncState } = $props();
</script>

<span class="pill {state}">
  <span class="dot" aria-hidden="true"></span>
  {state === "synced" ? "Synced · QR ready" : "Saved offline · QR not ready"}
</span>

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px 6px 10px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    /* A dark chip keeps gold/green text legible on purple and on bamboo. */
    background: var(--chip);
    border: 1px solid color-mix(in srgb, currentColor 35%, transparent);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .pending {
    color: var(--pending);
  }
  .synced {
    color: var(--success);
  }
</style>
