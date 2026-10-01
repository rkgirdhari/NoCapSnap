<script lang="ts">
  import { connectivity } from "$lib/connectivity.svelte";
  import Icon from "./Icon.svelte";
  // Network reachability only. It says nothing about sync, which W3 adds.
</script>

<span class="net" class:offline={!connectivity.online} role="status">
  <Icon name={connectivity.online ? "sync" : "cloud-off"} size={21} />
  <span class="dot" aria-hidden="true"></span>
  {connectivity.online ? "Online" : "Offline"}
</span>

<style>
  .net {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text);
  }
  .net :global(svg) {
    margin-right: 8px;
    color: var(--text);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok-dot);
  }
  .offline {
    color: var(--wait-hi);
  }
  .offline :global(svg) {
    color: var(--wait-hi);
  }
  .offline .dot {
    background: var(--wait-hi);
  }
</style>
