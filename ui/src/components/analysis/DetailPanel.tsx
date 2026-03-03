import { useSolverStore } from '@/store/useSolverStore'
import { useUiStore } from '@/store/useUiStore'
import type { ActionDto } from '@/bindings'

function actionColor(action: ActionDto, idx: number): string {
  if (action.action_type === 'fold') return 'var(--red)'
  if (action.action_type === 'check' || action.action_type === 'call') return 'var(--green)'
  const betColors = ['var(--accent2)', 'var(--accent)', 'var(--yellow)']
  return betColors[Math.min(idx, betColors.length - 1)]
}

export function DetailPanel() {
  const matrix = useSolverStore((s) => s.strategyMatrix)
  const pinnedHandIdx = useUiStore((s) => s.pinnedHandIdx)
  const hoveredHandIdx = useUiStore((s) => s.hoveredHandIdx)
  const pinHand = useUiStore((s) => s.pinHand)

  const displayIdx = pinnedHandIdx ?? hoveredHandIdx
  const cell = matrix && displayIdx !== null ? matrix.cells[displayIdx] : null
  const isPinned = pinnedHandIdx !== null

  return (
    <div className="w-60 bg-(--s1) border-l border-(--border) p-4 shrink-0 overflow-y-auto">
      <div className="text-[9px] uppercase tracking-[2px] text-(--muted) mb-3 flex justify-between items-center">
        {isPinned ? 'Pinned Hand' : 'Hand Detail'}
        {isPinned && (
          <span
            className="text-sm text-(--muted) cursor-pointer hover:text-(--text)"
            onClick={() => pinHand(null)}
          >
            ×
          </span>
        )}
      </div>

      {!cell ? (
        <div className="text-(--muted) text-[11px] leading-relaxed">
          {matrix
            ? 'Hover over a cell in the matrix to see details. Click to pin.'
            : 'Run a solve to see hand details.'}
        </div>
      ) : (
        <>
          <div className="text-[26px] font-bold font-mono mb-1">{cell.hand_label}</div>
          <div className="text-xs text-(--muted) mb-3.5">
            {cell.combos} combo{cell.combos !== 1 ? 's' : ''}
            {cell.is_pair ? ' (pair)' : cell.is_suited ? ' (suited)' : ' (offsuit)'}
          </div>

          {/* Action frequency bars */}
          {matrix && (
            <div className="mb-4">
              {matrix.actions.map((action, i) => {
                const freq = cell.frequencies[i] ?? 0
                if (freq < 0.005) return null
                const color = actionColor(action, i)
                return (
                  <div key={i} className="flex items-center gap-2 mb-1.5">
                    <div className="text-[10px] w-14 shrink-0" style={{ color }}>
                      {action.label}
                    </div>
                    <div className="flex-1 h-1 bg-(--border2) rounded-sm overflow-hidden">
                      <div
                        className="h-full rounded-sm"
                        style={{ width: `${freq * 100}%`, backgroundColor: color }}
                      />
                    </div>
                    <div
                      className="text-[10px] font-mono w-8 text-right"
                      style={{ color }}
                    >
                      {Math.round(freq * 100)}%
                    </div>
                  </div>
                )
              })}
            </div>
          )}

          {/* Stats */}
          <div className="border-t border-(--border) pt-3">
            <div className="flex justify-between py-1.5 border-b border-(--border) text-[11px]">
              <span className="text-(--muted)">Combos</span>
              <span className="font-mono text-(--text)">{cell.combos}</span>
            </div>
            <div className="flex justify-between py-1.5 border-b border-(--border) text-[11px]">
              <span className="text-(--muted)">Type</span>
              <span className="font-mono text-(--text)">
                {cell.is_pair ? 'Pair' : cell.is_suited ? 'Suited' : 'Offsuit'}
              </span>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
