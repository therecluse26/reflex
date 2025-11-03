<script lang="ts">
  import { onMount } from 'svelte';

  export let name: string = 'World';

  let count = 0;

  $: doubled = count * 2;
  $: console.log('Count changed:', count);

  function increment() {
    count += 1;
  }

  function decrement() {
    count -= 1;
  }

  onMount(() => {
    console.log('Component mounted');
  });
</script>

<main>
  <h1>Hello {name}!</h1>
  <p>Count: {count}</p>
  <p>Doubled: {doubled}</p>
  <button on:click={increment}>+</button>
  <button on:click={decrement}>-</button>
</main>

<style>
  main {
    padding: 1em;
    max-width: 240px;
    margin: 0 auto;
  }

  h1 {
    color: #ff3e00;
  }
</style>
