import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ErrorBoundary } from './ErrorBoundary'

vi.mock('@/lib/recovery', () => ({
  saveCrashState: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@/lib/logger', () => ({
  logger: {
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
}))

const { saveCrashState } = await import('@/lib/recovery')
const { logger } = await import('@/lib/logger')

function Bomb(): never {
  throw new Error('boom')
}

describe('ErrorBoundary', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    vi.clearAllMocks()
    // React logs the caught error to console.error; keep test output clean
    consoleErrorSpy = vi
      .spyOn(console, 'error')
      .mockImplementation(() => undefined)
  })

  afterEach(() => {
    consoleErrorSpy.mockRestore()
  })

  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <div>All good</div>
      </ErrorBoundary>
    )

    expect(screen.getByText('All good')).toBeInTheDocument()
  })

  it('renders the fallback UI when a child throws', () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    )

    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Reload Application' })
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Try Again' })
    ).toBeInTheDocument()
  })

  it('logs the crash via the logger', () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    )

    expect(logger.error).toHaveBeenCalledWith(
      'Application crashed',
      expect.objectContaining({ error: 'boom' })
    )
  })

  it('saves crash state with the error message', () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    )

    expect(saveCrashState).toHaveBeenCalledWith(
      expect.any(Object),
      expect.objectContaining({ error: 'boom' })
    )
  })

  it('does not throw from the crash handler when saving fails', () => {
    vi.mocked(saveCrashState).mockRejectedValueOnce(new Error('disk full'))

    expect(() =>
      render(
        <ErrorBoundary>
          <Bomb />
        </ErrorBoundary>
      )
    ).not.toThrow()
  })

  it('resets to children when "Try Again" is clicked and the child stops throwing', () => {
    let shouldThrow = true
    function MaybeBomb() {
      if (shouldThrow) throw new Error('boom')
      return <div>Recovered</div>
    }

    render(
      <ErrorBoundary>
        <MaybeBomb />
      </ErrorBoundary>
    )

    expect(screen.getByText('Something went wrong')).toBeInTheDocument()

    shouldThrow = false
    fireEvent.click(screen.getByRole('button', { name: 'Try Again' }))

    expect(screen.getByText('Recovered')).toBeInTheDocument()
  })

  it('reloads the window when "Reload Application" is clicked', () => {
    const reloadMock = vi.fn()
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...window.location, reload: reloadMock },
    })

    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    )

    fireEvent.click(screen.getByRole('button', { name: 'Reload Application' }))

    expect(reloadMock).toHaveBeenCalledTimes(1)
  })
})
