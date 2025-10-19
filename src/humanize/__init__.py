"""Main package for humanize."""

from __future__ import annotations

try:
    from humanize._fast import (  # type: ignore[attr-defined]
        intcomma,
        intword,
        naturalsize,
    )

    _FAST_ENABLED = True
except ImportError:
    from humanize.filesize import naturalsize
    from humanize.number import intcomma, intword

    _FAST_ENABLED = False


from humanize.i18n import activate, deactivate, decimal_separator, thousands_separator
from humanize.lists import natural_list
from humanize.number import apnumber, clamp, fractional, metric, ordinal, scientific
from humanize.time import (
    naturaldate,
    naturalday,
    naturaldelta,
    naturaltime,
    precisedelta,
)

try:
    from humanize._version import __version__  # type: ignore[import-not-found]
except ModuleNotFoundError:
    __version__ = "0.0.0"  # fallback version

__all__ = [
    "__version__",
    "activate",
    "apnumber",
    "clamp",
    "deactivate",
    "decimal_separator",
    "fractional",
    "intcomma",
    "intword",
    "metric",
    "natural_list",
    "naturaldate",
    "naturalday",
    "naturaldelta",
    "naturalsize",
    "naturaltime",
    "ordinal",
    "precisedelta",
    "scientific",
    "thousands_separator",
]
