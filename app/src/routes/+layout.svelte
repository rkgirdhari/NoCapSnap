<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { bridge } from "$lib/bridge";
  import Connectivity from "$lib/components/Connectivity.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Seal from "$lib/components/Seal.svelte";

  let { children } = $props();

  const tabs = [
    { href: "/", label: "Home", icon: "home" },
    { href: "/capture", label: "Capture", icon: "capture" },
    { href: "/history", label: "History", icon: "history" },
    { href: "/manager", label: "Manager", icon: "manager" },
  ] as const;

  const isActive = (href: string) =>
    href === "/" ? page.url.pathname === "/" : page.url.pathname.startsWith(href);
</script>

<div class="shell">
  <header class="masthead">
    <a class="brand" href="/" aria-label="CapSnap home">
      <Seal size={28} />
      <span class="wordmark">CapSnap</span>
    </a>
    <Connectivity />
  </header>

  {#if bridge.mode === "preview"}
    <p class="ribbon">Browser preview · nothing is saved</p>
  {/if}

  <main>
    {@render children()}
  </main>

  <nav class="tabbar" aria-label="Sections">
    {#each tabs as tab (tab.href)}
      <a class="tab" class:active={isActive(tab.href)} href={tab.href} aria-current={isActive(tab.href) ? "page" : undefined}>
        <Icon name={tab.icon} />
        <span>{tab.label}</span>
      </a>
    {/each}
  </nav>
</div>

<style>
  .shell {
    min-height: 100dvh;
    padding-bottom: calc(var(--tabbar) + env(safe-area-inset-bottom));
  }

  .masthead {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: calc(12px + env(safe-area-inset-top)) var(--gutter) 12px;
    background: var(--bg);
    border-bottom: 1px solid var(--line);
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    text-decoration: none;
  }

  .wordmark {
    font-family: var(--serif);
    font-size: 21px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .ribbon {
    margin: 0;
    padding: 7px var(--gutter);
    background: var(--bg-deep);
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-align: center;
  }

  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 28px var(--gutter) 36px;
  }

  .tabbar {
    position: fixed;
    inset: auto 0 0 0;
    z-index: 5;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    height: calc(var(--tabbar) + env(safe-area-inset-bottom));
    padding-bottom: env(safe-area-inset-bottom);
    background: var(--bg-deep);
    border-top: 1px solid var(--line);
  }

  .tab {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-decoration: none;
  }

  .tab.active {
    color: var(--accent);
  }

  /* A single gold stroke marks where you are. */
  .tab.active::before {
    content: "";
    position: absolute;
    top: -1px;
    left: 26%;
    right: 26%;
    height: 2px;
    border-radius: 0 0 2px 2px;
    background: var(--accent);
  }
</style>
