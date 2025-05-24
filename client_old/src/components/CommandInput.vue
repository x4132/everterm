<script setup lang="ts">
import { Input } from "@/components/ui/input";
import { ref, type HTMLAttributes } from "vue";
import { cn } from "@/lib/utils";

const props = defineProps<{
    class?: HTMLAttributes["class"];
}>();

const command = ref("");
const focused = ref(false);

function handleShortcuts(evt: KeyboardEvent) {
    console.log(evt.code);
    if (evt.code === "Escape") {
        if (command.value === "") {
            focused.value = false;
            (evt.target as HTMLInputElement).blur();
        } else {
            command.value = "";
        }
    }

    return true;
}
</script>

<template>
    <div :class="cn('flex items-center relative focus-visible:border-ring focus-visible: ring-ring/50 focus-visible:ring-[3px]', props.class)">
        <span class="w-full flex text-2xl items-center bg-input">
            <span>&gt;&nbsp;</span>
            <Input
                default-value=""
                class="rounded-none px-1 py-0 m-0 text-2xl uppercase font-mono focus-visible:border-none focus-visible:ring-0"
                v-model="command"
                @focus="() => (focused = true)"
                @blur="() => (focused = false)"
                @keydown="handleShortcuts"
            />
        </span>

        <div
            :class="`absolute top-full border w-[640px] flex flex-col ${focused ? 'block' : 'hidden'}`"
        >
            <div class="">blank input</div>
        </div>
    </div>
</template>

<style scoped></style>
