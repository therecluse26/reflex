<template>
  <div class="counter">
    <p>Count: {{ count }}</p>
    <button @click="increment">Increment</button>
    <button @click="decrement">Decrement</button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';

const count = ref(0);

const doubleCount = computed(() => count.value * 2);

function increment() {
  count.value++;
}

function decrement() {
  count.value--;
}

watch(count, (newValue, oldValue) => {
  console.log(`Count changed from ${oldValue} to ${newValue}`);
});

onMounted(() => {
  console.log('Component mounted');
});
</script>

<style scoped>
.counter {
  padding: 20px;
}
</style>
