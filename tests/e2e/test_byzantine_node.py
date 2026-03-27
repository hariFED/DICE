"""
E2E Test: Byzantine node
One node reveals entropy that does not match its commit hash.
Round must still finalize with the remaining 4 valid reveals.

Agent: Testing Agent — TODO
"""

import pytest


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_bad_reveal_rejected():
    """Coordinator rejects a reveal where hash(entropy) != commit."""
    pass


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_round_finalizes_despite_byzantine_node():
    """Round completes with 4 honest nodes despite 1 bad reveal."""
    pass
