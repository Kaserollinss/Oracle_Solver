import { useState } from 'react'
import { useSolverStore } from '@/store/useSolverStore'

const SUITS = ['s', 'h', 'd', 'c'] as const
const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'] as const

const SUIT_SYMBOLS: Record<string, string> = {
  s: '♠', h: '♥', d: '♦', c: '♣',
}
const SUIT_COLORS: Record<string, string> = {
  s: 'text-(--text)', h: 'text-(--red)', d: 'text-(--red)', c: 'text-(--text)',
}

interface CardSlot {
  rank: string
  suit: string
}

export function BoardSetup() {
  const board = useSolverStore((s) => s.board)
  const setBoard = useSolverStore((s) => s.setBoard)
  const [pickerOpen, setPickerOpen] = useState<number | null>(null)

  // Parse board string into card slots
  const cards: (CardSlot | null)[] = []
  for (let i = 0; i < board.length; i += 2) {
    const rank = board[i]
    const suit = board[i + 1]
    if (rank && suit) {
      cards.push({ rank, suit })
    }
  }
  while (cards.length < 5) cards.push(null)

  const labels = ['F', 'F', 'F', 'T', 'R']

  function setCardAt(index: number, rank: string, suit: string) {
    const newCards = [...cards]
    newCards[index] = { rank, suit }
    // Build board string from non-null cards
    const boardStr = newCards
      .filter((c): c is CardSlot => c !== null)
      .map((c) => c.rank + c.suit)
      .join('')
    setBoard(boardStr)
    setPickerOpen(null)
  }

  function clearCardAt(index: number) {
    const newCards = [...cards]
    newCards[index] = null
    // Compact: remove trailing nulls
    while (newCards.length > 0 && newCards[newCards.length - 1] === null) {
      newCards.pop()
    }
    const boardStr = newCards
      .filter((c): c is CardSlot => c !== null)
      .map((c) => c.rank + c.suit)
      .join('')
    setBoard(boardStr)
  }

  // Cards already on the board (for filtering)
  const usedCards = new Set(
    cards.filter((c): c is CardSlot => c !== null).map((c) => c.rank + c.suit)
  )

  return (
    <div className="border-b border-(--border) p-3.5">
      <div className="text-[9px] uppercase tracking-[2px] text-(--muted) mb-2.5">Board Setup</div>

      <div className="grid grid-cols-5 gap-1 mb-1">
        {cards.map((card, i) => (
          <div
            key={i}
            className="h-[46px] rounded bg-(--s2) border border-(--border2) flex flex-col items-center justify-center cursor-pointer transition-all hover:border-(--accent) hover:bg-(--s3) relative"
            onClick={() => setPickerOpen(pickerOpen === i ? null : i)}
            onContextMenu={(e) => {
              e.preventDefault()
              clearCardAt(i)
            }}
          >
            {card ? (
              <>
                <div className={`text-sm font-bold font-mono leading-none ${SUIT_COLORS[card.suit]}`}>
                  {card.rank}
                </div>
                <div className={`text-[10px] leading-none ${SUIT_COLORS[card.suit]}`}>
                  {SUIT_SYMBOLS[card.suit]}
                </div>
              </>
            ) : (
              <span className="text-(--muted) text-lg">+</span>
            )}

            {pickerOpen === i && (
              <CardPicker
                usedCards={usedCards}
                onSelect={(r, s) => setCardAt(i, r, s)}
                onClose={() => setPickerOpen(null)}
              />
            )}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-5 gap-1">
        {labels.map((l, i) => (
          <div key={i} className="text-[8px] text-(--muted) text-center">
            {l}
          </div>
        ))}
      </div>
    </div>
  )
}

function CardPicker({
  usedCards,
  onSelect,
  onClose,
}: {
  usedCards: Set<string>
  onSelect: (rank: string, suit: string) => void
  onClose: () => void
}) {
  return (
    <>
      <div className="fixed inset-0 z-40" onClick={onClose} />
      <div className="absolute top-full left-0 mt-1 z-50 bg-(--s2) border border-(--border2) rounded-md p-2 shadow-lg min-w-[200px]">
        {SUITS.map((suit) => (
          <div key={suit} className="flex gap-0.5 mb-0.5">
            {RANKS.map((rank) => {
              const key = rank + suit
              const used = usedCards.has(key)
              return (
                <button
                  key={key}
                  disabled={used}
                  className={`w-[14px] h-[18px] text-[9px] font-mono rounded-sm flex items-center justify-center transition-all
                    ${used ? 'opacity-20 cursor-not-allowed' : `cursor-pointer hover:bg-(--accent) hover:text-black ${SUIT_COLORS[suit]}`}
                    bg-(--s3)`}
                  onClick={(e) => {
                    e.stopPropagation()
                    if (!used) onSelect(rank, suit)
                  }}
                >
                  {rank}
                </button>
              )
            })}
          </div>
        ))}
      </div>
    </>
  )
}
