# Authentication

To authenticate with the API, you must first acquire a bot token.

## Create a Bot

1. Navigate to **Settings** > **My Bots** > **Create Bot**.
2. Enter a **Username** and configure any optional fields as needed.
3. Scroll to the bottom of the page and select **Copy Token**.

:::warning Security Notice
Store your token in a secure place and never commit it to public version control. You will need it to authenticate your bot.
:::

Then you may provide the bot token when working with an authenticated route.

|     Header      |        Value       |
| :-------------: | :----------------: |
|  `X-Bot-Token`  |   `qPWLwPTB3-...`  |
