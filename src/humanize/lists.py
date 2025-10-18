"""Lists related humanization."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Any

__all__ = ["natural_list"]


def natural_list(items: list[Any]) -> str:
    """Natural list.

    Convert a list of items into a human-readable string with commas and 'and'.

    Examples:
        >>> natural_list(["one", "two", "three"])
        'one, two and three'
        >>> natural_list(["one", "two"])
        'one and two'
        >>> natural_list(["one"])
        'one'

    Args:
        items (list): An iterable of items.

    Returns:
        str: A string with commas and 'and' in the right places.
    """
    if len(items) == 1:
        return str(items[0])

    if len(items) == 2:
        return f"{items[0]} and {items[1]}"

    return ", ".join(str(item) for item in items[:-1]) + f" and {items[-1]}"
