"""
E2E Test: Node dropout
7 nodes selected, 3 drop after commit phase → only 4 reveal → round still finalizes.

Agent: Testing Agent — TODO
"""

import pytest


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_round_completes_with_minimum_reveals():
    """Round finalizes with exactly 4 reveals (threshold minimum)."""
    pass


@pytest.mark.skip(reason="Testing Agent: not yet implemented")
def test_dropout_nodes_penalized():
    """Nodes that committed but did not reveal are excluded from next N rounds."""
    pass
