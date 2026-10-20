<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import logo from "./assets/logo.svg";
import { LOCALES, resolveLocale } from "./i18n";

const locale = ref(
  resolveLocale(
    localStorage.getItem("spaces-lang"),
    navigator.language || "es",
  ).code,
);

const t = computed(
  () => (LOCALES.find((l) => l.code === locale.value) ?? LOCALES[0]).strings,
);

watch(locale, (code) => {
  localStorage.setItem("spaces-lang", code);
  invoke("set_lang", { lang: code }).catch(() => {});
}, { immediate: true });

const langOpen = ref(false);
const langRef = ref<HTMLElement | null>(null);

const localeLabel = computed(
  () => (LOCALES.find((l) => l.code === locale.value) ?? LOCALES[0]).label,
);

function selectLocale(code: string) {
  locale.value = code;
  langOpen.value = false;
}

function onDocClick(e: MouseEvent) {
  if (langRef.value && !langRef.value.contains(e.target as Node)) {
    langOpen.value = false;
  }
}

onMounted(() => document.addEventListener("mousedown", onDocClick));
onBeforeUnmount(() => document.removeEventListener("mousedown", onDocClick));

const trigger = ref("MetaLeft");
const listening = ref(false);
const error = ref("");

const CODE_TO_KEY: Record<string, string> = {
  MetaLeft: "MetaLeft",
  MetaRight: "MetaRight",
  AltLeft: "Alt",
  AltRight: "AltGr",
  ShiftLeft: "ShiftLeft",
  ShiftRight: "ShiftRight",
  CapsLock: "CapsLock",
  F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
  F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
};

const LABELS: Record<string, string> = {
  MetaLeft: "Win",
  MetaRight: "Win",
  Alt: "Alt",
  AltGr: "Alt Gr",
  ShiftLeft: "Shift",
  ShiftRight: "Shift",
  CapsLock: "Bloq Mayús",
};

const label = computed(() => {
  if (trigger.value === "CapsLock") return t.value.capsLock;
  return LABELS[trigger.value] ?? trigger.value;
});

interface Link {
  label: string;
  url: string;
}

const links: Link[] = [
  { label: "GitHub", url: "https://github.com/gonnmerino/spaces" },
  { label: "X", url: "https://x.com/gonnmerino" },
];

const stars = Array.from({ length: 52 }, (_, i) => {
  const r = (n: number) => (i * n * 0.61803398875) % 1;
  return {
    id: i,
    x: `${(r(73) * 95 + 3).toFixed(1)}%`,
    y: `${(r(149) * 58 + 38).toFixed(1)}%`,
    size: r(31) > 0.78 ? 2 : 1,
    dur: +(2.2 + r(97) * 2.6).toFixed(1),
    delay: +(r(57) * 3.2).toFixed(1),
  };
});

async function openLink(url: string) {
  try {
    await openUrl(url);
  } catch (err) {
    error.value = String(err);
  }
}

onMounted(async () => {
  try {
    trigger.value = await invoke<string>("get_trigger");
  } catch (err) {
    error.value = String(err);
  }
});

onBeforeUnmount(stopListening);

async function onKey(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.repeat) return;

  stopListening();
  if (e.code === "Escape") return;

  const key = CODE_TO_KEY[e.code];
  if (!key) {
    error.value = t.value.keyError;
    return;
  }

  try {
    await invoke("set_trigger", { key });
    trigger.value = key;
  } catch (err) {
    error.value = String(err);
  }
}

function stopListening() {
  listening.value = false;
  window.removeEventListener("keydown", onKey, true);
}

function toggleListening() {
  if (listening.value) {
    stopListening();
    return;
  }
  error.value = "";
  listening.value = true;
  window.addEventListener("keydown", onKey, true);
}
</script>

<template>
  <main class="relative flex h-screen w-screen overflow-hidden bg-[#060608] text-[#f5f5f7] antialiased">
    <div class="pointer-events-none absolute inset-0" aria-hidden="true">
      <div
          class="absolute -left-24 -top-20 h-56 w-72 rounded-full bg-[radial-gradient(circle,rgba(190,35,55,0.28),transparent_70%)] blur-2xl"
      ></div>
      <span
          v-for="s in stars"
          :key="s.id"
          class="absolute rounded-full bg-white"
          :style="{
            left: s.x,
            top: s.y,
            width: s.size + 'px',
            height: s.size + 'px',
            animation: `twinkle ${s.dur}s ease-in-out ${s.delay}s infinite`,
          }"
      ></span>
    </div>

    <div class="relative flex flex-1 flex-col justify-between px-7 py-6">
      <header class="relative flex items-center gap-4">
        <img
            :src="logo"
            alt=""
            class="h-[76px] w-[76px] drop-shadow-[0_6px_24px_rgba(214,217,228,0.22)]"
        />
        <div>
          <h1 class="text-[1.2rem] font-semibold tracking-[0.28em] text-[#f5f5f7]">
            SPACES
          </h1>
          <p class="mt-0.5 text-[13px] tracking-wide text-[#86868b]">
            {{ t.subtitle }}
          </p>
        </div>
      </header>

      <p class="relative max-w-[38ch] text-[13.5px] leading-relaxed text-[#a1a1a6]">
        {{ t.description }}
      </p>

      <footer
          class="relative flex items-center gap-5 rounded-full border border-white/[0.07] bg-white/[0.035] px-5 py-2 backdrop-blur-md"
      >
        <a
            v-for="link in links"
            :key="link.label"
            href="#"
            class="text-[11px] font-medium uppercase tracking-[0.14em] text-[#86868b] transition-colors hover:text-[#f5f5f7] focus-visible:text-[#f5f5f7] focus-visible:outline-none"
            @click.prevent="openLink(link.url)"
        >
          {{ link.label }}
        </a>
        <span class="ml-auto flex items-center gap-1.5 text-[11px] font-medium tracking-wide text-[#6e6e73]">
          <span>v1.0</span>
          <span aria-hidden="true">·</span>
          <span>
            <template v-for="(part, i) in t.credit.split('♥')" :key="i">
              <span v-if="i > 0" class="text-[15px] leading-none text-[#d42a45]">♥</span>{{ part }}
            </template>
            <a
                href="#"
                class="ml-1 transition-colors hover:text-[#f5f5f7] focus-visible:text-[#f5f5f7] focus-visible:outline-none"
                title="gonnmerino"
                @click.prevent="openLink('https://github.com/gonnmerino')"
            >@gonnmerino</a>
          </span>
        </span>
      </footer>
    </div>

    <aside
        class="relative flex w-[264px] shrink-0 flex-col justify-between border-l border-white/[0.08] bg-[#0e0e11] px-6 py-6"
    >
      <div aria-hidden="true"></div>

      <div class="flex flex-col items-center gap-5">
        <div class="flex items-center gap-3.5" aria-live="polite">
        <kbd
            class="min-w-[76px] rounded-lg border px-4 py-2.5 text-center text-lg font-semibold tracking-wide transition-all duration-300"
            :class="
            listening
              ? 'animate-pulse border-white bg-white/[0.1] text-white shadow-[0_0_24px_rgba(255,255,255,0.2)] motion-reduce:animate-none'
              : 'border-white/[0.1] bg-white/[0.06] text-[#f5f5f7]'
            "
        >
          {{ listening ? "…" : label }}
        </kbd>

        <span class="text-[#48484d]">+</span>

        <svg
            class="h-9 w-[22px] fill-none stroke-[#86868b] stroke-2 transition-colors duration-300"
            :class="listening ? 'stroke-white' : ''"
            viewBox="0 0 24 36"
            stroke-linecap="round"
            role="img"
            aria-label="Rueda del mouse"
        >
          <rect x="2" y="2" width="20" height="32" rx="10" />
          <line x1="12" y1="8" x2="12" y2="14" />
        </svg>
      </div>

      <button
          class="cursor-pointer rounded-full px-7.5 py-2 text-[13px] font-medium transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-white"
          :class="
          listening
            ? 'border border-white/[0.12] bg-transparent text-[#a1a1a6] hover:text-[#f5f5f7]'
            : 'border border-transparent bg-[#f5f5f7] text-[#111114] hover:bg-white'
        "
          @click="toggleListening"
      >
        {{ listening ? t.cancel : t.changeKey }}
      </button>

      <p class="min-h-5 text-center text-[12.5px] text-[#86868b]">
        {{ listening ? t.hint : "" }}
      </p>
      <p v-if="error" class="-mt-3 max-w-[220px] text-center text-[12px] text-[#ff6961]" role="alert">
        {{ error }}
      </p>
      </div>

      <div ref="langRef" class="relative w-full">
        <button
            type="button"
            class="flex w-full cursor-pointer items-center justify-between rounded-lg border border-white/[0.1] bg-white/[0.06] px-3 py-[7px] text-[12.5px] text-[#a1a1a6] outline-none transition-colors hover:text-[#f5f5f7] focus-visible:border-white/40"
            aria-haspopup="listbox"
            :aria-expanded="langOpen"
            @click="langOpen = !langOpen"
        >
          <span>{{ localeLabel }}</span>
          <svg
              class="h-3.5 w-3.5 stroke-current stroke-2 transition-transform duration-200"
              :class="langOpen ? 'rotate-180' : ''"
              fill="none"
              viewBox="0 0 24 24"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
          >
            <path d="m6 9 6 6 6-6" />
          </svg>
        </button>

        <transition name="lang">
          <ul
              v-if="langOpen"
              role="listbox"
              class="absolute bottom-full left-0 z-20 mb-1.5 max-h-[93px] w-full overflow-y-auto rounded-lg border border-white/[0.1] bg-[#141418] shadow-[0_12px_32px_rgba(0,0,0,0.55)] backdrop-blur-xl"
          >
            <li v-for="l in LOCALES" :key="l.code" role="option" :aria-selected="l.code === locale">
              <button
                  type="button"
                  class="w-full cursor-pointer px-3 py-1.5 text-left text-[12.5px] transition-colors"
                  :class="
                  l.code === locale
                    ? 'bg-white/[0.07] text-white'
                    : 'text-[#86868b] hover:bg-white/[0.05] hover:text-[#f5f5f7]'
                "
                  @click="selectLocale(l.code)"
              >
                {{ l.label }}
              </button>
            </li>
          </ul>
        </transition>
      </div>
    </aside>
  </main>
</template>
