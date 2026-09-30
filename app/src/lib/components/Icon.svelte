<script lang="ts">
  type Name = "home" | "capture" | "history" | "manager" | "camera" | "check";
  let { name, size = 22 }: { name: Name; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === "home"}
    <path d="M3 10.5 12 4l9 6.5" /><path d="M5.5 9.5V20h13V9.5" /><path d="M10 20v-5.5h4V20" />
  {:else if name === "capture"}
    <circle cx="12" cy="12" r="8.5" /><circle cx="12" cy="12" r="3.2" />
  {:else if name === "history"}
    <path d="M4 6.5h16" /><path d="M4 12h16" /><path d="M4 17.5h10" />
  {:else if name === "manager"}
    <path d="M6 3.5h10.5a2 2 0 0 1 2 2V20.5H8a2 2 0 0 1-2-2z" /><path d="M6 18.5a2 2 0 0 1 2-2h10.5" /><path d="M10 8h5" />
  {:else if name === "camera"}
    <path d="M4 8.5h3l1.6-2.5h6.8L17 8.5h3v10H4z" /><circle cx="12" cy="13.2" r="3.4" />
  {:else if name === "check"}
    <path d="m5 12.5 4.2 4.2L19 7" />
  {/if}
</svg>
