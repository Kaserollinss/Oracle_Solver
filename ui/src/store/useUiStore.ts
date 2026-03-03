import { create } from 'zustand'
import type { HandDetailDto } from '../bindings'

type ActivePlayer = 'IP' | 'OOP'
type ActiveStreet = 'Flop' | 'Turn' | 'River'
type ViewMode = 'freq' | 'ev' | 'equity'
type ActiveTab = 'analysis' | 'aggregation' | 'blocker' | 'compare'

interface UiState {
  activePlayer: ActivePlayer
  activeStreet: ActiveStreet
  viewMode: ViewMode
  colorblindMode: boolean

  hoveredHandIdx: number | null
  pinnedHandIdx: number | null
  pinnedDetail: HandDetailDto | null

  activeTab: ActiveTab
  sidebarCollapsed: boolean

  setActivePlayer: (p: ActivePlayer) => void
  togglePlayer: () => void
  setActiveStreet: (s: ActiveStreet) => void
  setViewMode: (m: ViewMode) => void
  cycleViewMode: () => void
  setColorblindMode: (on: boolean) => void
  setHoveredHandIdx: (idx: number | null) => void
  pinHand: (idx: number | null, detail?: HandDetailDto | null) => void
  setActiveTab: (t: ActiveTab) => void
  setSidebarCollapsed: (c: boolean) => void
}

export const useUiStore = create<UiState>((set) => ({
  activePlayer: 'IP',
  activeStreet: 'Flop',
  viewMode: 'freq',
  colorblindMode: false,

  hoveredHandIdx: null,
  pinnedHandIdx: null,
  pinnedDetail: null,

  activeTab: 'analysis',
  sidebarCollapsed: false,

  setActivePlayer: (activePlayer) => {
    console.debug('[ui] player:', activePlayer)
    set({ activePlayer })
  },
  togglePlayer: () =>
    set((s) => {
      const next = s.activePlayer === 'IP' ? 'OOP' : 'IP'
      console.debug('[ui] toggle player:', next)
      return { activePlayer: next as ActivePlayer }
    }),
  setActiveStreet: (activeStreet) => {
    console.debug('[ui] street:', activeStreet)
    set({ activeStreet })
  },
  setViewMode: (viewMode) => set({ viewMode }),
  cycleViewMode: () =>
    set((s) => {
      const modes: ViewMode[] = ['freq', 'ev', 'equity']
      const idx = modes.indexOf(s.viewMode)
      const next = modes[(idx + 1) % modes.length]
      console.debug('[ui] viewMode:', next)
      return { viewMode: next }
    }),
  setColorblindMode: (colorblindMode) => set({ colorblindMode }),
  setHoveredHandIdx: (hoveredHandIdx) => set({ hoveredHandIdx }),
  pinHand: (pinnedHandIdx, detail = null) => set({ pinnedHandIdx, pinnedDetail: detail ?? null }),
  setActiveTab: (activeTab) => set({ activeTab }),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
}))
