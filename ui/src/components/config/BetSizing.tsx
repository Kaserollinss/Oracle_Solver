import { useSolverStore } from '@/store/useSolverStore'
import { cn } from '@/lib/utils'

const COMMON_SIZES = [0.25, 0.33, 0.5, 0.75, 1.0, 1.5, 2.0]

export function BetSizing() {
  const betSizes = useSolverStore((s) => s.betSizes)
  const raiseSizes = useSolverStore((s) => s.raiseSizes)
  const setBetSizes = useSolverStore((s) => s.setBetSizes)
  const setRaiseSizes = useSolverStore((s) => s.setRaiseSizes)

  function toggleSize(current: number[], size: number, setter: (s: number[]) => void) {
    if (current.includes(size)) {
      setter(current.filter((s) => s !== size))
    } else {
      setter([...current, size].sort((a, b) => a - b))
    }
  }

  return (
    <div className="border-b border-(--border) p-3.5">
      <div className="text-[9px] uppercase tracking-[2px] text-(--muted) mb-2.5">Bet Sizing</div>

      <div className="text-[9px] text-(--muted) mb-1">Bet Sizes (pot fraction)</div>
      <div className="flex gap-1 flex-wrap mb-2">
        {COMMON_SIZES.map((size) => (
          <Pill
            key={size}
            label={`${Math.round(size * 100)}%`}
            active={betSizes.includes(size)}
            onClick={() => toggleSize(betSizes, size, setBetSizes)}
          />
        ))}
      </div>

      <div className="text-[9px] text-(--muted) mb-1">Raise Sizes (pot fraction)</div>
      <div className="flex gap-1 flex-wrap">
        {[0.5, 0.75, 1.0, 1.5, 2.0].map((size) => (
          <Pill
            key={size}
            label={`${Math.round(size * 100)}%`}
            active={raiseSizes.includes(size)}
            onClick={() => toggleSize(raiseSizes, size, setRaiseSizes)}
          />
        ))}
      </div>
    </div>
  )
}

function Pill({
  label,
  active,
  onClick,
}: {
  label: string
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      className={cn(
        'px-2.5 py-1 rounded text-[10px] font-mono cursor-pointer transition-all border',
        active
          ? 'bg-[rgba(192,132,252,0.15)] border-(--accent) text-(--accent)'
          : 'bg-(--s2) border-(--border2) text-(--muted) hover:text-(--text2)'
      )}
      onClick={onClick}
    >
      {label}
    </button>
  )
}
