"""
Script to test components visually on Stoat.

Before running:
1. Create a bot at https://stoat.chat/developers
2. Set environment variables STOAT_TOKEN and STOAT_CHANNEL_ID
3. Run: python send_components.py

Example (Windows PowerShell):
  $env:STOAT_TOKEN="your_token_here"
  $env:STOAT_CHANNEL_ID="your_channel_id_here"
  python send_components.py

Example (Linux/Mac):
  export STOAT_TOKEN="your_token_here"
  export STOAT_CHANNEL_ID="your_channel_id_here"
  python send_components.py
"""

import os
import requests
import json


TOKEN = os.environ.get("STOAT_TOKEN")
CHANNEL_ID = os.environ.get("STOAT_CHANNEL_ID")

API_URL = "https://api.stoat.chat"


def send_message(channel_id: str, content: str, components: list = None):
    headers = {
        "X-Bot-Token": TOKEN,
        "Content-Type": "application/json",
    }

    payload = {"content": content}
    if components:
        payload["components"] = components

    response = requests.post(
        f"{API_URL}/channels/{channel_id}/messages",
        headers=headers,
        json=payload,
    )

    if response.status_code in (200, 201):
        print(f"  Message sent successfully!")
        return response.json()
    else:
        print(f"  Error {response.status_code}: {response.text}")
        return None


def test_buttons():
    print("\n1. Testing buttons...")

    components = [
        {
            "type": 1,
            "components": [
                {"type": 2, "style": 1, "label": "Primary", "custom_id": "btn_primary"},
                {"type": 2, "style": 2, "label": "Secondary", "custom_id": "btn_secondary"},
                {"type": 2, "style": 3, "label": "Success", "custom_id": "btn_success"},
                {"type": 2, "style": 4, "label": "Danger", "custom_id": "btn_danger"},
            ],
        }
    ]

    send_message(CHANNEL_ID, "Button test:", components)


def test_link_button():
    print("\n2. Testing link button...")

    components = [
        {
            "type": 1,
            "components": [
                {"type": 2, "style": 5, "label": "Visit Stoat", "url": "https://stoat.chat"},
            ],
        }
    ]

    send_message(CHANNEL_ID, "Link button:", components)


def test_disabled_button():
    print("\n3. Testing disabled button...")

    components = [
        {
            "type": 1,
            "components": [
                {"type": 2, "style": 1, "label": "Active", "custom_id": "active_btn"},
                {"type": 2, "style": 1, "label": "Disabled", "custom_id": "disabled_btn", "disabled": True},
            ],
        }
    ]

    send_message(CHANNEL_ID, "Active vs disabled buttons:", components)


def test_select_menu():
    print("\n4. Testing select menu...")

    components = [
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "custom_id": "role_selector",
                    "options": [
                        {"label": "Admin", "value": "admin", "description": "Full access"},
                        {"label": "Moderator", "value": "mod", "description": "Chat moderation"},
                        {"label": "Member", "value": "member", "description": "Default member"},
                    ],
                    "placeholder": "Select a role...",
                }
            ],
        }
    ]

    send_message(CHANNEL_ID, "Select menu (dropdown):", components)


def test_multi_select():
    print("\n5. Testing multi-select...")

    components = [
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "custom_id": "channel_selector",
                    "options": [
                        {"label": "General", "value": "general"},
                        {"label": "Help", "value": "help"},
                        {"label": "Off-Topic", "value": "offtopic"},
                    ],
                    "placeholder": "Select channels...",
                    "min_values": 1,
                    "max_values": 3,
                }
            ],
        }
    ]

    send_message(CHANNEL_ID, "Multi-select (choose multiple):", components)


def test_mixed_components():
    print("\n6. Testing mixed components...")

    components = [
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "custom_id": "action_select",
                    "options": [
                        {"label": "Ban", "value": "ban"},
                        {"label": "Mute", "value": "mute"},
                        {"label": "Kick", "value": "kick"},
                    ],
                    "placeholder": "Select an action...",
                }
            ],
        },
        {
            "type": 1,
            "components": [
                {"type": 2, "style": 3, "label": "Confirm", "custom_id": "confirm_action"},
                {"type": 2, "style": 4, "label": "Cancel", "custom_id": "cancel_action"},
            ],
        },
    ]

    send_message(CHANNEL_ID, "Select menu + buttons:", components)


if __name__ == "__main__":
    if not TOKEN or not CHANNEL_ID:
        print("ERROR: Set STOAT_TOKEN and STOAT_CHANNEL_ID environment variables!")
        print("")
        print("Windows PowerShell:")
        print('  $env:STOAT_TOKEN="your_token"')
        print('  $env:STOAT_CHANNEL_ID="your_channel_id"')
        print("")
        print("Linux/Mac:")
        print('  export STOAT_TOKEN="your_token"')
        print('  export STOAT_CHANNEL_ID="your_channel_id"')
    else:
        print("Sending components to Stoat...")
        test_buttons()
        test_link_button()
        test_disabled_button()
        test_select_menu()
        test_multi_select()
        test_mixed_components()
        print("\nDone! Check the channel on Stoat.")
