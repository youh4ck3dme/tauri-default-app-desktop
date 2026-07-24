import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@/test/test-utils'
import { LeftSideBar } from './LeftSideBar'

describe('LeftSideBar', () => {
  it('renders the Websupport group expanded by default with its three subcategories', () => {
    render(<LeftSideBar />)

    expect(screen.getByRole('button', { name: /websupport/i })).toHaveAttribute(
      'aria-expanded',
      'true'
    )
    expect(screen.getByText('Domény')).toBeInTheDocument()
    expect(screen.getByText('Email')).toBeInTheDocument()
    expect(screen.getByText('DNS')).toBeInTheDocument()
  })

  it('collapses the subcategories when Websupport is clicked', () => {
    render(<LeftSideBar />)

    fireEvent.click(screen.getByRole('button', { name: /websupport/i }))

    expect(screen.getByRole('button', { name: /websupport/i })).toHaveAttribute(
      'aria-expanded',
      'false'
    )
    expect(screen.queryByText('Domény')).not.toBeInTheDocument()
    expect(screen.queryByText('Email')).not.toBeInTheDocument()
    expect(screen.queryByText('DNS')).not.toBeInTheDocument()
  })

  it('expands the subcategories again on a second click', () => {
    render(<LeftSideBar />)
    const toggle = screen.getByRole('button', { name: /websupport/i })

    fireEvent.click(toggle)
    fireEvent.click(toggle)

    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText('Domény')).toBeInTheDocument()
  })
})
