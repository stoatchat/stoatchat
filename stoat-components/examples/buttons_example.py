"""
Example: Creating interactive buttons for a Stoat bot.
"""

import json
from stoat_components import Button, ButtonStyle


def create_action_row():
    """Create a row with multiple buttons."""
    confirm_btn = Button(
        label="Confirmar",
        style=ButtonStyle.SUCCESS,
        custom_id="confirm_action",
    )

    cancel_btn = Button(
        label="Cancelar",
        style=ButtonStyle.DANGER,
        custom_id="cancel_action",
    )

    link_btn = Button(
        label="Documentação",
        style=ButtonStyle.LINK,
        url="https://stoat.dev/docs",
    )

    return {
        "type": 1,
        "components": [
            confirm_btn.to_dict(),
            cancel_btn.to_dict(),
            link_btn.to_dict(),
        ]
    }


def create_disabled_button():
    """Example of a disabled button."""
    btn = Button(
        label="Indisponível",
        style=ButtonStyle.SECONDARY,
        custom_id="disabled_btn",
        disabled=True,
    )
    return btn.to_dict()


def create_button_with_callback():
    """Example using callback decorator."""
    btn = Button(
        label="Clique Aqui",
        style=ButtonStyle.PRIMARY,
        custom_id="click_me",
    )

    @btn.callback
    async def on_click(interaction):
        print(f"Botão clicado por {interaction.user}")
        await interaction.respond("Você clicou no botão!")

    return btn


if __name__ == "__main__":
    row = create_action_row()
    print("Action Row:")
    print(json.dumps(row, indent=2, ensure_ascii=False))

    print("\nDisabled Button:")
    print(json.dumps(create_disabled_button(), indent=2, ensure_ascii=False))

    print("\nButton with callback:")
    btn = create_button_with_callback()
    print(btn)
