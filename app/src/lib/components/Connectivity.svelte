<script lang="ts">
  import { connectivity } from "$lib/connectivity.svelte";
</script>

<span class="net" class:offline={!connectivity.online} role="status">
  <span class="dot" aria-hidden="true"></span>
  {connectivity.online ? "Online" : "Offline"}
</span>

<style>
  .net {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 11px;
    border-radius: 999px;
    border: 1px solid var(--hairline);
    background: var(--paper-raised);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--matcha);
  }
  .net.offline {
    color: var(--indigo);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
</style>
