<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type CaptureRecord } from "$lib/bridge";
  import Icon from "$lib/components/Icon.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { isSameLocalDay } from "$lib/format";

  // Only what this device knows: its own captures. Guest ratings need the
  // server and guest feedback (later phases), so they are not faked here.
  let captures = $state<CaptureRecord[]>([]);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      captures = await bridge.list(1000);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  const now = new Date();
  const weekAgo = now.getTime() - 7 * 24 * 60 * 60 * 1000;
  const week = $derived(captures.filter((c) => new Date(c.capturedAt).getTime() >= weekAgo));
  const today = $derived(captures.filter((c) => isSameLocalDay(new Date(c.capturedAt), now)).length);
  const topDishes = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const c of week) {
      const name = c.dishName ?? "Unlabelled dish";
      counts.set(name, (counts.get(name) ?? 0) + 1);
    }
    return [...counts].sort((a, b) => b[1] - a[1]).slice(0, 5);
  });
  const most = $derived(topDishes[0]?.[1] ?? 1);
</script>

<TopBar status />

<main class="page">
  <p class="kicker">Insights · this device</p>
  <h1 class="display">The week at the pass.</h1>

  <div class="nums">
    <div class="num card"><strong>{today}</strong><span>captured today</span></div>
    <div class="num card"><strong>{week.length}</strong><span>in the last 7 days</span></div>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <h2 class="title sec">Most captured</h2>
  {#if topDishes.length}
    <ol class="bars">
      {#each topDishes as [name, count] (name)}
        <li>
          <span class="bar-name">{name}</span>
          <span class="bar-count">{count}</span>
          <span class="bar" style:width="{(count / most) * 100}%" aria-hidden="true"></span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="helper">Nothing captured in the last 7 days.</p>
  {/if}

  <div class="card soon">
    <Icon name="insights" size={28} stroke={1.4} />
    <div>
      <strong>Guest feedback</strong>
      <p class="muted">Ratings and comments from guests appear here once guest invitations are live.</p>
    </div>
  </div>
</main>

<style>
  .display {
    margin-top: 8px;
  }
  .nums {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-top: 22px;
  }
  .num {
    display: grid;
    gap: 2px;
    padding: 16px;
  }
  .num strong {
    font-family: var(--serif);
    font-size: 34px;
    line-height: 1.1;
    color: var(--headline);
  }
  .num span {
    color: var(--muted);
    font-size: 14.5px;
  }
  .sec {
    margin: 28px 0 12px;
  }
  .bars {
    display: grid;
    gap: 14px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .bars li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px 12px;
  }
  .bar-name {
    font-size: 16px;
  }
  .bar-count {
    color: var(--secondary);
    font-variant-numeric: tabular-nums;
  }
  .bar {
    grid-column: 1 / -1;
    height: 6px;
    border-radius: 3px;
    background: var(--tab-active);
  }
  .soon {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 14px;
    margin-top: 28px;
    padding: 16px 18px;
    color: var(--muted);
  }
  .soon strong {
    color: var(--text);
    font-weight: 600;
  }
  .soon p {
    margin: 4px 0 0;
  }
</style>
