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
}
</script>

<template>
    <div :class="cn('flex items-center relative', props.class)">
        <!-- WHY DOES VUE NEED TYPE ASSERT -->
        <Input
            default-value="> "
            class="rounded-none px-1 py-0 m-0 text-2xl uppercase select-none font-mono"
            :value="'> ' + command"
            @input="
                (evt: InputEvent) =>
                    (command = (evt.target as HTMLInputElement).value.substring(
                        2,
                    ))
            "
            @focus="() => (focused = true)"
            @blur="() => (focused = false)"
            @keydown="handleShortcuts"
            v
        />

        <div
            :class="`absolute top-full border w-[640px] flex flex-col ${focused ? 'block' : 'hidden'}`"
        >
            <div class="">blank input</div>
        </div>
    </div>
</template>

<style scoped></style>
