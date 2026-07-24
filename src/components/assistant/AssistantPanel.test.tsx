import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@/test/test-utils'
import { commands } from '@/lib/tauri-bindings'
import { useUIStore } from '@/store/ui-store'
import { AssistantPanel } from './AssistantPanel'

const mockCommands = vi.mocked(commands)

const pendingAction = {
  id: 'pending-1',
  tool_name: 'create_dns_record',
  description: 'Create A record api.example.com → 1.2.3.4 (TTL 300)',
  args: {
    domain: 'example.com',
    type: 'A',
    name: 'api',
    content: '1.2.3.4',
    ttl: 300,
  },
}

describe('AssistantPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useUIStore.setState({ assistantOpen: true })
    mockCommands.mistralSendMessage.mockResolvedValue({
      status: 'ok',
      data: { reply: '', pending_actions: [] },
    })
    mockCommands.mistralConfirmAction.mockResolvedValue({
      status: 'ok',
      data: 'DNS record created successfully',
    })
  })

  it('renders empty state when there are no messages', () => {
    render(<AssistantPanel />)

    expect(screen.getByTestId('assistant-empty-state')).toBeInTheDocument()
    expect(
      screen.getByText(
        'Ask me to manage your Websupport DNS records, domains, or mailboxes.'
      )
    ).toBeInTheDocument()
  })

  it('renders a sent user message and the assistant reply', async () => {
    mockCommands.mistralSendMessage.mockResolvedValue({
      status: 'ok',
      data: {
        reply: 'Here are your DNS records.',
        pending_actions: [],
      },
    })

    render(<AssistantPanel />)

    const input = screen.getByTestId('assistant-input')
    fireEvent.change(input, { target: { value: 'List DNS for example.com' } })
    fireEvent.click(screen.getByTestId('assistant-send'))

    await waitFor(() => {
      expect(screen.getByText('List DNS for example.com')).toBeInTheDocument()
      expect(screen.getByText('Here are your DNS records.')).toBeInTheDocument()
    })

    expect(mockCommands.mistralSendMessage).toHaveBeenCalledWith([
      { role: 'user', content: 'List DNS for example.com' },
    ])
  })

  it('renders pending action cards with Confirm calling mistralConfirmAction', async () => {
    mockCommands.mistralSendMessage.mockResolvedValue({
      status: 'ok',
      data: {
        reply: 'I queued a DNS change for confirmation.',
        pending_actions: [pendingAction],
      },
    })

    render(<AssistantPanel />)

    fireEvent.change(screen.getByTestId('assistant-input'), {
      target: { value: 'Create an A record' },
    })
    fireEvent.click(screen.getByTestId('assistant-send'))

    await waitFor(() => {
      expect(
        screen.getByTestId(`pending-action-${pendingAction.id}`)
      ).toBeInTheDocument()
    })

    expect(screen.getByText(pendingAction.description)).toBeInTheDocument()

    fireEvent.click(screen.getByTestId(`confirm-action-${pendingAction.id}`))

    await waitFor(() => {
      expect(mockCommands.mistralConfirmAction).toHaveBeenCalledWith(
        pendingAction
      )
    })

    await waitFor(() => {
      expect(
        screen.getByText('DNS record created successfully')
      ).toBeInTheDocument()
      expect(
        screen.queryByTestId(`pending-action-${pendingAction.id}`)
      ).not.toBeInTheDocument()
    })
  })

  it('Cancel removes the pending card without calling mistralConfirmAction', async () => {
    mockCommands.mistralSendMessage.mockResolvedValue({
      status: 'ok',
      data: {
        reply: 'Please confirm this change.',
        pending_actions: [pendingAction],
      },
    })

    render(<AssistantPanel />)

    fireEvent.change(screen.getByTestId('assistant-input'), {
      target: { value: 'Delete a record' },
    })
    fireEvent.click(screen.getByTestId('assistant-send'))

    await waitFor(() => {
      expect(
        screen.getByTestId(`pending-action-${pendingAction.id}`)
      ).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId(`cancel-action-${pendingAction.id}`))

    expect(
      screen.queryByTestId(`pending-action-${pendingAction.id}`)
    ).not.toBeInTheDocument()
    expect(mockCommands.mistralConfirmAction).not.toHaveBeenCalled()
  })
})
