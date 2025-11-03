<script context="module" lang="ts">
  import { writable } from 'svelte/store';

  export const globalCount = writable(0);

  export function resetGlobalCount() {
    globalCount.set(0);
  }
</script>

<script lang="ts">
  import { derived } from 'svelte/store';

  const doubled = derived(globalCount, $count => $count * 2);

  function increment() {
    globalCount.update(n => n + 1);
  }

  function decrement() {
    globalCount.update(n => n - 1);
  }
</script>

<div>
  <p>Count: {$globalCount}</p>
  <p>Doubled: {$doubled}</p>
  <button on:click={increment}>Increment</button>
  <button on:click={decrement}>Decrement</button>
</div>
