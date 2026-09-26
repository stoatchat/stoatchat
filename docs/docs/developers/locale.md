# Locale

Certain endpoints have full text errors with localization keys attached. The current error types with these keys are:

- ContactSupport(locale: localization key, msg: plaintext english message)

the following localization keys are used by the backend in error messages:

- ContactSupport - discover.bot_removal_approved - The bot is already on discover, the user must contact support to have it removed.
- ContactSupport - discover.bot_removal_removed - The bot has been removed from discover by moderators, the user must contact support.
- ContactSupport - discover.server_removal_approved - The server is already on discover, the user must contact support to have it removed.
- ContactSupport - discover.server_removal_removed - The server has been removed from discover by moderators, the user must contact support.
- ContactSupport - discover.declined_apply_again - The Discover request was declined. Declined requests cannot be removed, but the user may submit again.
- ContactSupport - discover.cannot_auto_remove - The Discover request is approved or the item has been removed from discover. In either case, the user must contact support.
- ContactSupport - discover.removed_cannot_apply - The item was removed by moderators, and as such future applications are prohibited. Contact support for more information.