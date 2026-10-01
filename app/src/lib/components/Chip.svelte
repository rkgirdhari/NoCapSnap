<script lang="ts">
  import type { SyncState } from "$lib/bridge";
  import Icon from "./Icon.svelte";
  // The mockup's chip, carrying the spec's words (Spec §3, owner default M5):
  // staff must never mistake a pending capture for a ready QR.
  // Demo captures (made while signed out) never sync, so they say so instead.
  let { state, demo = false }: { state: SyncState; demo?: boolean } = $props();
</script>

{#if demo}
  <span class="chip demo"><Icon name="plate" size={16} /> Demo · stays on this phone</span>
{:else}
  <span class="chip {state}">
    <Icon name={state === "synced" ? "qr" : "hourglass"} size={16} />
    {state === "synced" ? "Synced · QR ready" : "Saved offline · QR not ready"}
  </span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 4px 11px 4px 8px;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 500;
    line-height: 1.3;
    letter-spacing: 0.01em;
    /* The spec's wording is long; on a narrow phone it may wrap rather than spill. */
    text-wrap: balance;
    border: 1px solid color-mix(in srgb, currentColor 30%, transparent);
  }
  .chip :global(svg) {
    flex: none;
  }
  .pending {
    color: var(--wait);
    background: var(--wait-bg);
  }
  .synced {
    color: var(--ok);
    background: var(--ok-bg);
  }
  .demo {
    color: var(--secondary);
    background: var(--card);
  }
</style>
