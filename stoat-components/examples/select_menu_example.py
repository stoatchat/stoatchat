"""
Example: Creating select menus for a Stoat bot.
"""

import json
from stoat_components import SelectMenu, SelectOption


def create_role_selector():
    """Create a role selection menu."""
    options = [
        SelectOption(
            label="Admin",
            value="admin",
            description="Acesso total ao servidor",
        ),
        SelectOption(
            label="Moderador",
            value="mod",
            description="Permissoes de moderacao",
        ),
        SelectOption(
            label="Membro",
            value="member",
            description="Membro padrao",
        ),
        SelectOption(
            label="Visitante",
            value="visitor",
            description="Apenas visualizacao",
        ),
    ]

    menu = SelectMenu(
        custom_id="role_selector",
        options=options,
        placeholder="Selecione seu cargo...",
        min_values=1,
        max_values=1,
    )
    return menu


def create_channel_selector():
    """Create a multi-select channel menu."""
    options = [
        SelectOption(label="Geral", value="geral"),
        SelectOption(label="Ajuda", value="ajuda"),
        SelectOption(label="Off-Topic", value="offtopic"),
    ]

    menu = SelectMenu(
        custom_id="channel_selector",
        options=options,
        placeholder="Selecione canais para acesso...",
        min_values=1,
        max_values=3,
    )
    return menu


def create_disabled_menu():
    """Example of a disabled select menu."""
    options = [
        SelectOption(label="Opcao 1", value="opt1"),
    ]

    menu = SelectMenu(
        custom_id="disabled_menu",
        options=options,
        placeholder="Menu indisponivel",
        disabled=True,
    )
    return menu


if __name__ == "__main__":
    print("Role Selector:")
    role_menu = create_role_selector()
    print(json.dumps(role_menu.to_dict(), indent=2, ensure_ascii=False))

    print("\nChannel Selector (multi-select):")
    channel_menu = create_channel_selector()
    print(json.dumps(channel_menu.to_dict(), indent=2, ensure_ascii=False))

    print("\nDisabled Menu:")
    disabled_menu = create_disabled_menu()
    print(json.dumps(disabled_menu.to_dict(), indent=2, ensure_ascii=False))
