<template>
  <div>
    <h1>{{ title }}</h1>
    <p>{{ message }}</p>
    <button @click="handleClick">Click me</button>
  </div>
</template>

<script>
export default {
  name: 'OptionsComponent',

  props: {
    title: {
      type: String,
      required: true
    }
  },

  data() {
    return {
      message: 'Hello Vue',
      count: 0
    };
  },

  computed: {
    uppercaseMessage() {
      return this.message.toUpperCase();
    },

    formattedCount() {
      return `Count: ${this.count}`;
    }
  },

  methods: {
    handleClick() {
      this.count++;
      this.$emit('clicked', this.count);
    },

    resetCount() {
      this.count = 0;
    }
  },

  watch: {
    count(newValue, oldValue) {
      console.log(`Count: ${oldValue} -> ${newValue}`);
    }
  },

  mounted() {
    console.log('Component mounted');
  },

  unmounted() {
    console.log('Component unmounted');
  }
};
</script>

<style scoped>
h1 {
  color: blue;
}
</style>
