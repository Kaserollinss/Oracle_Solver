import { useSolverStore } from '@/store/useSolverStore'

export function GameContext() {
  const pot = useSolverStore((s) => s.pot)
  const stacks = useSolverStore((s) => s.stacks)
  const setPot = useSolverStore((s) => s.setPot)
  const setStacks = useSolverStore((s) => s.setStacks)

  return (
    <div className="border-b border-(--border) p-3.5">
      <div className="text-[9px] uppercase tracking-[2px] text-(--muted) mb-2.5">Game Context</div>

      <div className="flex gap-2">
        <div className="flex-1">
          <label className="text-[9px] uppercase tracking-[1px] text-(--muted) block mb-1">
            Stack (bb)
          </label>
          <input
            type="number"
            value={stacks}
            onChange={(e) => setStacks(Number(e.target.value))}
            className="w-full bg-(--s2) border border-(--border2) text-(--text) px-2 py-1.5 rounded text-[11px] font-mono focus:outline-none focus:border-(--accent)"
          />
        </div>
        <div className="flex-1">
          <label className="text-[9px] uppercase tracking-[1px] text-(--muted) block mb-1">
            Pot (bb)
          </label>
          <input
            type="number"
            value={pot}
            onChange={(e) => setPot(Number(e.target.value))}
            className="w-full bg-(--s2) border border-(--border2) text-(--text) px-2 py-1.5 rounded text-[11px] font-mono focus:outline-none focus:border-(--accent)"
          />
        </div>
      </div>
    </div>
  )
}
