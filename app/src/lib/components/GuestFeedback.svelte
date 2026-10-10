<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type FeedbackItem, type FeedbackSummary, type Profile } from "$lib/bridge";
  import { whenLabel } from "$lib/format";

  // What guests said, for administrators and managers (the roles the server allows). Newest first;
  // never sorted, filtered or highlighted by rating (Spec §4: the form and the reading are neutral).
  const PERIODS = [
    { days: 7, label: "7 days" },
    { days: 30, label: "30 days" },
    { days: 90, label: "90 days" },
  ];

  let profile = $state<Profile | null>(null);
  let days = $state(30);
  let summary = $state<FeedbackSummary | null>(null);
  let items = $state<FeedbackItem[]>([]);
  let nextBefore = $state<string | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  const allowed = $derived(profile?.signedIn === true && (profile.role === "admin" || profile.role === "manager"));
  const most = $derived(Math.max(1, ...(summary?.distribution ?? [1])));

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

  async function load(more = false) {
    loading = true;
    error = null;
    try {
      const page = await bridge.feedback(days, more ? nextBefore : null);
      summary = page.summary;
      items = more ? [...items, ...page.items] : page.items;
      nextBefore = page.nextBefore;
    } catch (e) {
      error = message(e);
    } finally {
      loading = false;
    }
  }

  function choose(d: number) {
    if (d === days) return;
    days = d;
    void load();
  }

  onMount(async () => {
    try {
      profile = await bridge.profile();
    } catch (e) {
      error = message(e);
      return;
    }
    if (allowed) await load();
  });
</script>

<section aria-labelledby="gf-title">
  <h2 class="title sec" id="gf-title">Guest feedback</h2>

  {#if !profile}
    <!-- still loading the profile -->
  {:else if !profile.signedIn}
    <div class="card note">
      <p class="muted">Sign in to a server in Settings to read what guests said.</p>
    </div>
  {:else if !allowed}
    <div class="card note">
      <p class="muted">Guest feedback is shown to administrators and managers.</p>
    </div>
  {:else}
    <div class="periods" role="group" aria-label="Period">
      {#each PERIODS as p (p.days)}
        <button class="period" class:on={days === p.days} aria-pressed={days === p.days} onclick={() => choose(p.days)}>
          {p.label}
        </button>
      {/each}
    </div>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if summary}
      {#if summary.count === 0}
        <div class="card note">
          <p class="muted">No guest has answered in the last {days} days.</p>
        </div>
      {:else}
        <div class="card summary">
          <div class="avg">
            <strong>{summary.average?.toFixed(1)}</strong>
            <span>average of {summary.count} {summary.count === 1 ? "answer" : "answers"}</span>
          </div>
          <ol class="dist" aria-label="Answers by rating">
            {#each [5, 4, 3, 2, 1] as rating (rating)}
              {@const n = summary.distribution[rating - 1]}
              <li>
                <span class="r">{rating}</span>
                <span class="track"><span class="fill" style:width="{(n / most) * 100}%"></span></span>
                <span class="n">{n}</span>
              </li>
            {/each}
          </ol>
        </div>

        <ul class="answers">
          {#each items as item (item.id)}
            <li class="card answer">
              <div class="head">
                <span class="rating" aria-label="{item.rating} out of 5">{item.rating}<small>/5</small></span>
                <span class="dish">{item.dishName ?? "Unlabelled dish"}</span>
              </div>
              {#if item.comment}<p class="comment">{item.comment}</p>{/if}
              <p class="when muted">{whenLabel(item.createdAt)}</p>
            </li>
          {/each}
        </ul>
        {#if nextBefore}
          <button class="btn outline block" onclick={() => load(true)} disabled={loading}>
            {loading ? "Loading…" : "Show older answers"}
          </button>
        {/if}
      {/if}
    {:else if loading}
      <p class="helper">Loading…</p>
    {/if}
    <p class="helper foot">Private to your restaurant. Guests are never named; answers are kept for 12 months.</p>
  {/if}
</section>

<style>
  section {
    margin-top: 12px;
  }
  .note {
    padding: 16px 18px;
  }
  .note p {
    margin: 0;
  }
  .periods {
    display: flex;
    gap: 8px;
    margin-bottom: 14px;
  }
  .period {
    min-height: 40px;
    padding: 0 16px;
    border: 1px solid var(--tab-active);
    border-radius: 999px;
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 14.5px;
  }
  .period.on {
    background: var(--tab-active);
    color: var(--text);
  }
  .summary {
    display: grid;
    gap: 16px;
    padding: 16px 18px;
    margin-bottom: 14px;
  }
  .avg {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .avg strong {
    font-family: var(--serif);
    font-size: 40px;
    line-height: 1.1;
    color: var(--headline);
  }
  .avg span {
    color: var(--muted);
    font-size: 14.5px;
  }
  .dist {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .dist li {
    display: grid;
    grid-template-columns: 1.2em 1fr 2.4em;
    align-items: center;
    gap: 10px;
    font-variant-numeric: tabular-nums;
  }
  .r,
  .n {
    color: var(--secondary);
  }
  .n {
    text-align: right;
  }
  .track {
    height: 6px;
    border-radius: 3px;
    background: var(--stat);
    overflow: hidden;
  }
  /* Every rating gets the same colour: nothing is flagged good or bad. */
  .fill {
    display: block;
    height: 100%;
    background: var(--headline);
  }
  .answers {
    display: grid;
    gap: 10px;
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
  }
  .answer {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .rating {
    font-family: var(--serif);
    font-size: 24px;
    color: var(--headline);
  }
  .rating small {
    font-size: 14px;
    color: var(--muted);
  }
  .dish {
    font-weight: 600;
  }
  .comment {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .when {
    margin: 0;
    font-size: 13.5px;
  }
  .foot {
    margin-top: 16px;
  }
</style>
