# Localization Error Details

This document details the error messages that occur on certain endpoints when localization keys are involved.

## Error Format
All error messages follow this standardized format:
`ContactSupport(locale: <localization key>, msg: <plaintext english message>)`

## Backend Localization Keys

The following backend keys are used in error messages:

| Key | Description |
| :--- | :--- |
| `discover.bot_removal_approved` | The bot is already on discover; users must contact support for removal. |
| `discover.bot_removal_removed` | The bot has been removed from discover by moderators; users must contact support. |
| `discover.server_removal_approved` | The server is already on discover; users must contact support for removal. |
| `discover.server_removal_removed` | The server has been removed from discover by moderators; users must contact support. |
| `discover.declined_apply_again` | The Discover request was declined. Users can resubmit the request. |
| `discover.cannot_auto_remove` | The Discover request is approved or the item has been removed. Users must contact support. |
| `discover.removed_cannot_apply` | The item was removed by moderators, and future applications are prohibited. Contact support for details. |