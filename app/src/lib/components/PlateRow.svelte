<script lang="ts">
  import type { CaptureRecord } from "$lib/bridge";
  import { formatTime, whenLabel } from "$lib/format";
  import Chip from "./Chip.svelte";
  import Icon from "./Icon.svelte";
  import Thumb from "./Thumb.svelte";

  // One captured plate. `withDay` for mixed-day lists (Home); History groups by
  // day, so it shows the time and the device-only table label instead.
  let { capture, withDay = false }: { capture: CaptureRecord; withDay?: boolean } = $props();
</script>

<a class="row list-card" href="/invite?c={encodeURIComponent(capture.clientId)}">
  <span class="pic"><Thumb sha256={capture.sha256} alt="" /></span>
  <span class="name">{capture.dishName ?? "Unlabelled dish"}</span>
  <span class="meta">
    {#if withDay}{whenLabel(capture.capturedAt)}{:else}{formatTime(capture.capturedAt)}{#if capture.tableLabel}&ensp;·&ensp;Table
        {capture.tableLabel}{/if}{/if}
  </span>
  <span class="state"><Chip state={capture.syncState} demo={capture.isDemo} /></span>
  <span class="go"><Icon name="chevron-right" size={22} stroke={1.5} /></span>
</a>

<style>
  .row {
    display: grid;
    grid-template-columns: 76px 1fr 22px;
    grid-template-areas:
      "pic name go"
      "pic meta go"
      "pic state state";
    align-items: center;
    column-gap: 14px;
    row-gap: 4px;
    padding: 8px 12px 8px 8px;
    text-decoration: none;
  }
  .pic {
    grid-area: pic;
    display: block;
    height: 76px;
  }
  .name {
    grid-area: name;
    min-width: 0;
    overflow: hidden;
    font-size: 16px;
    font-weight: 550;
    letter-spacing: 0.01em;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text);
  }
  .meta {
    grid-area: meta;
    font-size: 13.5px;
    color: var(--secondary);
  }
  .state {
    grid-area: state;
    min-width: 0;
  }
  .go {
    grid-area: go;
    color: var(--text);
  }
</style>
