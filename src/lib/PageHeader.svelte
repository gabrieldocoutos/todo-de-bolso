<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, subtitle }: { title: string; subtitle?: Snippet } = $props();
</script>

<div class="page-header">
  <h1 class="page-title">{title}</h1>
  {#if subtitle}
    <p class="page-subtitle">{@render subtitle()}</p>
  {/if}
</div>

<style>
  .page-header {
    margin-bottom: 24px;
  }

  .page-title {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 1.5rem;
    font-weight: 700;
    color: #e5e2e1;
    margin: 0 0 8px;
  }

  .page-subtitle {
    font-size: 0.8125rem;
    color: #bccac4;
    margin: 0;
    line-height: 1.5;
  }
</style>
