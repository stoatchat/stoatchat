from enum import IntEnum
from typing import Any, Dict, Optional, Callable


class ButtonStyle(IntEnum):
    """Button style variants."""
    PRIMARY = 1
    SECONDARY = 2
    SUCCESS = 3
    DANGER = 4
    LINK = 5


class Button:
    """Interactive button component for Stoat bots."""

    def __init__(
        self,
        label: str,
        style: ButtonStyle = ButtonStyle.PRIMARY,
        custom_id: Optional[str] = None,
        disabled: bool = False,
        emoji: Optional[str] = None,
        url: Optional[str] = None,
    ):
        if style == ButtonStyle.LINK and not url:
            raise ValueError("Link buttons must have a URL")
        if style != ButtonStyle.LINK and not custom_id:
            raise ValueError("Non-link buttons must have a custom_id")

        self.label = label
        self.style = style
        self._custom_id = custom_id
        self.disabled = disabled
        self.emoji = emoji
        self.url = url
        self._callback: Optional[Callable] = None

    @property
    def custom_id(self) -> Optional[str]:
        return self._custom_id

    def callback(self, func: Callable):
        """Decorator to register a callback function."""
        self._callback = func
        return func

    def to_dict(self) -> Dict[str, Any]:
        data = {
            "type": 2,
            "style": self.style.value,
            "label": self.label,
            "disabled": self.disabled,
        }
        if self._custom_id:
            data["custom_id"] = self._custom_id
        if self.emoji:
            data["emoji"] = {"name": self.emoji}
        if self.url:
            data["url"] = self.url
        return data

    def __repr__(self) -> str:
        return f"<Button label='{self.label}' style={self.style.name}>"
