<template>
  <main class="relative h-screen select-none overflow-hidden text-ink" :data-theme="currentThemeKey" :style="appThemeStyle" @contextmenu.prevent @keydown.tab.prevent="switchPrimaryView">
    <section class="flex h-full w-full overflow-hidden border bg-panel transition" :class="mainWindowPinned ? 'border-[3px] border-rust shadow-[inset_0_0_0_1px_rgba(178,95,58,0.35),0_0_0_2px_rgba(178,95,58,0.25)]' : 'border-line'">
      <aside class="flex shrink-0 flex-col border-r border-line bg-sidebar transition-[width]" :class="sidebarCollapsed ? 'w-16' : 'w-56'">
        <div data-tauri-drag-region class="border-b border-line px-3 py-3" @pointerdown="startWindowDrag">
          <div class="flex items-center gap-3">
            <img :src="logoUrl" alt="" class="h-9 w-9 shrink-0 rounded-md" />
            <div v-if="!sidebarCollapsed" class="min-w-0">
              <h1 class="truncate text-base font-semibold leading-tight">SheepClip</h1>
              <p class="text-xs text-stone-600">智能剪贴板</p>
            </div>
          </div>
        </div>

        <nav data-guide="nav" class="no-drag flex-1 space-y-1 p-3">
          <button
            v-for="item in navItems"
            :key="item.key"
            type="button"
            class="flex h-10 w-full items-center gap-3 rounded-md px-3 text-left text-sm transition"
            :class="activeView === item.key ? 'bg-mint text-white' : 'text-muted hover:bg-white/70'"
            :title="item.label"
            @click="activeView = item.key"
          >
            <component :is="item.icon" class="h-4 w-4 shrink-0" />
            <span v-if="!sidebarCollapsed">{{ item.label }}</span>
          </button>
          <button
            type="button"
            class="flex h-10 w-full items-center gap-3 rounded-md px-3 text-left text-sm text-muted transition hover:bg-white/70"
            title="收起/展开"
            @click="manualSidebarCollapsed = !manualSidebarCollapsed"
          >
            <PanelLeftClose v-if="!sidebarCollapsed" class="h-4 w-4 shrink-0" />
            <PanelLeftOpen v-else class="h-4 w-4 shrink-0" />
            <span v-if="!sidebarCollapsed">收起菜单</span>
          </button>
        </nav>

        <div class="no-drag border-t border-line text-xs text-stone-600" :class="sidebarCollapsed ? 'px-2 py-3' : 'space-y-2 p-3'">
          <template v-if="sidebarCollapsed">
            <div class="flex flex-col items-center gap-2">
              <span
                v-for="hint in shortcutHints"
                :key="hint.label"
                class="flex h-7 w-9 items-center justify-center rounded-md bg-white/70 text-[11px] font-medium text-stone-600"
                :title="`${hint.label}：${hint.value}`"
              >
                {{ hint.short }}
              </span>
            </div>
          </template>
          <template v-else>
            <div v-for="hint in shortcutHints" :key="hint.label" class="flex h-6 items-center justify-between gap-3" :title="`${hint.label}：${hint.value}`">
              <span>{{ hint.label }}</span>
              <code class="shrink-0 rounded bg-white px-2 py-0.5 text-ink">{{ hint.value }}</code>
            </div>
          </template>
        </div>
      </aside>

      <section class="flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region class="flex h-11 shrink-0 items-center justify-between border-b border-line bg-header px-4" @pointerdown="startWindowDrag">
          <div class="min-w-0 text-sm font-medium text-muted">{{ activeViewTitle }}</div>
          <div class="no-drag flex items-center gap-1">
            <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md transition" :class="mainWindowPinned ? 'bg-rust text-white' : 'text-stone-600 hover:bg-white'" title="始终置顶" @click="toggleMainWindowPinned">
              <Pin class="h-4 w-4" />
            </button>
            <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md text-stone-600 hover:bg-white" title="最小化" @click="minimizeMainWindow">
              <Minus class="h-4 w-4" />
            </button>
            <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md text-stone-600 hover:bg-white" title="最大化/还原" @click="toggleMaximizeWindow">
              <Maximize2 class="h-4 w-4" />
            </button>
            <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md text-stone-600 hover:bg-rust hover:text-white" title="关闭" @click="requestCloseMainWindow">
              <X class="h-4 w-4" />
            </button>
          </div>
        </div>
        <header v-if="activeView === 'clipboard' || activeView === 'quick'" data-guide="search" class="border-b border-line bg-header px-5 py-4">
          <div class="flex items-center gap-3">
            <div class="no-drag relative flex-1">
              <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-stone-500" />
              <input
                ref="searchInput"
                v-model="query"
                class="h-10 w-full rounded-md border border-line bg-white px-10 text-sm outline-none transition focus:border-mint"
                placeholder="搜索剪贴板历史或快捷输入"
                @keydown.down.prevent="moveSelection(1)"
                @keydown.up.prevent="moveSelection(-1)"
                @keydown.enter.prevent="confirmSelected"
              />
            </div>
          </div>
        </header>

        <div class="min-h-0 flex-1 overflow-hidden">
          <Transition name="view-slide" mode="out-in">
          <section v-if="activeView === 'clipboard'" key="clipboard" class="flex h-full flex-col">
            <TransitionGroup
              ref="clipboardListRef"
              tag="div"
              name="list-motion"
              data-guide="clipboard-list"
              class="scroll-thin min-h-0 flex-1 overflow-y-auto p-3"
            >
              <div
                v-for="(item, index) in clipboardItems"
                :key="`clip-${item.id}`"
                :data-entry-key="itemKey('clipboard', item.id)"
                :data-guide="index === 0 ? 'clipboard-first-item' : undefined"
                class="mb-1 grid min-h-12 w-full grid-cols-[auto_1fr_auto_auto] items-center gap-2 rounded-md border px-3 text-left transition"
                :class="selectedKey === itemKey('clipboard', item.id) ? 'border-primaryMuted bg-primarySoft text-ink shadow-sm ring-2 ring-primaryMuted' : 'border-transparent bg-card hover:bg-primarySoft'"
                @click="selectItem('clipboard', item.id)"
                @dblclick="copyClipboard(item)"
                @contextmenu.prevent="showClipboardDetail(item)"
              >
                <span class="flex h-8 w-8 items-center justify-center rounded-md bg-white/80 text-stone-500">
                  <component :is="clipboardKindIcon(item.kind)" class="h-4 w-4" />
                </span>
                <span class="min-w-0">
                  <span class="block truncate text-sm">{{ clipboardDisplayText(item) }}</span>
                  <span class="mt-0.5 block truncate text-xs text-stone-500">
                    {{ kindLabel(item.kind) }}
                    <template v-if="item.copy_count > 1"> · 复制 {{ item.copy_count }} 次</template>
                  </span>
                </span>
                <span class="flex items-center gap-2">
                  <img v-if="clipboardThumbnail(item)" :src="clipboardThumbnail(item) || ''" alt="" class="h-8 w-10 rounded border border-line object-cover" />
                  <span class="text-xs text-stone-500">{{ formatTime(item.created_at) }}</span>
                </span>
                <span class="flex gap-1">
                  <button type="button" class="flex h-7 w-7 items-center justify-center rounded-md border border-line bg-white text-stone-700 hover:border-mint hover:text-mint" title="复制" @click.stop="copyClipboard(item)">
                    <Copy class="h-3.5 w-3.5" />
                  </button>
                  <button type="button" class="flex h-7 w-7 items-center justify-center rounded-md border border-line bg-white text-stone-700 hover:border-mint hover:text-mint" title="详情" @click.stop="showClipboardDetail(item)">
                    <Info class="h-3.5 w-3.5" />
                  </button>
                  <button type="button" class="flex h-7 w-7 items-center justify-center rounded-md border border-line bg-white text-stone-700 hover:border-rust hover:text-rust" title="删除" @click.stop="deleteClipboard(item)">
                    <Trash2 class="h-3.5 w-3.5" />
                  </button>
                </span>
                <span class="sr-only">{{ index }}</span>
              </div>
              <EmptyState v-if="clipboardItems.length === 0" key="clipboard-empty" label="暂无剪贴板历史" />
            </TransitionGroup>
          </section>

          <section v-else-if="activeView === 'quick'" key="quick" data-guide="quick-view" class="grid h-full grid-cols-[1fr_320px]">
            <div class="flex min-h-0 flex-col">
              <TransitionGroup tag="div" name="list-motion" data-guide="quick-list" class="scroll-thin min-h-0 flex-1 overflow-y-auto p-4">
                <div
                v-for="item in quickInputs"
                :key="`quick-${item.id}`"
                class="relative mb-2"
              >
                <div v-if="showQuickDropLine(item.id, 'before')" class="pointer-events-none absolute -top-1 left-2 right-2 z-10 h-0.5 rounded-full bg-mint shadow-[0_0_0_2px_rgba(47,143,113,0.18)]" />
                <div
                  role="button"
                  tabindex="0"
                  :data-entry-key="itemKey('quick', item.id)"
                  :data-quick-id="item.id"
                  class="grid w-full cursor-grab grid-cols-[auto_1fr] items-center gap-3 rounded-md border px-3 py-2 text-left transition duration-150 active:cursor-grabbing"
                  :class="[
                    selectedKey === itemKey('quick', item.id) ? 'border-primaryMuted bg-primarySoft text-ink shadow-sm ring-2 ring-primaryMuted' : 'border-transparent bg-card hover:bg-primarySoft',
                    dragQuickId === item.id ? 'scale-[0.99] opacity-70 shadow-soft ring-2 ring-rust/50' : '',
                    dragQuickId && dragQuickTargetId === item.id && dragQuickId !== item.id ? 'translate-y-0.5' : ''
                  ]"
                  @click="selectItem('quick', item.id)"
                  @keydown.enter.prevent="copyQuick(item)"
                  @dblclick="copyQuick(item)"
                  @contextmenu.prevent="showQuickDetail(item)"
                  @pointerdown="startQuickPointerDrag(item.id, $event)"
                  @pointerenter="hoverQuickDropTarget(item.id)"
                  @pointercancel="cancelQuickDrag"
                >
                  <GripVertical class="h-4 w-4 shrink-0 text-stone-400" />
                  <span class="min-w-0">
                    <span class="block truncate text-sm">{{ item.content }}</span>
                    <span class="mt-1 flex flex-wrap gap-1">
                      <span v-for="tag in item.tags" :key="tag" class="rounded bg-sidebar px-1.5 py-0.5 text-xs text-stone-600">{{ tag }}</span>
                    </span>
                  </span>
                </div>
                <div v-if="showQuickDropLine(item.id, 'after')" class="pointer-events-none absolute -bottom-1 left-2 right-2 z-10 h-0.5 rounded-full bg-mint shadow-[0_0_0_2px_rgba(47,143,113,0.18)]" />
                </div>
                <EmptyState v-if="quickInputs.length === 0" key="quick-empty" label="暂无快捷输入" />
              </TransitionGroup>
              <div class="border-t border-line bg-header p-3">
                <button type="button" class="flex h-9 w-full items-center justify-center gap-2 rounded-md bg-mint px-3 text-sm text-white" @click="beginCreateQuickInput">
                  <Plus class="h-4 w-4" />
                  新增快捷输入
                </button>
              </div>
            </div>

            <form class="border-l border-line bg-header p-4" @submit.prevent="submitQuickInput">
              <h2 class="text-sm font-semibold">{{ quickForm.id ? '编辑快捷输入' : '新增快捷输入' }}</h2>
              <label class="mt-4 block text-xs font-medium text-stone-600">内容</label>
              <textarea v-model="quickForm.content" class="mt-1 h-28 w-full resize-none rounded-md border border-line bg-white p-3 text-sm outline-none focus:border-mint" />

              <label class="mt-3 block text-xs font-medium text-stone-600">标签</label>
              <div class="mt-2 flex flex-wrap gap-2">
                <label
                  v-for="tag in tags"
                  :key="tag"
                  class="flex h-7 items-center gap-1 rounded-md border border-line bg-white px-2 text-xs text-stone-700"
                >
                  <input v-model="selectedQuickTags" type="checkbox" :value="tag" class="accent-mint" />
                  {{ tag }}
                </label>
              </div>

              <div class="mt-4">
                <button type="submit" class="flex h-9 w-full items-center justify-center gap-2 rounded-md bg-mint px-3 text-sm text-white">
                  <Save class="h-4 w-4" />
                  保存
                </button>
              </div>

              <button
                v-if="quickForm.id"
                type="button"
                class="mt-2 flex h-9 w-full items-center justify-center gap-2 rounded-md border border-line bg-white px-3 text-sm text-rust"
                @click="removeQuickInput"
              >
                <Trash2 class="h-4 w-4" />
                删除
              </button>
            </form>
          </section>

          <section v-else-if="activeView === 'theme'" key="theme" class="scroll-thin h-full overflow-y-auto p-5">
            <div class="max-w-4xl">
              <div>
                <h2 class="text-sm font-semibold">主题</h2>
                <p class="mt-1 text-xs text-muted">选择内置颜色和本机可用字体，切换后会自动保存。</p>
              </div>

              <section class="mt-5">
                <h3 class="text-xs font-semibold text-muted">颜色主题</h3>
                <div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                  <button
                    v-for="theme in THEME_OPTIONS"
                    :key="theme.key"
                    type="button"
                    class="group rounded-md border bg-card p-3 text-left transition hover:-translate-y-0.5 hover:shadow-soft"
                    :class="currentThemeKey === theme.key ? 'border-mint ring-2 ring-primaryMuted' : 'border-line'"
                    @click="selectTheme(theme.key)"
                  >
                    <div class="overflow-hidden rounded-md border border-line" :style="{ backgroundColor: theme.colors.panel }">
                      <div class="flex h-24">
                        <div class="w-16 border-r" :style="{ backgroundColor: theme.colors.sidebar, borderColor: theme.colors.line }">
                          <div class="m-2 h-4 rounded" :style="{ backgroundColor: theme.colors.primary }" />
                          <div class="mx-2 mt-2 h-3 rounded" :style="{ backgroundColor: theme.colors.card, opacity: 0.72 }" />
                          <div class="mx-2 mt-1 h-3 rounded" :style="{ backgroundColor: theme.colors.card, opacity: 0.48 }" />
                        </div>
                        <div class="min-w-0 flex-1">
                          <div class="h-6 border-b" :style="{ backgroundColor: theme.colors.header, borderColor: theme.colors.line }" />
                          <div class="space-y-1.5 p-2">
                            <div class="h-4 rounded" :style="{ backgroundColor: theme.colors.primarySoft, border: `1px solid ${theme.colors.primaryMuted}` }" />
                            <div class="h-4 rounded" :style="{ backgroundColor: theme.colors.card, opacity: 0.82 }" />
                            <div class="flex gap-1">
                              <div class="h-4 flex-1 rounded" :style="{ backgroundColor: theme.colors.card, opacity: 0.82 }" />
                              <div class="h-4 w-8 rounded" :style="{ backgroundColor: theme.colors.primary }" />
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                    <div class="mt-3 flex items-start justify-between gap-3">
                      <span>
                        <span class="block text-sm font-medium">{{ theme.name }}</span>
                        <span class="mt-1 block text-xs leading-5 text-muted">{{ theme.description }}</span>
                      </span>
                      <Check v-if="currentThemeKey === theme.key" class="mt-0.5 h-4 w-4 shrink-0 text-mint" />
                    </div>
                  </button>
                </div>
              </section>

              <section class="mt-6">
                <h3 class="text-xs font-semibold text-muted">文字字体</h3>
                <div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                  <button
                    v-for="font in availableFontOptions"
                    :key="font.key"
                    type="button"
                    class="rounded-md border bg-card p-3 text-left transition hover:-translate-y-0.5 hover:shadow-soft"
                    :class="currentFontKey === font.key ? 'border-mint ring-2 ring-primaryMuted' : 'border-line'"
                    :style="{ fontFamily: font.family }"
                    @click="selectFont(font.key)"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <span class="text-sm font-medium">{{ font.name }}</span>
                      <Check v-if="currentFontKey === font.key" class="h-4 w-4 text-mint" />
                    </div>
                    <p class="mt-3 text-lg leading-7">{{ font.sample }}</p>
                    <p class="mt-1 text-xs text-muted">SheepClip 0123 ABC</p>
                  </button>
                </div>
              </section>

              <section class="mt-6">
                <h3 class="text-xs font-semibold text-muted">文字显示</h3>
                <div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2">
                  <div class="rounded-md border border-line bg-card p-3">
                    <div class="flex items-center justify-between gap-3">
                      <span class="text-sm font-medium">字号大小</span>
                      <span class="rounded bg-sidebar px-2 py-1 text-xs text-muted">{{ currentFontSize }}px</span>
                    </div>
                    <input
                      v-model.number="settings.font_size"
                      type="range"
                      :min="MIN_FONT_SIZE"
                      :max="MAX_FONT_SIZE"
                      step="1"
                      class="mt-4 w-full accent-mint"
                      @change="settings.font_size = resolveFontSize(settings.font_size)"
                    />
                    <div class="mt-2 flex justify-between text-xs text-muted">
                      <span>{{ MIN_FONT_SIZE }}px</span>
                      <span>{{ MAX_FONT_SIZE }}px</span>
                    </div>
                  </div>

                  <div class="rounded-md border border-line bg-card p-3">
                    <div class="flex items-center justify-between gap-3">
                      <span class="text-sm font-medium">字体粗细</span>
                      <span class="rounded bg-sidebar px-2 py-1 text-xs text-muted">{{ currentFontWeight }}</span>
                    </div>
                    <div class="mt-4 grid grid-cols-3 gap-2 rounded-md bg-sidebar p-1">
                      <button
                        v-for="weight in FONT_WEIGHT_OPTIONS"
                        :key="weight"
                        type="button"
                        class="h-8 rounded text-xs transition"
                        :class="currentFontWeight === weight ? 'bg-card text-ink shadow-sm' : 'text-muted'"
                        :style="{ fontWeight: weight }"
                        @click="settings.font_weight = weight"
                      >
                        {{ fontWeightLabel(weight) }}
                      </button>
                    </div>
                  </div>
                </div>
              </section>
            </div>
          </section>

          <section v-else-if="activeView === 'settings'" key="settings" data-guide="settings-view" data-guide-scroll class="scroll-thin h-full overflow-y-auto p-5">
            <div class="max-w-2xl">
              <h2 class="text-sm font-semibold">设置</h2>
              <div class="mt-4 space-y-4">
                <div class="flex items-center justify-between rounded-md border border-line bg-white px-3 py-3">
                  <span>
                    <span class="block text-sm font-medium">自动启动</span>
                    <span class="block text-xs text-stone-600">开机登录 Windows 后自动启动 SheepClip，并默认进入后台。</span>
                  </span>
                  <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.launch_at_startup ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.launch_at_startup" @click="settings.launch_at_startup = !settings.launch_at_startup">
                    <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.launch_at_startup ? 'left-5' : 'left-0.5'" />
                  </button>
                </div>
                <div class="flex items-center justify-between rounded-md border border-line bg-white px-3 py-3">
                  <span>
                    <span class="block text-sm font-medium">管理员身份启动</span>
                    <span class="block text-xs text-stone-600">开机自启时以管理员权限运行；开启时系统可能弹出授权确认。</span>
                  </span>
                  <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.launch_as_admin ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.launch_as_admin" @click="settings.launch_as_admin = !settings.launch_as_admin">
                    <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.launch_as_admin ? 'left-5' : 'left-0.5'" />
                  </button>
                </div>
                <div class="flex items-center justify-between rounded-md border border-line bg-white px-3 py-3">
                  <span>
                    <span class="block text-sm font-medium">退出时二次确认</span>
                    <span class="block text-xs text-stone-600">点击退出应用或托盘退出时先弹出确认，避免误关后台监听。</span>
                  </span>
                  <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.confirm_exit ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.confirm_exit" @click="settings.confirm_exit = !settings.confirm_exit">
                    <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.confirm_exit ? 'left-5' : 'left-0.5'" />
                  </button>
                </div>
                <label class="block rounded-md border border-line bg-white p-3">
                  <span class="block text-sm font-medium">条目历史数量上限</span>
                  <span class="block text-xs text-stone-600">控制本软件保存的历史数量，超过上限会自动删除较早记录。</span>
                  <input v-model.number="settings.history_limit" type="number" min="50" max="10000" class="mt-3 h-9 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-mint" />
                </label>
                <div class="rounded-md border border-line bg-white p-3">
                  <div class="flex items-center justify-between gap-4">
                    <span>
                      <span class="block text-sm font-medium">主窗口双击键</span>
                      <span class="block text-xs text-stone-600">连续快速按两次该按键后唤起主窗口。</span>
                    </span>
                    <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.main_hotkey_enabled ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.main_hotkey_enabled" @click="settings.main_hotkey_enabled = !settings.main_hotkey_enabled">
                      <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.main_hotkey_enabled ? 'left-5' : 'left-0.5'" />
                    </button>
                  </div>
                  <select v-model="settings.main_hotkey" :disabled="!settings.main_hotkey_enabled" class="mt-3 h-9 w-full rounded-md border border-line bg-white px-3 text-sm outline-none transition focus:border-mint disabled:bg-stone-100 disabled:text-stone-400">
                    <option value="Alt">Alt</option>
                    <option value="Ctrl">Ctrl</option>
                  </select>
                </div>
                <div class="rounded-md border border-line bg-white p-3">
                  <div class="flex items-center justify-between gap-4">
                    <span>
                      <span class="block text-sm font-medium">行内触发符</span>
                      <span class="block text-xs text-stone-600">在其它输入框中快速输入 // 后打开快捷输入弹窗。</span>
                    </span>
                    <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.inline_trigger_enabled ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.inline_trigger_enabled" @click="settings.inline_trigger_enabled = !settings.inline_trigger_enabled">
                      <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.inline_trigger_enabled ? 'left-5' : 'left-0.5'" />
                    </button>
                  </div>
                  <input :value="settings.inline_trigger" readonly class="mt-3 h-9 w-full cursor-default rounded-md border border-line bg-stone-100 px-3 text-sm text-stone-600 outline-none" />
                </div>
                <div data-guide="tray-settings" class="space-y-3 rounded-lg">
                  <div class="flex items-center justify-between rounded-md border border-line bg-white px-3 py-3">
                    <span>
                      <span class="block text-sm font-medium">自动隐藏至托盘</span>
                      <span class="block text-xs text-stone-600">开启后，主窗口关闭或自动关闭时会缩小至托盘；关闭时会显示在任务栏。</span>
                    </span>
                    <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.auto_hide_to_tray ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.auto_hide_to_tray" @click="settings.auto_hide_to_tray = !settings.auto_hide_to_tray">
                      <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.auto_hide_to_tray ? 'left-5' : 'left-0.5'" />
                    </button>
                  </div>
                  <div class="flex items-center justify-between rounded-md border border-line bg-white px-3 py-3">
                    <span>
                      <span class="block text-sm font-medium">不聚焦时关闭</span>
                      <span class="block text-xs text-stone-600">切换到其它应用时自动关闭主窗口；配合“自动隐藏至托盘”可让窗口缩小至托盘，之后双击 Alt 唤起。</span>
                    </span>
                    <button type="button" class="relative h-6 w-11 rounded-full transition" :class="settings.hide_on_blur ? 'bg-mint' : 'bg-stone-300'" role="switch" :aria-checked="settings.hide_on_blur" @click="settings.hide_on_blur = !settings.hide_on_blur">
                      <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition" :class="settings.hide_on_blur ? 'left-5' : 'left-0.5'" />
                    </button>
                  </div>
                </div>
                <div class="rounded-md border border-line bg-white p-3">
                  <span class="block text-sm font-medium">条目激活后（鼠标双击或回车）</span>
                  <span class="block text-xs text-stone-600">激活条目会先复制到系统剪贴板，再按下面选项执行后续动作。</span>
                  <div class="mt-3 space-y-2">
                    <SettingSwitch label="移动条目到顶端" description="成功复制后把该条历史或短语移动到列表顶部，方便下次继续使用。" :model-value="settings.move_activated_to_top" @update:model-value="settings.move_activated_to_top = $event" />
                    <SettingSwitch label="关闭主窗口" description="成功复制后关闭主窗口；如果已开启始终置顶，则不会自动关闭。" :model-value="settings.close_after_activation" @update:model-value="settings.close_after_activation = $event" />
                    <SettingSwitch label="聚焦上一窗口" description="成功复制后尝试回到唤起 SheepClip 前正在使用的窗口。" :model-value="settings.focus_previous_after_activation" @update:model-value="settings.focus_previous_after_activation = $event" />
                    <SettingSwitch label="粘贴到当前窗口" description="成功复制后尝试自动发送 Ctrl+V；关闭时只复制，不自动粘贴。" :model-value="settings.paste_after_activation" @update:model-value="settings.paste_after_activation = $event" />
                  </div>
                </div>
                <div class="rounded-md border border-line bg-white p-3">
                  <div class="flex items-center justify-between">
                    <div>
                      <span class="block text-sm font-medium">快捷输入标签</span>
                      <span class="block text-xs text-stone-600">给快捷短语分类，输入 // 后可在弹窗里按标签筛选。</span>
                    </div>
                  </div>
                  <div class="mt-3 flex flex-wrap gap-2">
                    <span v-for="tag in tags" :key="tag" class="flex h-8 items-center gap-1 rounded-md bg-sidebar px-2 text-xs">
                      {{ tag }}
                      <button type="button" class="text-stone-500 hover:text-rust" title="删除标签" @click="removeTag(tag)">
                        <X class="h-3 w-3" />
                      </button>
                    </span>
                  </div>
                  <div class="mt-3 flex gap-2">
                    <input v-model="newTagName" class="h-9 flex-1 rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-mint" placeholder="新增标签，如 ops" @keydown.enter.prevent="createTag" />
                    <button type="button" class="flex h-9 items-center gap-2 rounded-md bg-mint px-3 text-sm text-white" @click="createTag">
                      <Plus class="h-4 w-4" />
                      添加
                    </button>
                  </div>
                </div>

                <div class="rounded-md border border-rust/30 bg-white p-3">
                  <div class="flex items-center justify-between gap-4">
                    <div>
                      <span class="block text-sm font-medium text-rust">清空剪贴板历史</span>
                      <span class="block text-xs text-stone-600">只清空本软件记录，不清空系统当前剪贴板。</span>
                    </div>
                    <button type="button" class="flex h-9 items-center gap-2 rounded-md border border-rust px-3 text-sm text-rust" @click="requestClearHistory">
                      <Trash2 class="h-4 w-4" />
                      清空
                    </button>
                  </div>
                </div>
                <div class="rounded-md border border-line bg-white p-3">
                  <div class="flex items-center justify-between gap-4">
                    <div>
                      <span class="block text-sm font-medium">重置全部设置</span>
                      <span class="block text-xs text-stone-600">恢复推荐设置，并重新显示新手引导；不会删除剪贴板历史和快捷短语。</span>
                    </div>
                    <button type="button" class="flex h-9 items-center gap-2 rounded-md border border-line bg-white px-3 text-sm text-stone-700 hover:border-mint hover:text-mint" @click="requestResetSettings">
                      <RotateCcw class="h-4 w-4" />
                      重置
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section v-else key="about" data-guide="about-view" class="scroll-thin h-full overflow-y-auto p-5">
            <div class="max-w-2xl">
              <h2 class="text-sm font-semibold">关于软件</h2>
              <div class="mt-4 rounded-md border border-line bg-white p-4">
                <div class="flex items-center gap-3">
                  <img :src="logoUrl" alt="" class="h-12 w-12 rounded-md" />
                  <div>
                    <h3 class="text-base font-semibold">SheepClip</h3>
                    <p class="text-xs text-stone-600">面向 Windows 的剪贴板历史和快捷输入工具</p>
                  </div>
                </div>
                <dl class="mt-5 space-y-3 text-sm">
                  <div class="flex items-center justify-between gap-4 border-t border-line pt-3">
                    <dt class="text-stone-500">软件版本</dt>
                    <dd class="font-medium">0.20</dd>
                  </div>
                  <div class="flex items-center justify-between gap-4 border-t border-line pt-3">
                    <dt class="text-stone-500">开发者</dt>
                    <dd class="font-medium">PASSHEEP</dd>
                  </div>
                  <div class="flex items-center justify-between gap-4 border-t border-line pt-3">
                    <dt class="text-stone-500">联系开发者</dt>
                    <dd class="font-medium">QQ：903081605</dd>
                  </div>
                  <div class="flex items-center justify-between gap-4 border-t border-line pt-3">
                    <dt class="text-stone-500">Git 地址</dt>
                    <dd class="min-w-0">
                      <button type="button" class="inline-flex max-w-full items-center gap-2 rounded-md border border-line bg-header px-3 py-2 text-left text-sm text-mint hover:border-mint" @click="openGitRepository">
                        <ExternalLink class="h-4 w-4 shrink-0" />
                        <span class="truncate">https://github.com/passheep/SheepClip</span>
                      </button>
                    </dd>
                  </div>
                </dl>
              </div>
            </div>
          </section>
          </Transition>
        </div>
        <footer class="flex h-10 items-center border-t border-line bg-header px-5 text-xs text-stone-600">
          <span>{{ statusText }}</span>
        </footer>
      </section>
    </section>

    <div v-if="detailItem" class="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4" @click.self="closeDetail">
      <section class="flex max-h-[calc(100vh-2rem)] w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-line bg-panel shadow-soft">
        <header class="flex items-center justify-between border-b border-line px-4 py-3">
          <div>
            <h2 class="text-sm font-semibold">{{ detailKind === 'quick' ? '快捷输入详情' : '剪贴板详情' }}</h2>
            <p v-if="detailKind === 'clipboard'" class="mt-1 text-xs text-stone-500">
              类型：{{ detailClipboardItem ? kindLabel(detailClipboardItem.kind) : '文本' }}
              <span class="mx-1">·</span>
              复制时间：{{ formatDateTime(detailItem.created_at) }}
              <template v-if="detailClipboardItem && detailClipboardItem.copy_count > 1">
                <span class="mx-1">·</span>
                合并复制：{{ detailClipboardItem.copy_count }} 次
              </template>
            </p>
            <p v-if="detailKind === 'clipboard'" class="mt-1 max-w-xl truncate text-xs text-stone-500">
              来源：{{ detailClipboardItem?.source_app || '未知来源' }}
            </p>
          </div>
          <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md border border-line bg-white" title="关闭" @click="closeDetail">
            <X class="h-4 w-4" />
          </button>
        </header>
        <div v-if="detailClipboardItem?.kind === 'image'" class="scroll-thin min-h-0 flex-1 overflow-auto bg-white p-4">
          <img :src="detailClipboardItem.content" alt="" class="max-h-[55vh] w-full rounded-md border border-line object-contain" />
        </div>
        <div
          v-else-if="detailClipboardItem?.kind === 'rich_text'"
          ref="detailRichTextRef"
          class="rich-text-preview scroll-thin min-h-[42vh] flex-1 overflow-auto bg-white p-4"
          v-html="detailHtml"
        />
        <textarea v-else ref="detailTextRef" class="min-h-[42vh] flex-1 resize-none bg-white p-4 text-sm outline-none" :value="detailText" readonly />
        <footer class="flex flex-wrap items-center justify-between gap-2 border-t border-line p-3">
          <span class="text-xs text-stone-500">{{ detailMetaText }}</span>
          <span class="flex flex-wrap justify-end gap-2">
            <button type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm" @click="copyDetailItem">复制</button>
            <button v-if="detailKind === 'clipboard'" type="button" class="h-9 rounded-md border border-rust bg-white px-3 text-sm text-rust" @click="deleteDetailItem">删除</button>
            <button v-if="detailTextSelectable" type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm" @click="selectAllDetail">全选</button>
            <button v-if="canAddDetailToQuickInput" type="button" class="h-9 rounded-md bg-mint px-3 text-sm text-white" @click="addDetailToQuickInput">添加到快捷输入</button>
          </span>
        </footer>
      </section>
    </div>

    <div v-if="clearConfirmVisible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4" @click.self="clearConfirmVisible = false">
      <section class="w-full max-w-sm rounded-lg border border-line bg-panel p-4 shadow-soft">
        <h2 class="text-sm font-semibold text-rust">确认清空历史？</h2>
        <p class="mt-2 text-sm text-stone-600">此操作会删除 SheepClip 保存的所有剪贴板历史记录，无法撤销。</p>
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm" @click="clearConfirmVisible = false">取消</button>
          <button type="button" class="h-9 rounded-md bg-rust px-3 text-sm text-white" @click="confirmClearHistory">确认清空</button>
        </div>
      </section>
    </div>

    <div v-if="resetConfirmVisible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4" @click.self="resetConfirmVisible = false">
      <section class="w-full max-w-sm rounded-lg border border-line bg-panel p-4 shadow-soft">
        <h2 class="text-sm font-semibold">重置全部设置？</h2>
        <p class="mt-2 text-sm text-stone-600">会恢复推荐设置并重新显示新手引导；已有剪贴板历史和快捷短语不会被删除。</p>
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm" @click="resetConfirmVisible = false">取消</button>
          <button type="button" class="h-9 rounded-md bg-mint px-3 text-sm text-white disabled:cursor-not-allowed disabled:opacity-60" :disabled="resetBusy" @click="confirmResetSettings">
            {{ resetBusy ? '重置中...' : '确认重置' }}
          </button>
        </div>
      </section>
    </div>

    <div v-if="closeConfirmVisible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4" @click.self="closeConfirmVisible = false">
      <section class="w-full max-w-sm rounded-lg border border-line bg-panel p-4 shadow-soft">
        <h2 class="text-sm font-semibold">关闭 SheepClip？</h2>
        <p class="mt-2 text-sm text-stone-600">放到托盘后仍会继续监听剪贴板；退出应用会停止所有快捷键和剪贴板监听。</p>
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm text-rust" @click="confirmExitApp">退出应用</button>
          <button type="button" class="h-9 rounded-md bg-mint px-3 text-sm text-white" @click="confirmCloseToTray">放到托盘</button>
        </div>
      </section>
    </div>
    <div v-if="showOnboarding && currentGuideStep" class="fixed inset-0 z-[100]">
      <div v-if="!guideHighlight" class="pointer-events-none fixed inset-0 z-[100] bg-black/55" />
      <div
        v-if="guideHighlight"
        class="guide-hole pointer-events-none fixed z-[101]"
        :style="guideHighlightStyle"
      />
      <section
        class="guide-card pointer-events-auto fixed z-[102] w-[min(22rem,calc(100vw-2rem))] rounded-lg border border-line bg-panel p-4 shadow-soft"
        :style="guideCardStyle"
      >
        <div class="flex items-start justify-between gap-3">
          <div>
            <p class="text-xs font-medium text-mint">{{ guideStepText }}</p>
            <h2 class="mt-1 text-base font-semibold">{{ currentGuideStep.title }}</h2>
          </div>
          <button type="button" class="flex h-8 w-8 items-center justify-center rounded-md border border-line bg-white text-stone-600" title="跳过引导" @click="finishOnboarding">
            <X class="h-4 w-4" />
          </button>
        </div>
        <p class="mt-3 text-sm leading-6 text-stone-700">{{ currentGuideStep.description }}</p>
        <div class="mt-4 flex items-center justify-between gap-3">
          <button type="button" class="h-9 rounded-md border border-line bg-white px-3 text-sm" :disabled="guideIndex === 0" @click="prevGuideStep">上一步</button>
          <div class="flex gap-1">
            <span v-for="(_, index) in guideSteps" :key="index" class="h-1.5 w-5 rounded-full transition" :class="index === guideIndex ? 'bg-mint' : 'bg-stone-300'" />
          </div>
          <button type="button" class="h-9 rounded-md bg-mint px-3 text-sm text-white" @click="nextGuideStep">{{ guideIndex === guideSteps.length - 1 ? '完成' : '下一步' }}</button>
        </div>
      </section>
    </div>
    <div class="absolute bottom-0 right-0 h-4 w-4 cursor-nwse-resize" title="调整大小" @pointerdown.stop.prevent="startWindowResize">
      <div class="absolute bottom-1 right-1 h-2 w-2 border-b border-r border-stone-400" />
    </div>
  </main>
</template>

<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, defineComponent, h, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import {
  Check,
  ClipboardList,
  Cog,
  Copy,
  ExternalLink,
  File,
  FileText,
  GripVertical,
  Info,
  Image as ImageIcon,
  Maximize2,
  Minus,
  Palette,
  PanelLeftClose,
  PanelLeftOpen,
  Pin,
  Plus,
  RotateCcw,
  Save,
  Search,
  Trash2,
  Type,
  X,
  Zap,
} from 'lucide-vue-next';
import logoUrl from './assets/clipboard.png';
import type { AppSettings, ClipboardItem, QuickInput } from './types';
import {
  addClipboardItemToQuickInput,
  addTag,
  clearClipboardHistory,
  copyClipboardItem,
  copyQuickInput,
  deleteClipboardItem,
  deleteQuickInput,
  deleteTag,
  exitApp,
  getSettings,
  hideMainWindow,
  listClipboardItems,
  listQuickInputs,
  listTags,
  minimizeMainWindow,
  markMainPointerOperation,
  openExternalUrl,
  reorderQuickInputs,
  resetSettings,
  saveQuickInput,
  saveSettings,
  setMainWindowAlwaysOnTop,
} from './lib/commands';
import { restoreAndTrackWindowSize } from './lib/windowSize';
import {
  FONT_OPTIONS,
  FONT_WEIGHT_OPTIONS,
  MAX_FONT_SIZE,
  MIN_FONT_SIZE,
  THEME_OPTIONS,
  getAvailableFontOptions,
  getThemeStyle,
  resolveFontSize,
  resolveFontKey,
  resolveFontWeight,
  resolveThemeKey,
  type FontKey,
  type FontOption,
  type ThemeKey,
} from './theme';

type ViewKey = 'clipboard' | 'quick' | 'theme' | 'settings' | 'about';
type SourceKey = 'clipboard' | 'quick';
type GuidePlacement = 'right' | 'bottom' | 'left' | 'top';
type GuideStep = {
  selector: string;
  fallbackSelector?: string;
  title: string;
  description: string;
  view: ViewKey;
  placement: GuidePlacement;
};

const navItems = [
  { key: 'clipboard' as const, label: '剪贴板历史', icon: ClipboardList },
  { key: 'quick' as const, label: '快捷输入', icon: Zap },
  { key: 'theme' as const, label: '主题', icon: Palette },
  { key: 'settings' as const, label: '设置', icon: Cog },
  { key: 'about' as const, label: '关于软件', icon: Info },
];

const activeView = ref<ViewKey>('clipboard');
const autoSidebarCollapsed = ref(false);
const manualSidebarCollapsed = ref(false);
const query = ref('');
const selectedKey = ref('');
const clipboardItems = ref<ClipboardItem[]>([]);
const quickInputs = ref<QuickInput[]>([]);
const tags = ref<string[]>([]);
const statusText = ref('准备就绪');
const searchInput = ref<HTMLInputElement | null>(null);
const clipboardListRef = ref<HTMLElement | null>(null);
const detailTextRef = ref<HTMLTextAreaElement | null>(null);
const detailRichTextRef = ref<HTMLElement | null>(null);
const selectedQuickTags = ref<string[]>([]);
const newTagName = ref('');
const detailItem = ref<ClipboardItem | QuickInput | null>(null);
const detailKind = ref<SourceKey>('clipboard');
const clearConfirmVisible = ref(false);
const resetConfirmVisible = ref(false);
const resetBusy = ref(false);
const closeConfirmVisible = ref(false);
const mainWindowPinned = ref(false);
const dragQuickId = ref<number | null>(null);
const dragQuickTargetId = ref<number | null>(null);
const guideIndex = ref(0);
const guideHighlight = ref<DOMRect | null>(null);
const availableFontOptions = ref<FontOption[]>([FONT_OPTIONS[0]]);
let quickDragStarted = false;
let quickDragStartX = 0;
let quickDragStartY = 0;
let guideMeasureTimer = 0;
let guideStepTimers: number[] = [];

const settings = reactive<AppSettings>({
  history_limit: 2000,
  theme_key: 'warm',
  font_key: 'system',
  font_size: 14,
  font_weight: 400,
  main_hotkey: 'Alt',
  main_hotkey_enabled: true,
  inline_trigger: '//',
  inline_trigger_enabled: true,
  launch_at_startup: false,
  launch_as_admin: false,
  auto_hide_to_tray: false,
  confirm_close_to_tray: true,
  enter_paste_to_active: false,
  hide_on_blur: false,
  confirm_exit: true,
  move_activated_to_top: true,
  close_after_activation: true,
  focus_previous_after_activation: true,
  paste_after_activation: false,
  onboarding_completed: false,
});

const guideSteps: GuideStep[] = [
  {
    selector: '[data-guide="nav"]',
    title: '切换功能区',
    description: '左侧可以在剪贴板历史、快捷输入和设置之间切换。',
    view: 'clipboard' as ViewKey,
    placement: 'right' as GuidePlacement,
  },
  {
    selector: '[data-guide="clipboard-first-item"]',
    fallbackSelector: '[data-guide="clipboard-list"]',
    title: '查看剪贴板历史',
    description: '复制文本、图片、文件或富文本后会出现在这里。双击或按 Enter 复制，右键可以查看详情。',
    view: 'clipboard' as ViewKey,
    placement: 'right' as GuidePlacement,
  },
  {
    selector: '[data-guide="quick-list"]',
    fallbackSelector: '[data-guide="quick-view"]',
    title: '维护快捷短语',
    description: '常用手机号、邮箱、地址都可以放到快捷输入里，在外部软件中输入//后可快速选择并粘贴。',
    view: 'quick' as ViewKey,
    placement: 'bottom' as GuidePlacement,
  },
  {
    selector: '[data-guide="search"]',
    title: '搜索与键盘操作',
    description: '搜索框支持关键词筛选；上下键移动选中项，Enter 激活条目，Tab 在历史和快捷输入之间切换。',
    view: 'clipboard' as ViewKey,
    placement: 'bottom' as GuidePlacement,
  },
  {
    selector: '[data-guide="tray-settings"]',
    title: '按习惯隐藏窗口',
    description: '默认不会自动隐藏。你可以开启“自动隐藏至托盘”和“不聚焦时关闭”，切换到其它应用后窗口会缩小到托盘，需要时双击 Alt 再唤起。',
    view: 'settings' as ViewKey,
    placement: 'left' as GuidePlacement,
  },
];

const SettingSwitch = defineComponent({
  props: {
    label: { type: String, required: true },
    description: { type: String, required: true },
    modelValue: { type: Boolean, required: true },
  },
  emits: ['update:modelValue'],
  setup(props, { emit }) {
    return () => h('div', { class: 'flex items-center justify-between gap-4 rounded-md bg-header px-3 py-2' }, [
      h('span', [
        h('span', { class: 'block text-sm font-medium' }, props.label),
        h('span', { class: 'block text-xs text-stone-600' }, props.description),
      ]),
      h('button', {
        type: 'button',
        class: [
          'relative h-6 w-11 shrink-0 rounded-full transition',
          props.modelValue ? 'bg-mint' : 'bg-stone-300',
        ],
        role: 'switch',
        'aria-checked': props.modelValue,
        onClick: () => emit('update:modelValue', !props.modelValue),
      }, [
        h('span', {
          class: [
            'absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition',
            props.modelValue ? 'left-5' : 'left-0.5',
          ],
        }),
      ]),
    ]);
  },
});

const quickForm = reactive({
  id: null as number | null,
  content: '',
});

const visibleEntries = computed(() => {
  if (activeView.value === 'quick') {
    return quickInputs.value.map((item) => ({ source: 'quick' as const, id: item.id }));
  }
  return clipboardItems.value.map((item) => ({ source: 'clipboard' as const, id: item.id }));
});
const sidebarCollapsed = computed(() => autoSidebarCollapsed.value || manualSidebarCollapsed.value);
const activeViewTitle = computed(() => navItems.find((item) => item.key === activeView.value)?.label ?? 'SheepClip');
const currentThemeKey = computed(() => resolveThemeKey(settings.theme_key));
const currentFontKey = computed(() => resolveFontKey(settings.font_key, availableFontOptions.value.map((font) => font.key)));
const currentFontSize = computed(() => resolveFontSize(settings.font_size));
const currentFontWeight = computed(() => resolveFontWeight(settings.font_weight));
const appThemeStyle = computed(() => getThemeStyle(currentThemeKey.value, currentFontKey.value, currentFontSize.value, currentFontWeight.value, availableFontOptions.value));
const detailClipboardItem = computed(() => detailKind.value === 'clipboard' ? detailItem.value as ClipboardItem | null : null);
const detailTextSelectable = computed(() => detailKind.value === 'quick' || !detailClipboardItem.value || detailClipboardItem.value.kind === 'text');
const canAddDetailToQuickInput = computed(() => detailKind.value === 'clipboard' && detailClipboardItem.value?.kind === 'text');
const showOnboarding = computed(() => !settings.onboarding_completed);
const currentGuideStep = computed(() => guideSteps[guideIndex.value]);
const guideStepText = computed(() => `${guideIndex.value + 1} / ${guideSteps.length}`);
const guideHighlightStyle = computed(() => {
  if (!guideHighlight.value) return {};
  return {
    left: `${guideHighlight.value.left - 6}px`,
    top: `${guideHighlight.value.top - 6}px`,
    width: `${guideHighlight.value.width + 12}px`,
    height: `${guideHighlight.value.height + 12}px`,
  };
});
const guideCardStyle = computed(() => {
  const rect = guideHighlight.value;
  if (!rect) {
    return { left: '1rem', top: '1rem', maxHeight: 'calc(100vh - 2rem)' };
  }
  const cardWidth = Math.min(352, window.innerWidth - 32);
  const cardHeight = 214;
  const gap = 18;
  const margin = 16;
  const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);
  const viewportMaxLeft = Math.max(margin, window.innerWidth - cardWidth - margin);
  const viewportMaxTop = Math.max(margin, window.innerHeight - cardHeight - margin);
  const preferredPlacements: GuidePlacement[] = [
    currentGuideStep.value.placement,
    'right',
    'bottom',
    'left',
    'top',
  ];
  const placements = preferredPlacements.filter((placement, index, list) => list.indexOf(placement) === index);
  const candidates: Record<GuidePlacement, { left: number; top: number; fits: boolean }> = {
    right: {
      left: rect.right + gap,
      top: clamp(rect.top, margin, viewportMaxTop),
      fits: rect.right + gap + cardWidth <= window.innerWidth - margin,
    },
    bottom: {
      left: clamp(rect.left, margin, viewportMaxLeft),
      top: rect.bottom + gap,
      fits: rect.bottom + gap + cardHeight <= window.innerHeight - margin,
    },
    left: {
      left: rect.left - cardWidth - gap,
      top: clamp(rect.top, margin, viewportMaxTop),
      fits: rect.left - cardWidth - gap >= margin,
    },
    top: {
      left: clamp(rect.left, margin, viewportMaxLeft),
      top: rect.top - cardHeight - gap,
      fits: rect.top - cardHeight - gap >= margin,
    },
  };
  const chosenPlacement = placements.find((placement) => candidates[placement].fits) ?? currentGuideStep.value.placement;
  const chosen = candidates[chosenPlacement];
  return {
    left: `${clamp(chosen.left, margin, viewportMaxLeft)}px`,
    top: `${clamp(chosen.top, margin, viewportMaxTop)}px`,
    maxHeight: 'calc(100vh - 2rem)',
  };
});
const detailHtml = computed(() => {
  if (detailKind.value !== 'clipboard' || detailClipboardItem.value?.kind !== 'rich_text') return '';
  return sanitizeRichTextHtml(detailClipboardItem.value.content);
});
const detailText = computed(() => {
  if (!detailItem.value) return '';
  if (detailKind.value === 'clipboard') {
    const item = detailItem.value as ClipboardItem;
    if (item.kind === 'image') return item.title || item.preview || '图片剪贴板';
    if (item.kind === 'rich_text') return stripHtmlTags(item.content) || item.preview || item.content;
  }
  return detailItem.value.content;
});
const detailMetaText = computed(() => {
  if (!detailItem.value) return '';
  if (detailKind.value === 'clipboard') {
    return (detailItem.value as ClipboardItem).meta || '';
  }
  return `${detailItem.value.content.length} 字`;
});
const shortcutHints = computed(() => [
  {
    label: '主界面唤起',
    short: '主',
    value: settings.main_hotkey_enabled ? `双击 ${settings.main_hotkey}` : '已关闭',
  },
  {
    label: '行内触发',
    short: '行',
    value: settings.inline_trigger_enabled ? settings.inline_trigger : '已关闭',
  },
  { label: '视图切换', short: 'Tab', value: 'Tab' },
  { label: '选择/复制', short: '选', value: '↑↓ / Enter' },
  { label: '查看详情', short: '详', value: '右击' },
]);

function itemKey(source: SourceKey, id: number) {
  return `${source}:${id}`;
}

function refreshAvailableFonts() {
  availableFontOptions.value = getAvailableFontOptions();
}

function normalizeAppearanceSettings() {
  settings.theme_key = resolveThemeKey(settings.theme_key);
  settings.font_key = resolveFontKey(settings.font_key, availableFontOptions.value.map((font) => font.key));
  settings.font_size = resolveFontSize(settings.font_size);
  settings.font_weight = resolveFontWeight(settings.font_weight);
}

function selectTheme(themeKey: ThemeKey) {
  settings.theme_key = themeKey;
}

function selectFont(fontKey: FontKey) {
  settings.font_key = resolveFontKey(fontKey, availableFontOptions.value.map((font) => font.key));
}

function fontWeightLabel(weight: number) {
  if (weight === 600) return '偏粗';
  if (weight === 500) return '适中';
  return '常规';
}

function selectItem(source: SourceKey, id: number) {
  selectedKey.value = itemKey(source, id);
  if (source === 'quick') {
    const item = quickInputs.value.find((entry) => entry.id === id);
    if (item) {
      quickForm.id = item.id;
      quickForm.content = item.content;
      selectedQuickTags.value = [...item.tags];
    }
  }
}

function scrollSelectedIntoView() {
  nextTick(() => {
    if (!selectedKey.value) return;
    const escapedKey = CSS.escape(selectedKey.value);
    const element = document.querySelector<HTMLElement>(`[data-entry-key="${escapedKey}"]`);
    element?.scrollIntoView({ block: 'nearest' });
  });
}

function ensureSelection(preferredIndex = 0) {
  const entries = visibleEntries.value;
  if (entries.length === 0) {
    selectedKey.value = '';
    return;
  }
  if (!entries.some((entry) => itemKey(entry.source, entry.id) === selectedKey.value)) {
    const next = entries[Math.min(preferredIndex, entries.length - 1)];
    selectedKey.value = itemKey(next.source, next.id);
  }
}

function selectPreferredClipboardItem() {
  activeView.value = 'clipboard';
  query.value = '';
  scrollClipboardToTop();
  const preferred = clipboardItems.value[Math.min(1, clipboardItems.value.length - 1)];
  if (preferred) {
    selectedKey.value = itemKey('clipboard', preferred.id);
  }
}

function scrollClipboardToTop() {
  nextTick(() => {
    const element = document.querySelector<HTMLElement>('[data-guide="clipboard-list"]');
    if (element) element.scrollTop = 0;
  });
}

function switchPrimaryView() {
  const currentIndex = navItems.findIndex((item) => item.key === activeView.value);
  activeView.value = navItems[(currentIndex + 1) % navItems.length].key;
}

function findGuideElement(step: GuideStep) {
  return document.querySelector<HTMLElement>(step.selector)
    ?? (step.fallbackSelector ? document.querySelector<HTMLElement>(step.fallbackSelector) : null);
}

function clearGuideStepTimers() {
  guideStepTimers.forEach((timer) => window.clearTimeout(timer));
  guideStepTimers = [];
}

function scrollGuideTargetIntoView(behavior: ScrollBehavior = 'smooth') {
  const step = currentGuideStep.value;
  const element = findGuideElement(step);
  if (!element) return;

  const scroller = step.view === 'settings' ? element.closest<HTMLElement>('[data-guide-scroll]') : null;
  if (scroller) {
    const elementRect = element.getBoundingClientRect();
    const scrollerRect = scroller.getBoundingClientRect();
    const targetTop = scroller.scrollTop
      + elementRect.top
      - scrollerRect.top
      - (scroller.clientHeight - elementRect.height) / 2;
    scroller.scrollTo({ top: Math.max(0, targetTop), behavior });
    return;
  }

  element.scrollIntoView({ block: step.view === 'settings' ? 'center' : 'nearest', inline: 'nearest', behavior });
}

function updateGuideHighlight() {
  if (!showOnboarding.value || !currentGuideStep.value) {
    guideHighlight.value = null;
    return;
  }
  const element = findGuideElement(currentGuideStep.value);
  const rect = element?.getBoundingClientRect() ?? null;
  const visible = rect
    && rect.width > 8
    && rect.height > 8
    && rect.bottom > 0
    && rect.right > 0
    && rect.top < window.innerHeight
    && rect.left < window.innerWidth;
  guideHighlight.value = visible ? rect : null;
}

function measureGuideHighlight() {
  window.clearTimeout(guideMeasureTimer);
  guideMeasureTimer = window.setTimeout(updateGuideHighlight, 80);
}

function scheduleGuideTargetSync() {
  clearGuideStepTimers();
  [40, 180, 340, 560].forEach((delay, index) => {
    guideStepTimers.push(window.setTimeout(() => {
      scrollGuideTargetIntoView(index === 0 ? 'smooth' : 'auto');
      window.requestAnimationFrame(updateGuideHighlight);
    }, delay));
  });
}

function goGuideStep(index: number) {
  guideIndex.value = Math.min(Math.max(index, 0), guideSteps.length - 1);
  activeView.value = currentGuideStep.value.view;
  nextTick(() => {
    scheduleGuideTargetSync();
  });
}

function prevGuideStep() {
  goGuideStep(guideIndex.value - 1);
}

function nextGuideStep() {
  if (guideIndex.value >= guideSteps.length - 1) {
    void finishOnboarding();
    return;
  }
  goGuideStep(guideIndex.value + 1);
}

async function finishOnboarding() {
  settings.onboarding_completed = true;
  guideHighlight.value = null;
  await nextTick();
}

async function openGitRepository() {
  await openExternalUrl('https://github.com/passheep/SheepClip');
}

function clipboardKindIcon(kind: ClipboardItem['kind']) {
  if (kind === 'image') return ImageIcon;
  if (kind === 'file') return File;
  if (kind === 'rich_text') return FileText;
  return Type;
}

function kindLabel(kind?: ClipboardItem['kind']) {
  if (kind === 'image') return '图片';
  if (kind === 'file') return '文件/文件夹';
  if (kind === 'rich_text') return '富文本';
  return '文本';
}

function clipboardDisplayText(item: ClipboardItem) {
  if (item.kind === 'image') return item.title || item.preview || '图片剪贴板';
  if (item.kind === 'file') return item.preview || item.content.split(/\r?\n/).filter(Boolean).join('，');
  if (item.kind === 'rich_text') return item.preview || stripHtmlTags(item.content);
  return item.preview || item.content;
}

function clipboardThumbnail(item: ClipboardItem) {
  if (item.kind === 'image') return item.content;
  if (item.kind === 'rich_text') return firstRichTextImage(item.content);
  return '';
}

function firstRichTextImage(html: string) {
  const document = new DOMParser().parseFromString(html, 'text/html');
  const image = document.querySelector('img');
  return image?.getAttribute('src') || '';
}

function sanitizeRichTextHtml(html: string) {
  const document = new DOMParser().parseFromString(html, 'text/html');
  document.querySelectorAll('script, iframe, object, embed, link, meta').forEach((node) => node.remove());
  document.body.querySelectorAll<HTMLElement>('*').forEach((node) => {
    for (const attribute of [...node.attributes]) {
      const name = attribute.name.toLowerCase();
      const value = attribute.value.trim().toLowerCase();
      if (name.startsWith('on')) {
        node.removeAttribute(attribute.name);
      }
      if ((name === 'href' || name === 'src') && value.startsWith('javascript:')) {
        node.removeAttribute(attribute.name);
      }
    }
  });
  return document.body.innerHTML;
}

function stripHtmlTags(value: string) {
  return value.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim();
}

function moveSelection(delta: number) {
  const entries = visibleEntries.value;
  if (entries.length === 0) return;
  const currentIndex = entries.findIndex((entry) => itemKey(entry.source, entry.id) === selectedKey.value);
  const nextIndex = currentIndex < 0 ? 0 : (currentIndex + delta + entries.length) % entries.length;
  const next = entries[nextIndex];
  selectedKey.value = itemKey(next.source, next.id);
  scrollSelectedIntoView();
}

async function copySelected() {
  const [source, rawId] = selectedKey.value.split(':');
  const id = Number(rawId);
  if (!id) return;
  const normalizedSource = source === 'quick' ? 'quick' : 'clipboard';
  if (normalizedSource === 'quick') {
    await copyQuickInput(id);
  } else {
    await copyClipboardItem(id);
  }
  statusText.value = settings.paste_after_activation ? '已复制并尝试粘贴到当前窗口' : '已复制到系统剪贴板';
  await refreshAll();
  scrollSelectedIntoView();
}

async function confirmSelected() {
  await copySelected();
}

async function copyClipboard(item: ClipboardItem) {
  selectedKey.value = itemKey('clipboard', item.id);
  await copySelected();
}

async function copyQuick(item: QuickInput) {
  selectedKey.value = itemKey('quick', item.id);
  await copySelected();
}

function resetQuickForm() {
  quickForm.id = null;
  quickForm.content = '';
  selectedQuickTags.value = [];
}

function beginCreateQuickInput() {
  resetQuickForm();
  selectedKey.value = '';
  activeView.value = 'quick';
}

async function submitQuickInput() {
  if (!quickForm.content.trim()) {
    statusText.value = '内容不能为空';
    return;
  }
  const saved = await saveQuickInput({
    id: quickForm.id,
    content: quickForm.content,
    tags: selectedQuickTags.value,
  });
  statusText.value = '快捷输入已保存';
  await refreshQuickInputs();
  selectItem('quick', saved.id);
}

async function removeQuickInput() {
  if (!quickForm.id) return;
  await deleteQuickInput(quickForm.id);
  resetQuickForm();
  statusText.value = '快捷输入已删除';
  await refreshQuickInputs();
}

async function deleteClipboard(item: ClipboardItem) {
  const key = itemKey('clipboard', item.id);
  await deleteClipboardItem(item.id);
  if (detailKind.value === 'clipboard' && detailItem.value?.id === item.id) {
    closeDetail();
  }
  if (selectedKey.value === key) {
    selectedKey.value = '';
  }
  statusText.value = '剪贴板历史已删除';
  await refreshClipboardItems();
}

async function deleteDetailItem() {
  if (!detailItem.value || detailKind.value !== 'clipboard') return;
  await deleteClipboard(detailItem.value as ClipboardItem);
  closeDetail();
}

async function persistSettings() {
  const saved = await saveSettings({ ...settings });
  Object.assign(settings, saved);
  statusText.value = '设置已保存';
}

async function handleEscape() {
  if (detailItem.value) {
    closeDetail();
    return;
  }
  if (clearConfirmVisible.value) {
    clearConfirmVisible.value = false;
    return;
  }
  if (resetConfirmVisible.value) {
    resetConfirmVisible.value = false;
    return;
  }
  if (closeConfirmVisible.value) {
    closeConfirmVisible.value = false;
    return;
  }
  if (settings.auto_hide_to_tray) {
    await hideMainWindow();
  } else {
    await minimizeMainWindow();
  }
}

function startWindowDrag(event: PointerEvent) {
  if (event.button !== 0 || !('__TAURI_INTERNALS__' in window)) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest('button,input,textarea,select,a')) return;
  event.preventDefault();
  void markMainPointerOperation();
  void getCurrentWindow().startDragging();
}

function startWindowResize(event?: PointerEvent) {
  if (!('__TAURI_INTERNALS__' in window)) return;
  event?.preventDefault();
  void markMainPointerOperation();
  void getCurrentWindow().startResizeDragging('SouthEast');
}

async function toggleMaximizeWindow() {
  if (!('__TAURI_INTERNALS__' in window)) return;
  await getCurrentWindow().toggleMaximize();
}

function requestCloseMainWindow() {
  if (settings.confirm_exit) {
    closeConfirmVisible.value = true;
    return;
  }
  if (settings.auto_hide_to_tray) {
    void hideMainWindow();
  } else {
    void minimizeMainWindow();
  }
}

async function confirmCloseToTray() {
  closeConfirmVisible.value = false;
  await hideMainWindow();
}

async function confirmExitApp() {
  closeConfirmVisible.value = false;
  await exitApp();
}

async function toggleMainWindowPinned() {
  const nextPinned = !mainWindowPinned.value;
  mainWindowPinned.value = nextPinned;
  try {
    await setMainWindowAlwaysOnTop(nextPinned);
    statusText.value = nextPinned ? '主窗口已置顶' : '主窗口已取消置顶';
  } catch (error) {
    mainWindowPinned.value = !nextPinned;
    statusText.value = error instanceof Error ? error.message : '设置置顶失败';
  }
}

function updateResponsiveSidebar() {
  autoSidebarCollapsed.value = window.innerWidth < 860;
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  event.preventDefault();
  event.stopPropagation();
  void handleEscape();
}

function requestClearHistory() {
  clearConfirmVisible.value = true;
}

async function confirmClearHistory() {
  await clearClipboardHistory();
  clearConfirmVisible.value = false;
  statusText.value = '剪贴板历史已清空';
  await refreshClipboardItems();
}

function requestResetSettings() {
  resetConfirmVisible.value = true;
}

async function confirmResetSettings() {
  if (resetBusy.value) return;
  resetBusy.value = true;
  resetConfirmVisible.value = false;
  settingsReady = false;
  try {
    const restored = await resetSettings();
    Object.assign(settings, restored);
    guideIndex.value = 0;
    statusText.value = '已恢复推荐设置';
    await nextTick();
    goGuideStep(0);
  } catch (error) {
    statusText.value = error instanceof Error ? error.message : '重置设置失败';
  } finally {
    settingsReady = true;
    resetBusy.value = false;
  }
}

function showClipboardDetail(item: ClipboardItem) {
  detailItem.value = item;
  detailKind.value = 'clipboard';
}

function showQuickDetail(item: QuickInput) {
  detailItem.value = item;
  detailKind.value = 'quick';
}

function closeDetail() {
  detailItem.value = null;
}

async function copyDetailItem() {
  if (!detailItem.value) return;
  if (detailKind.value === 'clipboard') {
    await copyClipboard(detailItem.value as ClipboardItem);
  } else {
    await copyQuick(detailItem.value as QuickInput);
  }
  closeDetail();
}

function selectAllDetail() {
  if (detailClipboardItem.value?.kind === 'rich_text' && detailRichTextRef.value) {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(detailRichTextRef.value);
    selection?.removeAllRanges();
    selection?.addRange(range);
    return;
  }
  detailTextRef.value?.focus();
  detailTextRef.value?.select();
}

function startQuickPointerDrag(id: number, event: PointerEvent) {
  if (event.button !== 0) return;
  dragQuickId.value = id;
  dragQuickTargetId.value = id;
  quickDragStarted = false;
  quickDragStartX = event.clientX;
  quickDragStartY = event.clientY;
  const element = event.currentTarget as HTMLElement | null;
  element?.setPointerCapture?.(event.pointerId);
  window.addEventListener('pointermove', markQuickDragging, { passive: true });
  window.addEventListener('pointerup', handleGlobalQuickPointerUp, { once: true });
}

function markQuickDragging(event: PointerEvent) {
  if (Math.abs(event.clientX - quickDragStartX) + Math.abs(event.clientY - quickDragStartY) > 6) {
    quickDragStarted = true;
    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-quick-id]');
    const targetId = Number(target?.dataset.quickId);
    if (targetId) {
      dragQuickTargetId.value = targetId;
    }
  }
}

function hoverQuickDropTarget(id: number) {
  if (dragQuickId.value) {
    dragQuickTargetId.value = id;
  }
}

function quickIndex(id: number | null) {
  if (!id) return -1;
  return quickInputs.value.findIndex((item) => item.id === id);
}

function showQuickDropLine(id: number, position: 'before' | 'after') {
  if (!quickDragStarted || !dragQuickId.value || !dragQuickTargetId.value || dragQuickTargetId.value !== id || dragQuickId.value === id) {
    return false;
  }
  const fromIndex = quickIndex(dragQuickId.value);
  const targetIndex = quickIndex(id);
  if (fromIndex < 0 || targetIndex < 0) return false;
  return fromIndex < targetIndex ? position === 'after' : position === 'before';
}

function handleGlobalQuickPointerUp() {
  void finishQuickPointerDrag(dragQuickTargetId.value);
}

async function finishQuickPointerDrag(targetId: number | null) {
  window.removeEventListener('pointermove', markQuickDragging);
  const sourceId = dragQuickId.value;
  dragQuickId.value = null;
  dragQuickTargetId.value = null;
  if (!quickDragStarted) return;

  quickDragStarted = false;
  if (!sourceId || !targetId || sourceId === targetId) return;

  const nextItems = [...quickInputs.value];
  const fromIndex = nextItems.findIndex((item) => item.id === sourceId);
  const toIndex = nextItems.findIndex((item) => item.id === targetId);
  if (fromIndex < 0 || toIndex < 0) return;

  const [moved] = nextItems.splice(fromIndex, 1);
  const adjustedToIndex = fromIndex < toIndex ? toIndex : toIndex;
  nextItems.splice(adjustedToIndex, 0, moved);
  quickInputs.value = nextItems.map((item, index) => ({ ...item, sort_order: index + 1 }));
  selectedKey.value = itemKey('quick', sourceId);
  await reorderQuickInputs(quickInputs.value.map((item) => item.id));
  statusText.value = '快捷输入顺序已更新';
  scrollSelectedIntoView();
}

function cancelQuickDrag() {
  window.removeEventListener('pointermove', markQuickDragging);
  dragQuickId.value = null;
  dragQuickTargetId.value = null;
  quickDragStarted = false;
}

async function addDetailToQuickInput() {
  if (!detailItem.value || detailKind.value !== 'clipboard') return;
  const saved = await addClipboardItemToQuickInput(detailItem.value.id);
  statusText.value = '已添加到快捷输入';
  closeDetail();
  activeView.value = 'quick';
  await refreshQuickInputs();
  selectItem('quick', saved.id);
}

async function refreshTags() {
  tags.value = await listTags();
}

async function createTag() {
  if (!newTagName.value.trim()) return;
  const saved = await addTag(newTagName.value);
  newTagName.value = '';
  await refreshTags();
  if (!selectedQuickTags.value.includes(saved)) {
    selectedQuickTags.value.push(saved);
  }
  statusText.value = '标签已添加';
}

async function removeTag(tag: string) {
  await deleteTag(tag);
  selectedQuickTags.value = selectedQuickTags.value.filter((item) => item !== tag);
  await Promise.all([refreshTags(), refreshQuickInputs()]);
  statusText.value = '标签已删除';
}

async function refreshClipboardItems() {
  clipboardItems.value = await listClipboardItems(query.value, settings.history_limit);
  ensureSelection();
}

async function refreshQuickInputs() {
  quickInputs.value = await listQuickInputs(query.value);
  ensureSelection();
}

async function refreshAll() {
  await Promise.all([refreshClipboardItems(), refreshQuickInputs(), refreshTags()]);
}

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

let refreshTimer = 0;
let settingsSaveTimer = 0;
let settingsReady = false;
let unlistenClipboardUpdate: (() => void) | null = null;
let unlistenMainWindowShown: (() => void) | null = null;
let unlistenQuickInputsUpdated: (() => void) | null = null;
let unlistenMainCloseRequested: (() => void) | null = null;
let unlistenWindowSize: (() => void) | null = null;
watch(query, () => {
  window.clearTimeout(refreshTimer);
  refreshTimer = window.setTimeout(refreshAll, 160);
});

watch(activeView, () => {
  ensureSelection();
  scrollSelectedIntoView();
  nextTick(() => searchInput.value?.focus());
  measureGuideHighlight();
});

watch(
  () => settings.onboarding_completed,
  (completed) => {
    if (!completed) {
      nextTick(() => goGuideStep(guideIndex.value));
    }
  },
);

watch(
  settings,
  () => {
    if (!settingsReady) return;
    window.clearTimeout(settingsSaveTimer);
    settingsSaveTimer = window.setTimeout(async () => {
      try {
        await persistSettings();
      } catch (error) {
        statusText.value = error instanceof Error ? error.message : '设置保存失败';
        Object.assign(settings, await getSettings());
      }
    }, 260);
  },
  { deep: true },
);

onMounted(async () => {
  refreshAvailableFonts();
  Object.assign(settings, await getSettings());
  normalizeAppearanceSettings();
  await nextTick();
  settingsReady = true;
  updateResponsiveSidebar();
  await refreshAll();
  selectPreferredClipboardItem();
  if (showOnboarding.value) {
    goGuideStep(0);
  }
  if ('__TAURI_INTERNALS__' in window) {
    unlistenWindowSize = await restoreAndTrackWindowSize('sheepclip:main-window-size', {
      minWidth: 760,
      minHeight: 520,
      maxWidth: 3200,
      maxHeight: 2200,
    });
    mainWindowPinned.value = await getCurrentWindow().isAlwaysOnTop();
    unlistenClipboardUpdate = await listen('clipboard-history-updated', async () => {
      await refreshClipboardItems();
      if (activeView.value === 'clipboard') {
        selectPreferredClipboardItem();
      }
      statusText.value = '捕获到新的剪贴板内容';
    });
    unlistenMainWindowShown = await listen('main-window-shown', async () => {
      await refreshClipboardItems();
      selectPreferredClipboardItem();
      await nextTick();
      searchInput.value?.focus();
    });
    unlistenQuickInputsUpdated = await listen('quick-inputs-updated', async () => {
      await Promise.all([refreshQuickInputs(), refreshTags()]);
    });
    unlistenMainCloseRequested = await listen('main-close-requested', () => {
      requestCloseMainWindow();
    });
  }
  window.addEventListener('focus', refreshClipboardItems);
  window.addEventListener('keydown', handleWindowKeydown, true);
  window.addEventListener('resize', updateResponsiveSidebar);
  window.addEventListener('resize', measureGuideHighlight);
  nextTick(() => searchInput.value?.focus());
  window.setTimeout(measureGuideHighlight, 240);
});

onUnmounted(() => {
  window.clearTimeout(refreshTimer);
  window.clearTimeout(settingsSaveTimer);
  window.clearTimeout(guideMeasureTimer);
  clearGuideStepTimers();
  window.removeEventListener('focus', refreshClipboardItems);
  window.removeEventListener('keydown', handleWindowKeydown, true);
  window.removeEventListener('resize', updateResponsiveSidebar);
  window.removeEventListener('resize', measureGuideHighlight);
  unlistenClipboardUpdate?.();
  unlistenMainWindowShown?.();
  unlistenQuickInputsUpdated?.();
  unlistenMainCloseRequested?.();
  unlistenWindowSize?.();
});
</script>


