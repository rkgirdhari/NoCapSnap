<script lang="ts">
  import Icon from "./Icon.svelte";
  import NetStatus from "./NetStatus.svelte";

  let {
    back,
    label,
    status = false,
    center = false,
  }: {
    /** Where the back arrow goes; a function for in-page steps. */
    back?: string | (() => void);
    /** Appended to the wordmark: "NO CAP SNAP · GUEST INVITATION". */
    label?: string;
    status?: boolean;
    center?: boolean;
  } = $props();
</script>

<header class="bar" class:center>
  <div class="left">
    {#if typeof back === "string"}
      <a class="back" href={back} aria-label="Back"><Icon name="arrow-left" size={26} stroke={1.5} /></a>
    {:else if back}
      <button class="back" onclick={back} aria-label="Back"><Icon name="arrow-left" size={26} stroke={1.5} /></button>
    {/if}
    <span class="wordmark">NO CAP SNAP{#if label}<span class="label">&ensp;·&ensp;{label}</span>{/if}</span>
  </div>
  {#if status}
    <NetStatus />
  {/if}
</header>

<style>
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    max-width: var(--maxw);
    margin: 0 auto;
    padding: calc(14px + var(--safe-top)) var(--gutter) 10px;
    min-height: 64px;
  }
  .center {
    justify-content: center;
  }
  .left {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
  }
  .back {
    display: inline-grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-left: -10px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  .wordmark {
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.3em;
    white-space: nowrap;
    color: var(--text);
  }
  .label {
    text-transform: uppercase;
  }
  .center .wordmark {
    font-size: 12px;
    letter-spacing: 0.24em;
  }
</style>
