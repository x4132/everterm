<script setup lang="ts">
import CommandInput from "@/components/CommandInput.vue";
</script>

<template>
    <div class="dark w-full h-full bg-background text-primary">
        <nav class="flex">
            <CommandInput class="grow" />
        </nav>

        <main>
            <RouterView />
        </main>
    </div>
</template>

<style scoped></style>
