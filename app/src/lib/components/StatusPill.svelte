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
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .pending {
    color: var(--indigo);
    background: var(--indigo-tint);
  }
  .synced {
    color: var(--matcha);
    background: var(--matcha-tint);
  }
</style>
