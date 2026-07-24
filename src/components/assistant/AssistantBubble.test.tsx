import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/test-utils'
import { useUIStore } from '@/store/ui-store'
import { AssistantBubble } from './AssistantBubble'

describe('AssistantBubble', () => {
  beforeEach(() => {
    useUIStore.setState({ assistantOpen: false })
  })

  it('renders a button with accessible label', () => {
    render(<AssistantBubble />)
    expect(
      screen.getByRole('button', { name: 'Open Mistral assistant' })
    ).toBeInTheDocument()
  })

  it('toggles assistantOpen in the store when clicked', () => {
    render(<AssistantBubble />)

    expect(useUIStore.getState().assistantOpen).toBe(false)

    fireEvent.click(screen.getByTestId('assistant-bubble'))
    expect(useUIStore.getState().assistantOpen).toBe(true)

    fireEvent.click(screen.getByTestId('assistant-bubble'))
    expect(useUIStore.getState().assistantOpen).toBe(false)
  })
})
