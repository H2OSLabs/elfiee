import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CollaboratorItem } from './CollaboratorItem'
import type { Block, Editor, Grant } from '@/bindings'

describe('CollaboratorItem Component', () => {
  const mockOnGrantChange = vi.fn()
  const mockOnRemoveAccess = vi.fn()

  const mockHumanEditor: Editor = {
    editor_id: 'human-123',
    name: 'Alice',
    editor_type: 'Human',
  }

  const mockBotEditor: Editor = {
    editor_id: 'bot-456',
    name: 'CodeReviewer',
    editor_type: 'Bot',
  }

  const mockGrants: Grant[] = [
    {
      editor_id: 'human-123',
      cap_id: 'markdown.read',
      block_id: 'block-1',
    },
    {
      editor_id: 'human-123',
      cap_id: 'markdown.write',
      block_id: 'block-1',
    },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Visual Differentiation', () => {
    it('should display User icon for Human editor', () => {
      const { container } = render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      expect(screen.getByText('Alice')).toBeInTheDocument()
      // User icon should be present (blue user icon for humans)
      const iconContainer = container.querySelector('.text-blue-500')
      expect(iconContainer).toBeInTheDocument()
    })

    it('should display Bot icon and AI Assistant label for Bot editor', () => {
      const { container } = render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      expect(screen.getByText('CodeReviewer')).toBeInTheDocument()
      expect(screen.getByText('AI Assistant')).toBeInTheDocument()
      // Bot icon should be purple
      const iconContainer = container.querySelector('.text-purple-500')
      expect(iconContainer).toBeInTheDocument()
    })

    it('should display Crown icon for owner', () => {
      const { container } = render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={true}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Crown icon should be amber/yellow for owner
      const crownIcon = container.querySelector('.text-amber-500')
      expect(crownIcon).toBeInTheDocument()
    })

    it('should display Active badge when editor is active', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={true}
          onGrantChange={mockOnGrantChange}
        />
      )

      expect(screen.getByText('Active')).toBeInTheDocument()
    })
  })

  describe('Permission Display', () => {
    it('should show checked permissions that editor has', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Should show permission labels for Read, Write, Delete
      expect(screen.getByText('Read')).toBeInTheDocument()
      expect(screen.getByText('Write')).toBeInTheDocument()
      expect(screen.getByText('Delete')).toBeInTheDocument()
    })

    it('should show all permissions as checked for owner', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={[]} // Owner doesn't need grants
          isOwner={true}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // All checkboxes should be checked and disabled for owner
      const checkboxes = screen.getAllByRole('checkbox')
      checkboxes.forEach((checkbox) => {
        expect(checkbox).toBeDisabled()
      })
    })

    it('should disable permission checkboxes for owner', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={[]}
          isOwner={true}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      const checkboxes = screen.getAllByRole('checkbox')
      checkboxes.forEach((checkbox) => {
        expect(checkbox).toBeDisabled()
      })
    })
  })

  describe('Permission Toggle', () => {
    it('should call onGrantChange when toggling unchecked permission', async () => {
      const user = userEvent.setup()
      mockOnGrantChange.mockResolvedValue(undefined)

      const grantsWithoutWrite: Grant[] = [
        {
          editor_id: 'human-123',
          cap_id: 'markdown.read',
          block_id: 'block-1',
        },
      ]

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={grantsWithoutWrite}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Click on Write permission (currently unchecked)
      const writeLabel = screen.getByText('Write').closest('div')
      if (writeLabel) {
        await user.click(writeLabel)
      }

      await waitFor(() => {
        expect(mockOnGrantChange).toHaveBeenCalledWith(
          'human-123',
          'markdown.write',
          true
        )
      })
    })

    it('should call onGrantChange when toggling checked permission', async () => {
      const user = userEvent.setup()
      mockOnGrantChange.mockResolvedValue(undefined)

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Click on Read permission (currently checked)
      const readLabel = screen.getByText('Read').closest('div')
      if (readLabel) {
        await user.click(readLabel)
      }

      await waitFor(() => {
        expect(mockOnGrantChange).toHaveBeenCalledWith(
          'human-123',
          'markdown.read',
          false
        )
      })
    })

    it('should not call onGrantChange for owner permissions', async () => {
      const user = userEvent.setup()

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={[]}
          isOwner={true}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      const readLabel = screen.getByText('Read').closest('div')
      if (readLabel) {
        await user.click(readLabel)
      }

      // Should not have been called because owner permissions can't be modified
      expect(mockOnGrantChange).not.toHaveBeenCalled()
    })
  })

  describe('Bot Configuration Menu', () => {
    it('should show Configure option in menu for bot editor', async () => {
      const user = userEvent.setup()
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          onRemoveAccess={mockOnRemoveAccess}
        />
      )

      // Find the dropdown trigger button (MoreVertical icon button)
      const buttons = screen.getAllByRole('button')
      // The menu button should be the last button (DropdownMenuTrigger)
      const menuButton = buttons[buttons.length - 1]

      await user.click(menuButton)

      // Should show Configure option
      await waitFor(() => {
        expect(screen.getByText('Configure')).toBeInTheDocument()
      })
    })

    it('should NOT show Configure option for human editor', async () => {
      const user = userEvent.setup()

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          onRemoveAccess={mockOnRemoveAccess}
        />
      )

      // Find and click the dropdown trigger button
      const buttons = screen.getAllByRole('button')
      const menuButton = buttons[buttons.length - 1]

      await user.click(menuButton)

      // Should NOT show Configure option for humans
      await waitFor(() => {
        expect(screen.queryByText('Configure')).not.toBeInTheDocument()
      })
      // But should show Remove Access
      expect(screen.getByText('Remove Access')).toBeInTheDocument()
    })

    it('should NOT show menu button for owner', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={[]}
          isOwner={true}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Owner should not have dropdown menu
      // Only checkboxes should be disabled for owner
      expect(screen.getByText('Alice')).toBeInTheDocument()
      expect(screen.getByText('Owner')).toBeInTheDocument()
    })
  })

  describe('Remove Access', () => {
    it('should show Remove Access option in menu', async () => {
      const user = userEvent.setup()

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          onRemoveAccess={mockOnRemoveAccess}
        />
      )

      // Find the dropdown trigger button
      const buttons = screen.getAllByRole('button')
      const menuButton = buttons[buttons.length - 1]

      await user.click(menuButton)

      await waitFor(() => {
        expect(screen.getByText('Remove Access')).toBeInTheDocument()
      })
    })

    it('should call onRemoveAccess when Remove Access is clicked', async () => {
      const user = userEvent.setup()
      mockOnRemoveAccess.mockResolvedValue(undefined)

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          onRemoveAccess={mockOnRemoveAccess}
        />
      )

      // Find and click menu button
      const buttons = screen.getAllByRole('button')
      const menuButton = buttons[buttons.length - 1]

      await user.click(menuButton)

      const removeButton = await screen.findByText('Remove Access')
      await user.click(removeButton)

      await waitFor(() => {
        expect(mockOnRemoveAccess).toHaveBeenCalledWith('human-123')
      })
    })

    it('should not call onRemoveAccess if callback not provided', async () => {
      const user = userEvent.setup()

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          // No onRemoveAccess provided
        />
      )

      // Find and click menu button
      const buttons = screen.getAllByRole('button')
      const menuButton = buttons[buttons.length - 1]

      await user.click(menuButton)

      const removeButton = await screen.findByText('Remove Access')
      await user.click(removeButton)

      // Should not throw error
      await waitFor(() => {
        expect(mockOnRemoveAccess).not.toHaveBeenCalled()
      })
    })
  })

  describe('Wildcard Grants', () => {
    it('should recognize wildcard grants for permissions', () => {
      const wildcardGrants: Grant[] = [
        {
          editor_id: 'human-123',
          cap_id: 'markdown.read',
          block_id: '*', // Wildcard - applies to all blocks
        },
      ]

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={wildcardGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      // Should display editor name and permissions
      expect(screen.getByText('Alice')).toBeInTheDocument()
      expect(screen.getByText('Read')).toBeInTheDocument()
    })
  })

  describe('Agent Status Toggle', () => {
    const mockOnToggleAgentStatus = vi.fn()

    const mockAgentBlock: Block = {
      block_id: 'agent-block-789',
      name: 'CodeReviewer Agent',
      block_type: 'agent',
      owner: 'owner-123',
      contents: {
        name: 'CodeReviewer',
        target_project_id: 'dir-block-1',
        status: 'enabled',
      },
      children: {},
      metadata: {},
    }

    beforeEach(() => {
      mockOnToggleAgentStatus.mockClear()
    })

    it('should show toggle switch for bot editor with agent block', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          agentBlock={mockAgentBlock}
          onToggleAgentStatus={mockOnToggleAgentStatus}
        />
      )

      const toggle = screen.getByTestId('agent-toggle-bot-456')
      expect(toggle).toBeInTheDocument()
      expect(screen.getByText('Active')).toBeInTheDocument()
    })

    it('should show toggle for bot editor without agent block (switch off)', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
        />
      )

      const toggle = screen.getByTestId('agent-toggle-bot-456')
      expect(toggle).toBeInTheDocument()
      // Should show Disabled since no agent block exists
      expect(screen.getByText('Inactive')).toBeInTheDocument()
    })

    it('should NOT show toggle for human editor', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="markdown"
          editor={mockHumanEditor}
          grants={mockGrants}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          agentBlock={mockAgentBlock}
          onToggleAgentStatus={mockOnToggleAgentStatus}
        />
      )

      expect(
        screen.queryByTestId('agent-toggle-human-123')
      ).not.toBeInTheDocument()
    })

    it('should call onToggleAgentStatus when toggle is clicked', async () => {
      const user = userEvent.setup()
      mockOnToggleAgentStatus.mockResolvedValue(undefined)

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          agentBlock={mockAgentBlock}
          onToggleAgentStatus={mockOnToggleAgentStatus}
        />
      )

      const toggle = screen.getByTestId('agent-toggle-bot-456')
      await user.click(toggle)

      await waitFor(() => {
        expect(mockOnToggleAgentStatus).toHaveBeenCalledWith(
          'agent-block-789',
          'enabled'
        )
      })
    })

    it('should show Disabled status for disabled agent', () => {
      const disabledAgentBlock: Block = {
        ...mockAgentBlock,
        contents: {
          name: 'CodeReviewer',
          target_project_id: 'dir-block-1',
          status: 'disabled',
        },
      }

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          agentBlock={disabledAgentBlock}
          onToggleAgentStatus={mockOnToggleAgentStatus}
        />
      )

      expect(screen.getByText('Inactive')).toBeInTheDocument()
    })

    it('should call onCreateAgent when toggling on without agent block', async () => {
      const user = userEvent.setup()
      const mockOnCreateAgent = vi.fn().mockResolvedValue(undefined)

      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          onCreateAgent={mockOnCreateAgent}
        />
      )

      const toggle = screen.getByTestId('agent-toggle-bot-456')
      await user.click(toggle)

      await waitFor(() => {
        expect(mockOnCreateAgent).toHaveBeenCalledWith('bot-456')
      })
    })

    it('should show Agent label alongside the toggle', () => {
      render(
        <CollaboratorItem
          blockId="block-1"
          blockType="directory"
          editor={mockBotEditor}
          grants={[]}
          isOwner={false}
          isActive={false}
          onGrantChange={mockOnGrantChange}
          agentBlock={mockAgentBlock}
          onToggleAgentStatus={mockOnToggleAgentStatus}
        />
      )

      expect(screen.getByText('Agent Capabilities')).toBeInTheDocument()
    })
  })
})
