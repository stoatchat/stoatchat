from typing import Any, Dict, List, Optional


class SelectOption:
    """Option for a select menu."""

    def __init__(
        self,
        label: str,
        value: str,
        description: Optional[str] = None,
        emoji: Optional[str] = None,
        default: bool = False,
    ):
        self.label = label
        self.value = value
        self.description = description
        self.emoji = emoji
        self.default = default

    def to_dict(self) -> Dict[str, Any]:
        data = {
            "label": self.label,
            "value": self.value,
        }
        if self.description:
            data["description"] = self.description
        if self.emoji:
            data["emoji"] = {"name": self.emoji}
        if self.default:
            data["default"] = True
        return data

    def __repr__(self) -> str:
        return f"<SelectOption label='{self.label}' value='{self.value}'>"


class SelectMenu:
    """Dropdown select menu component for Stoat bots."""

    def __init__(
        self,
        custom_id: str,
        options: List[SelectOption],
        placeholder: Optional[str] = None,
        min_values: int = 1,
        max_values: int = 1,
        disabled: bool = False,
    ):
        self._custom_id = custom_id
        self.options = options
        self.placeholder = placeholder
        self.min_values = min_values
        self.max_values = max_values
        self.disabled = disabled
        self._callback = None

    @property
    def custom_id(self) -> Optional[str]:
        return self._custom_id

    def callback(self, func):
        """Decorator to register a callback function."""
        self._callback = func
        return func

    def to_dict(self) -> Dict[str, Any]:
        data = {
            "type": 3,
            "custom_id": self._custom_id,
            "options": [opt.to_dict() for opt in self.options],
            "min_values": self.min_values,
            "max_values": self.max_values,
            "disabled": self.disabled,
        }
        if self.placeholder:
            data["placeholder"] = self.placeholder
        return data

    def __repr__(self) -> str:
        return f"<SelectMenu custom_id='{self._custom_id}' options={len(self.options)}>"
