from __future__ import annotations

import pytest  # pylint: disable=import-error


@pytest.fixture(scope="module")
def nums() -> list[int]:
    return list(range(1, 1_000_001))
