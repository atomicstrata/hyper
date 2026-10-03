"""Native exceptions expose JSON Pointer locations and a reason."""
from . import _core


class _LocatedError(Exception):
    location: str
    reason: str


HifValidationError: type[_LocatedError] = getattr(_core, "HifValidationError")
HifCompatibilityError: type[_LocatedError] = getattr(_core, "HifCompatibilityError")
