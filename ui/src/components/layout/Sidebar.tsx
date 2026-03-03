import { cn } from '@/lib/utils'

const navItems = [
  { icon: '⬡', label: 'Session Setup', active: true },
  { icon: '⊞', label: 'Batch Queue', badge: '3' },
  { icon: '📁', label: 'Saved Ranges' },
  { icon: '🕒', label: 'Solve History' },
]

export function Sidebar() {
  return (
    <div className="w-[200px] bg-(--s1) border-r border-(--border) flex flex-col shrink-0 z-50">
      {/* Logo */}
      <div className="px-4 py-3 pb-3 flex items-center gap-2 border-b border-(--border)">
        <div className="w-7 h-7 bg-gradient-to-br from-(--accent) to-(--accent3) rounded-md flex items-center justify-center text-white text-sm font-serif italic">
          O
        </div>
        <div>
          <div className="text-[13px] font-bold text-(--text) tracking-wide">Oracle</div>
        </div>
        <div className="ml-auto text-[9px] text-(--muted) font-mono">v0.1</div>
      </div>

      {/* Nav sections */}
      <div className="py-2 border-b border-(--border)">
        <div className="text-[9px] uppercase tracking-[2px] text-(--muted) px-3.5 py-1 pb-1.5">
          Workbench
        </div>
        {navItems.slice(0, 2).map((item) => (
          <NavItem key={item.label} {...item} />
        ))}
      </div>

      <div className="py-2 border-b border-(--border)">
        <div className="text-[9px] uppercase tracking-[2px] text-(--muted) px-3.5 py-1 pb-1.5">
          Library
        </div>
        {navItems.slice(2).map((item) => (
          <NavItem key={item.label} {...item} />
        ))}
      </div>
    </div>
  )
}

function NavItem({
  icon,
  label,
  active,
  badge,
}: {
  icon: string
  label: string
  active?: boolean
  badge?: string
}) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 py-[7px] px-3.5 cursor-pointer text-xs transition-all border-l-2 border-transparent',
        active
          ? 'text-(--accent) bg-(--glow) border-l-(--accent)'
          : 'text-(--text2) hover:text-(--text) hover:bg-(--s2)'
      )}
    >
      <span className="text-[13px] w-4 text-center shrink-0">{icon}</span>
      {label}
      {badge && (
        <span className="ml-auto px-1.5 py-px rounded-full text-[9px] bg-[rgba(192,132,252,0.15)] text-(--accent) font-mono">
          {badge}
        </span>
      )}
    </div>
  )
}
