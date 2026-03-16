#!/bin/bash
# ============================================================================
# Demo 1: Error Recovery
# ============================================================================
#
# This demo shows how AIOS handles command failures intelligently instead of
# just printing "command not found" and giving up.
#
# To run this demo yourself:
#   1. Install aish: make install
#   2. Configure an LLM backend in ~/.config/aios/llm.yml
#   3. Start: aish
#   4. Type the commands below and watch what happens
#
# ============================================================================

cat << 'TRANSCRIPT'

# ── Scenario 1: Missing command ──────────────────────────────────────────────
# You type a command that doesn't exist. Instead of "command not found" and
# nothing else, AIOS investigates your system and tells you what to do.

  $ dockr ps
  aios: command not found: dockr
  🔍 Investigating...

  AI: 'dockr' isn't installed, but I found 'docker' at /usr/bin/docker.
      Did you mean:
        docker ps

      (Looks like a typo — docker is already installed on this system.)


# ── Scenario 2: Command fails with a cryptic error ──────────────────────────
# A real command fails. The AI reads the error and explains the fix.

  $ git push origin main
  fatal: Authentication failed for 'https://github.com/...'

  AI: Your Git push failed because HTTPS authentication didn't work.
      This usually means your credentials expired or you need a token.

      Quick fixes:
        # Switch to SSH (recommended):
        git remote set-url origin git@github.com:tgifriday/ai-os.git

        # Or generate a personal access token at:
        # https://github.com/settings/tokens


# ── Scenario 3: Wrong flags ─────────────────────────────────────────────────
# You use a flag that doesn't exist. The AI suggests the right one.

  $ tar -xf archive.tar.gz --verbose --directory /tmp
  tar: unrecognized option '--directory'

  AI: The flag is -C (capital C), not --directory:
        tar -xf archive.tar.gz --verbose -C /tmp

TRANSCRIPT

echo ""
echo "To try this yourself: install aish and configure any LLM backend."
echo "See: https://github.com/tgifriday/ai-os#bringing-ai-online"
