"""
Complete test suite for stoat-components.
Run: python test_components.py
"""

import json
from stoat_components import Button, ButtonStyle, SelectMenu, SelectOption


def test_button_styles():
    print("=== Button Styles Test ===\n")

    styles = [
        ("Primary", ButtonStyle.PRIMARY),
        ("Secondary", ButtonStyle.SECONDARY),
        ("Success", ButtonStyle.SUCCESS),
        ("Danger", ButtonStyle.DANGER),
    ]

    for name, style in styles:
        btn = Button(label=name, style=style, custom_id=f"btn_{style.name.lower()}")
        print(f"{name}: {json.dumps(btn.to_dict(), indent=2)}")

    link_btn = Button(
        label="Visit Site",
        style=ButtonStyle.LINK,
        url="https://stoat.chat"
    )
    print(f"\nLink: {json.dumps(link_btn.to_dict(), indent=2)}")


def test_button_features():
    print("\n=== Button Features Test ===\n")

    btn_emoji = Button(
        label="Like",
        style=ButtonStyle.PRIMARY,
        custom_id="like_btn",
        emoji="heart"
    )
    print(f"With emoji: {json.dumps(btn_emoji.to_dict(), indent=2)}")

    btn_disabled = Button(
        label="Unavailable",
        style=ButtonStyle.SECONDARY,
        custom_id="disabled_btn",
        disabled=True
    )
    print(f"\nDisabled: {json.dumps(btn_disabled.to_dict(), indent=2)}")

    @btn_emoji.callback
    async def on_like(interaction):
        print(f"Button clicked by {interaction.user}")

    print(f"\nCallback registered: {btn_emoji._callback is not None}")


def test_select_menu():
    print("\n=== Select Menu Test ===\n")

    options = [
        SelectOption("Option 1", "opt1", description="First option"),
        SelectOption("Option 2", "opt2", description="Second option"),
        SelectOption("Option 3", "opt3", description="Third option"),
    ]

    menu = SelectMenu(
        custom_id="simple_menu",
        options=options,
        placeholder="Select an option..."
    )
    print(f"Simple menu:\n{json.dumps(menu.to_dict(), indent=2)}")

    multi_options = [
        SelectOption("Channel 1", "ch1"),
        SelectOption("Channel 2", "ch2"),
        SelectOption("Channel 3", "ch3"),
    ]

    multi_menu = SelectMenu(
        custom_id="multi_menu",
        options=multi_options,
        placeholder="Select multiple channels...",
        min_values=1,
        max_values=3
    )
    print(f"\nMulti-select:\n{json.dumps(multi_menu.to_dict(), indent=2)}")


def test_action_row():
    print("\n=== Action Row Test ===\n")

    confirm = Button("Yes", ButtonStyle.SUCCESS, custom_id="yes")
    cancel = Button("No", ButtonStyle.DANGER, custom_id="no")
    link = Button("Help", ButtonStyle.LINK, url="https://stoat.chat/help")

    row = {
        "type": 1,
        "components": [
            confirm.to_dict(),
            cancel.to_dict(),
            link.to_dict(),
        ]
    }

    print(json.dumps(row, indent=2))


if __name__ == "__main__":
    print("Testing stoat-components v0.1.0\n")

    test_button_styles()
    test_button_features()
    test_select_menu()
    test_action_row()

    print("\nAll tests passed!")
