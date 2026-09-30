<script lang="ts">
  import { mediaUrl } from "$lib/media";
  import Icon from "./Icon.svelte";

  let {
    sha256,
    thumb = true,
    alt = "",
    radius = "10px",
  }: { sha256: string | null | undefined; thumb?: boolean; alt?: string; radius?: string } = $props();

  let url = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const digest = sha256;
    url = null;
    failed = false;
    if (!digest) return;
    let live = true;
    mediaUrl(digest, thumb).then(
      (u) => live && (url = u),
      () => live && (failed = true),
    );
    return () => {
      live = false;
    };
  });
</script>

<div class="thumb" style:border-radius={radius}>
  {#if url}
    <img src={url} {alt} />
  {:else}
    <span class="empty" title={failed ? "Photo not on this device" : undefined}>
      <Icon name="plate" size={28} stroke={1.2} />
    </span>
  {/if}
</div>

<style>
  .thumb {
    position: relative;
    overflow: hidden;
    width: 100%;
    height: 100%;
    background: radial-gradient(circle at 50% 45%, #33222f, #1b1020);
    border: 1px solid var(--hairline);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
</style>
