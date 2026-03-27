"""
E2E Test: Round timeout
Inject network delay past commit timeout → round marked Failed →
requester can reclaim escrow.

Agent: Testing Agent — TODO
"""

import pytest


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_commit_phase_timeout():
    """Request account shows Failed status when commit phase times out."""
    pass


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_escrow_refunded_on_timeout():
    """Developer can reclaim escrow lamports after a failed round."""
    pass
