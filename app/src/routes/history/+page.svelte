<script lang="ts">
  import { bridge, type CaptureRecord } from "$lib/bridge";
  import StatusPill from "$lib/components/StatusPill.svelte";
  import { dayLabel, formatBytes, formatTime, shortDigest } from "$lib/format";

  let captures = $state<CaptureRecord[]>([]);
  let loaded = $state(false);

  const groups = $derived.by(() => {
    const out: { label: string; items: CaptureRecord[] }[] = [];
    for (const c of captures) {
      const label = dayLabel(c.capturedAt);
      const last = out.at(-1);
      if (last?.label === label) last.items.push(c);
      else out.push({ label, items: [c] });
    }
    return out;
  });

  $effect(() => {
    bridge
      .list()
      .then((list) => (captures = list))
      .finally(() => (loaded = true));
  });
</script>

<p class="kicker">History</p>
<h1 class="display">Plates, in order.</h1>
<p class="lede">Everything captured on this phone, newest first.</p>

{#if loaded && captures.length === 0}
  <div class="empty card">
    <svg class="guide" viewBox="0 0 100 100" aria-hidden="true">
      <circle cx="50" cy="50" r="42" />
      <circle cx="50" cy="50" r="31" />
    </svg>
    <h2>Nothing plated yet.</h2>
    <p class="muted">The first plate you capture shows up here.</p>
    <a class="btn primary" href="/capture">Capture a plate</a>
  </div>
{:else}
  {#each groups as group (group.label)}
    <h2 class="day">{group.label}</h2>
    <ul class="ledger on-surface">
      {#each group.items as c (c.clientId)}
        <li>
          <span class="time">{formatTime(c.capturedAt)}</span>
          <span class="state"><StatusPill state={c.syncState} /></span>
          <span class="mono muted meta">#{shortDigest(c.sha256)} · {formatBytes(c.bytes)}</span>
        </li>
      {/each}
    </ul>
  {/each}
{/if}

<style>
  .day {
    margin: 30px 0 10px;
    font-size: 15px;
    font-family: var(--sans);
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .ledger {
    list-style: none;
    margin: 0;
    padding: 0 14px;
    border: 1px solid var(--surface-edge);
    border-radius: var(--radius);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.05), transparent 40%), var(--surface);
    box-shadow: var(--shadow);
  }

  /* Time and status on one line, digest beneath: the status chip keeps its
     full wording without crowding the card edge on a narrow phone. */
  li {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-areas:
      "time state"
      "meta meta";
    align-items: center;
    gap: 6px 12px;
    padding: 14px 0;
  }

  li + li {
    border-top: 1px solid color-mix(in srgb, var(--muted-gold) 70%, transparent);
  }

  .time {
    grid-area: time;
    font-family: var(--serif);
    font-size: 20px;
    font-variant-numeric: lining-nums tabular-nums;
  }

  .state {
    grid-area: state;
    justify-self: end;
  }

  .meta {
    grid-area: meta;
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 10px;
    margin-top: 28px;
    padding: 34px 20px;
    text-align: center;
  }

  .empty h2 {
    font-size: 22px;
  }

  .empty p {
    margin: 0 0 8px;
  }

  .guide {
    width: 80px;
    height: 80px;
  }

  .guide circle {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
  }

  .guide circle + circle {
    stroke: var(--text);
    stroke-width: 1.2;
    opacity: 0.7;
  }
</style>
