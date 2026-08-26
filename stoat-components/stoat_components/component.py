from abc import ABC, abstractmethod
from typing import Any, Dict, Optional


class Component(ABC):
    """Base class for all Stoat bot components."""

    def __init__(self, custom_id: Optional[str] = None):
        self._custom_id = custom_id
        self._disabled = False

    @property
    def custom_id(self) -> Optional[str]:
        return self._custom_id

    @custom_id.setter
    def custom_id(self, value: str):
        self._custom_id = value

    @property
    def disabled(self) -> bool:
        return self._disabled

    def disable(self):
        self._disabled = True
        return self

    def enable(self):
        self._disabled = False
        return self

    @abstractmethod
    def to_dict(self) -> Dict[str, Any]:
        """Convert component to dictionary format for API."""
        pass

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} custom_id={self._custom_id}>"
