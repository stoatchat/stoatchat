# Stoat Migration Guide

:::warning

This is not yet finished.

:::

## Endpoint changes

| Service    | Old URL                             | New URL                                 |
| ---------- | ----------------------------------- | --------------------------------------- |
| **API**    | `https://api.revolt.chat`           | `https://stoat.chat/api`                |
|            | `https://app.revolt.chat/api`       | `https://stoat.chat/api`                |
|            | `https://revolt.chat/api`           | No equivalent                           |
| **Events** | `wss://ws.revolt.chat`              | `wss://stoat.chat/events`               |
|            | `wss://app.revolt.chat/events`      | `wss://stoat.chat/events`               |
|            | `wss://revolt.chat/events`          | No equivalent                           |
| **Files**  | `https://autumn.revolt.chat`        | `https://cdn.stoatusercontent.com`      |
|            | `https://cdn.revoltusercontent.com` | `https://cdn.stoatusercontent.com`      |
| **Proxy**  | `https://jan.revolt.chat`           | `https://external.stoatusercontent.com` |
| **Voice**  | `https://vortex.revolt.chat`        | Superseded by Voice Chats v2            |
