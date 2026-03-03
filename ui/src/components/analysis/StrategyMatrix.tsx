import { useSolverStore } from '@/store/useSolverStore'
import { useUiStore } from '@/store/useUiStore'
import { cn } from '@/lib/utils'
import type { MatrixCellDto, ActionDto } from '@/bindings'

const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2']

// Map action_type to a consistent color
function actionColorIndex(action: ActionDto, idx: number, _total: number): string {
  if (action.action_type === 'fold') return 'var(--red)'
  if (action.action_type === 'check') return 'var(--green)'
  if (action.action_type === 'call') return 'var(--green)'
  // For bets, distribute across accent colors
  const betColors = ['var(--accent2)', 'var(--accent)', 'var(--yellow)']
  return betColors[Math.min(idx, betColors.length - 1)]
}

export function StrategyMatrix() {
  const matrix = useSolverStore((s) => s.strategyMatrix)
  const pinnedHandIdx = useUiStore((s) => s.pinnedHandIdx)
  const pinHand = useUiStore((s) => s.pinHand)
  const setHoveredHandIdx = useUiStore((s) => s.setHoveredHandIdx)
  if (!matrix) {
    return (
      <div className="flex-1 flex items-center justify-center text-(--muted) text-sm p-4">
        Configure a board and run the solver to see strategies.
      </div>
    )
  }

  const actions = matrix.actions

  return (
    <div className="flex-1 p-4 overflow-auto flex flex-col">
      {/* Controls */}
      <div className="flex items-center gap-3 mb-3 flex-wrap">
        <div className="text-xs text-(--text2) font-medium">
          {matrix.player} Strategy at Node #{matrix.node_id}
        </div>
      </div>

      {/* Legend */}
      <div className="flex gap-3 flex-wrap mb-3">
        {actions.map((action, i) => (
          <div key={i} className="flex items-center gap-1 text-[10px] text-(--text2)">
            <div
              className="w-2.5 h-2.5 rounded-sm shrink-0 opacity-80"
              style={{ backgroundColor: actionColorIndex(action, i, actions.length) }}
            />
            {action.label}
          </div>
        ))}
      </div>

      {/* Column headers */}
      <div className="flex gap-0.5 mb-0.5 max-w-[580px]">
        <div className="w-4 shrink-0" />
        <div className="grid grid-cols-13 gap-0.5 flex-1">
          {RANKS.map((r) => (
            <div key={r} className="text-[8px] text-(--muted) text-center font-mono">
              {r}
            </div>
          ))}
        </div>
      </div>

      {/* Matrix grid */}
      <div className="flex flex-col gap-0.5 shrink-0 max-w-[580px]">
        {RANKS.map((rowRank, rowIdx) => (
          <div key={rowIdx} className="flex gap-0.5">
            <div className="w-4 shrink-0 text-[8px] text-(--muted) flex items-center font-mono">
              {rowRank}
            </div>
            <div className="grid grid-cols-13 gap-0.5 flex-1">
              {RANKS.map((_colRank, colIdx) => {
                const cellIdx = rowIdx * 13 + colIdx
                const cell = matrix.cells[cellIdx]
                if (!cell) return <div key={colIdx} />
                return (
                  <MatrixCell
                    key={colIdx}
                    cell={cell}
                    actions={actions}
                    isPinned={pinnedHandIdx === cellIdx}
                    onPin={() => pinHand(pinnedHandIdx === cellIdx ? null : cellIdx)}
                    onHover={(hovering) => setHoveredHandIdx(hovering ? cellIdx : null)}
                  />
                )
              })}
            </div>
          </div>
        ))}
      </div>

      <div className="mt-3 text-[10px] text-(--muted)">
        Click a cell to pin its details. Press Escape to unpin.
      </div>
    </div>
  )
}

function MatrixCell({
  cell,
  actions,
  isPinned,
  onPin,
  onHover,
}: {
  cell: MatrixCellDto
  actions: ActionDto[]
  isPinned: boolean
  onPin: () => void
  onHover: (hovering: boolean) => void
}) {
  if (cell.combos === 0) {
    return (
      <div className="aspect-square rounded-sm bg-(--s2) opacity-20" />
    )
  }

  // Find dominant action
  const maxFreq = Math.max(...cell.frequencies)
  const domIdx = cell.frequencies.indexOf(maxFreq)
  const domColor = actionColorIndex(actions[domIdx], domIdx, actions.length)

  return (
    <div
      className={cn(
        'aspect-square rounded-sm cursor-pointer relative overflow-hidden transition-transform',
        'hover:scale-[1.2] hover:z-10 hover:shadow-lg hover:border hover:border-white/20',
        isPinned && 'border-2 border-white z-5 scale-[1.1] shadow-[0_0_10px_var(--accent)]'
      )}
      style={{
        backgroundColor: domColor,
        opacity: 0.3 + maxFreq * 0.7,
      }}
      onClick={onPin}
      onMouseEnter={() => onHover(true)}
      onMouseLeave={() => onHover(false)}
    >
      {/* Frequency stripe at bottom */}
      <div className="absolute bottom-0 left-0 right-0 h-1 flex">
        {cell.frequencies.map((freq, i) =>
          freq > 0.02 ? (
            <div
              key={i}
              style={{
                width: `${freq * 100}%`,
                backgroundColor: actionColorIndex(actions[i], i, actions.length),
              }}
              className="h-full"
            />
          ) : null
        )}
      </div>
    </div>
  )
}
