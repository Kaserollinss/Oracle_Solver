import { useSolverStore } from '@/store/useSolverStore'

export function EngineSettings() {
  const maxIterations = useSolverStore((s) => s.maxIterations)
  const threshold = useSolverStore((s) => s.threshold)
  const checkEvery = useSolverStore((s) => s.checkEvery)
  const timeCap = useSolverStore((s) => s.timeCap)
  const setMaxIterations = useSolverStore((s) => s.setMaxIterations)
  const setThreshold = useSolverStore((s) => s.setThreshold)
  const setCheckEvery = useSolverStore((s) => s.setCheckEvery)
  const setTimeCap = useSolverStore((s) => s.setTimeCap)

  return (
    <details className="p-3.5 border-b border-(--border)">
      <summary className="text-[10px] uppercase tracking-[1px] text-(--text2) cursor-pointer list-none flex items-center gap-1.5 [&::-webkit-details-marker]:hidden">
        <span className="text-[8px] text-(--muted) transition-transform [[open]>&]:rotate-90">▶</span>
        Advanced Engine Settings
      </summary>
      <div className="mt-3 space-y-2.5">
        <Field label="Max Iterations" value={maxIterations} onChange={setMaxIterations} />
        <Field label="Threshold (bb)" value={threshold} onChange={setThreshold} step={0.01} />
        <Field label="Check Every" value={checkEvery} onChange={setCheckEvery} />
        <Field label="Time Cap (s)" value={timeCap} onChange={setTimeCap} />
      </div>
    </details>
  )
}

function Field({
  label,
  value,
  onChange,
  step = 1,
}: {
  label: string
  value: number
  onChange: (v: number) => void
  step?: number
}) {
  return (
    <div>
      <label className="text-[9px] uppercase tracking-[1px] text-(--muted) block mb-1">
        {label}
      </label>
      <input
        type="number"
        value={value}
        step={step}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full bg-(--s2) border border-(--border2) text-(--text) px-2 py-1.5 rounded text-[11px] font-mono focus:outline-none focus:border-(--accent)"
      />
    </div>
  )
}
