import { Sidebar } from '@/components/layout/Sidebar'
import { Topbar } from '@/components/layout/Topbar'
import { StatusBar } from '@/components/layout/StatusBar'
import { ConfigRail } from '@/components/config/ConfigRail'
import { AnalysisView } from '@/components/analysis/AnalysisView'
import { useSolveEvents } from '@/hooks/useSolveEvents'
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts'
import { useUiStore } from '@/store/useUiStore'

function App() {
  useSolveEvents()
  useKeyboardShortcuts()

  const colorblindMode = useUiStore((s) => s.colorblindMode)

  return (
    <div className={`flex h-screen w-screen ${colorblindMode ? 'colorblind' : ''}`}>
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        <Topbar />
        <div className="flex-1 flex overflow-hidden">
          <ConfigRail />
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Tab bar */}
            <div className="flex bg-(--s1) border-b border-(--border) px-3.5 shrink-0">
              <TabButton label="Analysis Matrix" active />
              <TabButton label="Aggregation" />
              <TabButton label="Blocker Impact" />
              <TabButton label="Compare Trees" />
            </div>
            {/* Content panel */}
            <AnalysisView />
          </div>
        </div>
        <StatusBar />
      </div>
    </div>
  )
}

function TabButton({ label, active }: { label: string; active?: boolean }) {
  return (
    <div
      className={`py-3 px-4 text-[11px] cursor-pointer border-b-2 transition-all uppercase tracking-[1px] whitespace-nowrap font-medium ${
        active
          ? 'text-(--text) border-b-(--accent)'
          : 'text-(--muted) border-transparent hover:text-(--text2)'
      }`}
    >
      {label}
    </div>
  )
}

export default App
