<script lang="ts">
  import { encode } from "uqr";

  // The real guest link as a QR code: round data dots as in the mockup, with
  // standard square finder patterns. The mockup's rounded finders were dropped
  // after a decoder test (W3a): with rounded finders OpenCV could not read the
  // code at any size; with square ones jsQR, ZXing and OpenCV all can. Error
  // correction H; alignment, timing and format modules stay square; the
  // standard 4-module quiet zone is kept.
  let { url }: { url: string } = $props();

  const POSITION = 2;
  const DATA = 0;
  const QUIET = 4;

  const qr = $derived(encode(url, { ecc: "H", border: 0 }));
  const finders = $derived([
    [0, 0],
    [qr.size - 7, 0],
    [0, qr.size - 7],
  ]);
  const modules = $derived.by(() => {
    const dots: [number, number][] = [];
    const squares: [number, number][] = [];
    qr.data.forEach((row, y) =>
      row.forEach((dark, x) => {
        const kind = qr.types[y][x];
        if (!dark || kind === POSITION) return;
        (kind === DATA ? dots : squares).push([x, y]);
      }),
    );
    return { dots, squares };
  });
</script>

<svg viewBox="{-QUIET} {-QUIET} {qr.size + 2 * QUIET} {qr.size + 2 * QUIET}" role="img" aria-label="Guest feedback QR code">
  <rect x={-QUIET} y={-QUIET} width={qr.size + 2 * QUIET} height={qr.size + 2 * QUIET} class="paper" />
  {#each finders as [fx, fy] (`${fx}-${fy}`)}
    <path d="M{fx} {fy}h7v7h-7z M{fx + 1} {fy + 1}v5h5v-5z" class="ink" fill-rule="evenodd" />
    <rect x={fx + 2} y={fy + 2} width="3" height="3" class="ink" />
  {/each}
  {#each modules.squares as [x, y] (`s${x}-${y}`)}
    <rect {x} {y} width="1.02" height="1.02" class="ink" />
  {/each}
  {#each modules.dots as [x, y] (`d${x}-${y}`)}
    <circle cx={x + 0.5} cy={y + 0.5} r="0.46" class="ink" />
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
    border-radius: 14px;
  }
  .paper {
    fill: var(--qr-paper);
  }
  .ink {
    fill: var(--qr-ink);
  }
</style>
