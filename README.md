<div align="center">
<h1>
  Stoat Backend

[![Stars](https://img.shields.io/github/stars/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/stargazers)
[![Forks](https://img.shields.io/github/forks/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/network/members)
[![Pull Requests](https://img.shields.io/github/issues-pr/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/pulls)
[![Issues](https://img.shields.io/github/issues/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/issues)
[![Contributors](https://img.shields.io/github/contributors/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/graphs/contributors)
[![License](https://img.shields.io/github/license/revoltchat/backend?style=flat-square&logoColor=white)](https://github.com/revoltchat/backend/blob/main/LICENSE)
</h1>
The services and libraries that power the Revolt service.<br/>
<br/>

| Crate              | Path                                               | Description                         |                                                                                                                                                                                                                                                                                                           |
| ------------------ | -------------------------------------------------- | ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `core/config`      | [crates/core/config](crates/core/config)           | Core: Configuration                 | ![Crates.io Version](https://img.shields.io/crates/v/revolt-config) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-config) ![Crates.io Version](https://img.shields.io/crates/size/revolt-config) ![Crates.io License](https://img.shields.io/crates/l/revolt-config)                     |
| `core/database`    | [crates/core/database](crates/core/database)       | Core: Database Implementation       | ![Crates.io Version](https://img.shields.io/crates/v/revolt-database) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-database) ![Crates.io Version](https://img.shields.io/crates/size/revolt-database) ![Crates.io License](https://img.shields.io/crates/l/revolt-database)             |
| `core/files`       | [crates/core/files](crates/core/files)             | Core: S3 and encryption subroutines | ![Crates.io Version](https://img.shields.io/crates/v/revolt-files) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-files) ![Crates.io Version](https://img.shields.io/crates/size/revolt-files) ![Crates.io License](https://img.shields.io/crates/l/revolt-files)                         |
| `core/models`      | [crates/core/models](crates/core/models)           | Core: API Models                    | ![Crates.io Version](https://img.shields.io/crates/v/revolt-models) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-models) ![Crates.io Version](https://img.shields.io/crates/size/revolt-models) ![Crates.io License](https://img.shields.io/crates/l/revolt-models)                     |
| `core/permissions` | [crates/core/permissions](crates/core/permissions) | Core: Permission Logic              | ![Crates.io Version](https://img.shields.io/crates/v/revolt-permissions) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-permissions) ![Crates.io Version](https://img.shields.io/crates/size/revolt-permissions) ![Crates.io License](https://img.shields.io/crates/l/revolt-permissions) |
| `core/presence`    | [crates/core/presence](crates/core/presence)       | Core: User Presence                 | ![Crates.io Version](https://img.shields.io/crates/v/revolt-presence) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-presence) ![Crates.io Version](https://img.shields.io/crates/size/revolt-presence) ![Crates.io License](https://img.shields.io/crates/l/revolt-presence)             |
| `core/result`      | [crates/core/result](crates/core/result)           | Core: Result and Error types        | ![Crates.io Version](https://img.shields.io/crates/v/revolt-result) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-result) ![Crates.io Version](https://img.shields.io/crates/size/revolt-result) ![Crates.io License](https://img.shields.io/crates/l/revolt-result)                     |
| `core/coalesced`   | [crates/core/coalesced](crates/core/coalesced)     | Core: Coalescion service            | ![Crates.io Version](https://img.shields.io/crates/v/revolt-coalesced) ![Crates.io Version](https://img.shields.io/crates/msrv/revolt-coalesced) ![Crates.io Version](https://img.shields.io/crates/size/revolt-coalesced) ![Crates.io License](https://img.shields.io/crates/l/revolt-coalesced)         |
| `delta`            | [crates/delta](crates/delta)                       | REST API server                     | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `bonfire`          | [crates/bonfire](crates/bonfire)                   | WebSocket events server             | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `services/january` | [crates/services/january](crates/services/january) | Proxy server                        | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `services/gifbox`  | [crates/services/gifbox](crates/services/gifbox)   | Tenor proxy server                  | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `services/autumn`  | [crates/services/autumn](crates/services/autumn)   | File server                         | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `daemons/crond`    | [crates/daemons/crond](crates/daemons/crond)       | Timed data clean up daemon server   | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |
| `daemons/pushd`    | [crates/daemons/pushd](crates/daemons/pushd)       | Push notification daemon server     | ![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)                                                                                                                                                                                                                                |

</div>
<br/>

## Minimum Supported Rust Version

Rust 1.86.0 or higher.

## Development Guide

Before contributing, make yourself familiar with [our contribution guidelines](https://developers.revolt.chat/contrib.html) and the [technical documentation for this project](https://revoltchat.github.io/backend/).

Before getting started, you'll want to install:

- mise
- Docker
- Git
- mold (optional, faster compilation)
- qemu (emulation of arm64 binary for livekit)

Add 127.0.0.1 local.revolt.chat to your hosts file, even though the project is renamed to stoatchat.

> A **default.nix** is available for Nix users!
> Run `nix-shell` to activate mise.

As a heads-up, the development environment uses the following ports:

| Service                   |      Port      |
|---------------------------|:--------------:|
| MongoDB                   |     27017      |
| Redis                     |      6379      |
| MinIO                     |     14009      |
| Maildev                   | 14025<br>14080 |
| Revolt Web App            |      5173      |
| RabbitMQ                  | 5672<br>15672  |
| Livekit                   |      7880      |
| `crates/delta`            |     14702      |
| `crates/bonfire`          |     14703      |
| `crates/services/autumn`  |     14704      |
| `crates/services/january` |     14705      |
| `crates/services/gifbox`  |     14706      |

Now you can clone and build the project:

```bash
git clone https://github.com/stoatchat/stoatchat stoatchat
cd stoatchat
mise build
```

A default configuration `Revolt.toml` is present in this project that is suited for development.

If you'd like to change anything, create a `Revolt.overrides.toml` file in the projects root directory and specify relevant variables.

> [!TIP]
> Use Sentry to catch unexpected service errors:
>
> ```toml
> # Revolt.overrides.toml
> [sentry]
> api = "https://abc@your.sentry/1"
> events = "https://abc@your.sentry/1"
> files = "https://abc@your.sentry/1"
> proxy = "https://abc@your.sentry/1"
> ```

> [!TIP]
> Use Livekit Dev environment for voice chat:
>
> ```toml
> # Revolt.overrides.toml
> [hosts.livekit]
> worldwide = "ws://local.revolt.chat:7880"
> 
> [hosts.livekit.nodes.worldwide]
> url = "http://local.revolt.chat:7880"
> lat = 0.0
> lon = 0.0
> key = "worldwide"
> secret = "ZjCofRlfm6GGtjlifmNpCDkcQbEIIVC0"
> ```

> [!TIP]
> If you have port conflicts on common services, you can try the following:
>
> ```yaml
> # compose.override.yml
> services:
>   redis:
>     ports: !override
>       - "14079:6379"
>
>   database:
>     ports: !override
>       - "14017:27017"
>
>   rabbit:
>     ports: !override
>       - "14072:5672"
>       - "14672:15672"
> ```
>
> And corresponding Revolt configuration:
>
> ```toml
> #     Revolt.overrides.toml
> # and Revolt.test-overrides.toml
> [database]
> mongodb = "mongodb://127.0.0.1:14017"
> redis = "redis://127.0.0.1:14079/"
>
> [rabbit]
> port = 14072
> ```

Then continue:

```bash
# Activate the default livekit.yml
cp livekit.example.yml livekit.yml
# start other necessary services
docker compose up -d

# run each of those in separate terminals / screen sessions / tmux panes
# if you have mold use mold -run cargo run --bin <crate> instead:
# run the events server
cargo run --bin revolt-bonfire
# run the file server
cargo run --bin revolt-autumn
# run the proxy server
cargo run --bin revolt-january
# run the tenor proxy
cargo run --bin revolt-gifbox
# run the push daemon (not usually needed in regular development)
cargo run --bin revolt-pushd
# run the API server
cargo run --bin revolt-delta
```

Build the most recent version of the web client:

```bash
git clone --recursive https://github.com/stoatchat/for-web client
cd client

# update submodules if you pull new changes
# git submodule init && git submodule update

# install all packages
mise install:frozen

# build deps:
mise build:deps

# or build a specific dep (e.g. stoat.js updates):
# pnpm --filter stoat.js run build

# customise the .env
cp packages/client/.env.example packages/client/.env
```

Add the following to your packages/client/vite.config.ts right before the closing }); :
```ts
  server: {
    allowedHosts: ["local.revolt.chat"]
  }
```

```bash
# run dev server
mise dev
```

Then go to http://local.revolt.chat:5173 to create an account/login.

When signing up, go to http://localhost:14080 to find confirmation/password reset emails.

## Deployment Guide

### Cutting new crate releases

Begin by bumping crate versions:

```bash
just patch # 0.0.X
just minor # 0.X.0
just major # X.0.0
```

Then commit the changes to package files.

Proceed to publish all the new crates:

```bash
just publish
```

### Cutting new binary releases

Tag and push a new release by running:

```bash
just release
```

If you have bumped the crate versions, proceed to [GitHub releases](https://github.com/revoltchat/backend/releases/new) to create a changelog.

## Testing

First, start the required services:

```sh
docker compose -f docker-compose.db.yml up -d
```

Now run tests for whichever database:

```sh
TEST_DB=REFERENCE cargo nextest run
TEST_DB=MONGODB cargo nextest run
```

## License

The Revolt backend is generally licensed under the [GNU Affero General Public License v3.0](https://github.com/revoltchat/backend/blob/master/LICENSE).

**Individual crates may supply their own licenses!**
