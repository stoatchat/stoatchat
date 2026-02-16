use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use async_tungstenite::WebSocketStream;
use authifier::AuthifierEvent;
use fred::{
    error::RedisErrorKind,
    interfaces::{ClientLike, EventInterface, PubsubInterface},
    types::RedisConfig,
};
use futures::{
    channel::oneshot,
    join, pin_mut, select,
    stream::{SplitSink, SplitStream},
    FutureExt, SinkExt, StreamExt, TryStreamExt,
};
use redis_kiss::{PayloadType, REDIS_PAYLOAD_TYPE, REDIS_URI};
use revolt_config::report_internal_error;
use revolt_database::{
    events::{client::EventV1, server::ClientMessage},
    iso8601_timestamp::Timestamp,
    Database, User, UserHint,
};
use revolt_presence::{create_session, delete_session};

use async_std::{
    net::TcpStream,
    sync::{Mutex, RwLock},
    task::spawn,
};
use revolt_result::create_error;
use sentry::Level;

use crate::config::{ProtocolConfiguration, WebsocketHandshakeCallback};
use crate::events::state::{State, SubscriptionStateChange};

type WsReader = SplitStream<WebSocketStream<TcpStream>>;
type WsWriter = SplitSink<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Message>;

/// Start a new WebSocket client worker given access to the database,
/// the relevant TCP stream and the remote address of the client.
pub async fn client(db: &'static Database, stream: TcpStream, addr: SocketAddr) {
    // Upgrade the TCP connection to a WebSocket connection.
    // In this process, we also parse any additional parameters given.
    // e.g. wss://example.com?format=json&version=1
    let (sender, receiver) = oneshot::channel();
    let Ok(ws) = async_tungstenite::accept_hdr_async_with_config(
        stream,
        WebsocketHandshakeCallback::from(sender),
        None,
    )
    .await
    else {
        return;
    };

    // Verify we've received a valid config, otherwise we should just drop the connection.
    let Ok(mut config) = receiver.await else {
        return;
    };

    info!(
        "User {addr:?} provided protocol configuration (version = {}, format = {:?})",
        config.get_protocol_version(),
        config.get_protocol_format()
    );

    // Split the socket for simultaneously read and write.
    let (mut write, mut read) = ws.split();

    // If the user has not provided authentication, request information.
    if config.get_session_token().is_none() {
        while let Ok(Some(message)) = read.try_next().await {
            if let Ok(ClientMessage::Authenticate { token }) = config.decode(&message) {
                config.set_session_token(token);
                break;
            }
        }
    }

    // Try to authenticate the user.
    let Some(token) = config.get_session_token().as_ref() else {
        write
            .send(config.encode(&EventV1::Error {
                data: create_error!(InvalidSession),
            }))
            .await
            .ok();
        return;
    };

    let (user, session_id) = match User::from_token(db, token, UserHint::Any).await {
        Ok(user) => user,
        Err(err) => {
            write
                .send(config.encode(&EventV1::Error { data: err }))
                .await
                .ok();
            return;
        }
    };

    info!("User {addr:?} authenticated as @{}", user.username);

    db.update_session_last_seen(&session_id, Timestamp::now_utc())
        .await
        .ok();

    // Create local state.
    let mut state = State::from(user, session_id);
    let user_id = state.cache.user_id.clone();

    // Notify socket we have authenticated.
    if report_internal_error!(write.send(config.encode(&EventV1::Authenticated)).await).is_err() {
        return;
    }

    // Download required data to local cache and send Ready payload.
    let ready_payload = match report_internal_error!(
        state
            .generate_ready_payload(db, config.get_ready_payload_fields())
            .await
    ) {
        Ok(ready_payload) => ready_payload,
        Err(_) => return,
    };

    if report_internal_error!(write.send(config.encode(&ready_payload)).await).is_err() {
        return;
    }

    // Create presence session.
    let (first_session, session_id) = create_session(&user_id, 0).await;

    // If this was the first session, notify other users that we just went online.
    if first_session {
        state.broadcast_presence_change(true).await;
    }

    {
        // Setup channels and mutexes
        let write = Mutex::new(write);
        let subscribed = state.subscribed.clone();
        let active_servers = state.active_servers.clone();
        let (topic_signal_s, topic_signal_r) = async_channel::unbounded();

        // TODO: this needs to be rewritten
        // Create channels through which the tasks can signal to each other they need to clean up
        let (kill_signal_1_s, kill_signal_1_r) = async_channel::bounded(1);
        let (kill_signal_2_s, kill_signal_2_r) = async_channel::bounded(1);

        // Create a PubSub connection to poll on.
        let listener = listener_with_kill_signal(
            db,
            &mut state,
            addr,
            &config,
            topic_signal_r,
            kill_signal_1_r,
            &write,
            kill_signal_2_s,
        );

        // Read from WebSocket stream.
        let worker = worker_with_kill_signal(
            addr,
            subscribed,
            active_servers,
            user_id.clone(),
            &config,
            topic_signal_s,
            kill_signal_2_r,
            read,
            &write,
            kill_signal_1_s,
        );

        join!(listener, worker);
    }
    // Clean up presence session.
    let last_session = delete_session(&user_id, session_id).await;

    // If this was the last session, notify other users that we just went offline.
    if last_session {
        state.broadcast_presence_change(false).await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn listener_with_kill_signal(
    db: &'static Database,
    state: &mut State,
    addr: SocketAddr,
    config: &ProtocolConfiguration,
    topic_signal_r: async_channel::Receiver<()>,
    kill_signal_r: async_channel::Receiver<()>,
    write: &Mutex<WsWriter>,
    kill_signal_s: async_channel::Sender<()>,
) {
    listener(
        db,
        state,
        addr,
        config,
        topic_signal_r,
        kill_signal_r,
        write,
    )
    .await;
    kill_signal_s.send(()).await.ok();
}

async fn listener(
    db: &'static Database,
    state: &mut State,
    addr: SocketAddr,
    config: &ProtocolConfiguration,
    topic_signal_r: async_channel::Receiver<()>,
    kill_signal_r: async_channel::Receiver<()>,
    write: &Mutex<WsWriter>,
) {
    let stoat_config = revolt_config::config().await;
    let url = stoat_config
        .database
        .redis_pubsub
        .unwrap_or(REDIS_URI.to_string());

    let redis_config = RedisConfig::from_url(&url).unwrap();
    let subscriber = match report_internal_error!(
        fred::types::Builder::from_config(redis_config).build_subscriber_client()
    ) {
        Ok(subscriber) => subscriber,
        Err(_) => return,
    };

    if report_internal_error!(subscriber.init().await).is_err() {
        return;
    }

    // Handle Redis connection dropping
    let (clean_up_s, clean_up_r) = async_channel::bounded(1);
    let clean_up_s = Arc::new(Mutex::new(clean_up_s));
    subscriber.on_error(move |err| {
        if let RedisErrorKind::Canceled = err.kind() {
            let clean_up_s = clean_up_s.clone();
            spawn(async move {
                clean_up_s.lock().await.send(()).await.ok();
            });
        }

        Ok(())
    });

    let mut message_rx = subscriber.message_rx();
    'out: loop {
        // Check for state changes for subscriptions.
        match state.apply_state().await {
            SubscriptionStateChange::Reset => {
                if report_internal_error!(subscriber.unsubscribe_all().await).is_err() {
                    break 'out;
                }

                let subscribed = state.subscribed.read().await;
                for id in subscribed.iter() {
                    if report_internal_error!(subscriber.subscribe(id).await).is_err() {
                        break 'out;
                    }
                }

                #[cfg(debug_assertions)]
                info!("{addr:?} has reset their subscriptions");
            }
            SubscriptionStateChange::Change { add, remove } => {
                for id in remove {
                    #[cfg(debug_assertions)]
                    info!("{addr:?} unsubscribing from {id}");

                    if report_internal_error!(subscriber.unsubscribe(id).await).is_err() {
                        break 'out;
                    }
                }

                for id in add {
                    #[cfg(debug_assertions)]
                    info!("{addr:?} subscribing to {id}");

                    if report_internal_error!(subscriber.subscribe(id).await).is_err() {
                        break 'out;
                    }
                }
            }
            SubscriptionStateChange::None => {}
        }

        let t1 = message_rx.recv().fuse();
        let t2 = topic_signal_r.recv().fuse();
        let t3 = kill_signal_r.recv().fuse();
        let t4 = clean_up_r.recv().fuse();

        pin_mut!(t1, t2, t3, t4);

        select! {
            _ = t4 => {
                break 'out;
            },
            _ = t3 => {
                break 'out;
            },
            _ = t2 => {},
            message = t1 => {
                // Handle incoming events.
                let message = match report_internal_error!(message) {
                    Ok(message) => message,
                    Err(_) => break 'out
                };

                let event = match *REDIS_PAYLOAD_TYPE {
                    PayloadType::Json => message
                        .value
                        .as_str()
                        .and_then(|s| report_internal_error!(serde_json::from_str::<EventV1>(s.as_ref())).ok()),
                    PayloadType::Msgpack => message
                        .value
                        .as_bytes()
                        .and_then(|b| report_internal_error!(rmp_serde::from_slice::<EventV1>(b)).ok()),
                    PayloadType::Bincode => message
                        .value
                        .as_bytes()
                        .and_then(|b| report_internal_error!(bincode::deserialize::<EventV1>(b)).ok()),
                };

                let Some(mut event) = event else {
                    let err = format!(
                        "Failed to deserialise event for {}: `{:?}`",
                        message.channel,
                        message
                            .value
                    );

                    error!("{}", err);
                    sentry::capture_message(&err, Level::Error);
                    break 'out;
                };

                if let EventV1::Auth(auth) = &event {
                    if let AuthifierEvent::DeleteSession { session_id, .. } = auth {
                        if &state.session_id == session_id {
                            event = EventV1::Logout;
                        }
                    } else if let AuthifierEvent::DeleteAllSessions {
                        exclude_session_id, ..
                    } = auth
                    {
                        if let Some(excluded) = exclude_session_id {
                            if &state.session_id != excluded {
                                event = EventV1::Logout;
                            }
                        } else {
                            event = EventV1::Logout;
                        }
                    }
                } else {
                    let should_send = state.handle_incoming_event_v1(db, &mut event).await;
                    if !should_send {
                        continue;
                    }
                }

                let result = write.lock().await.send(config.encode(&event)).await;
                if let Err(e) = result {
                    use async_tungstenite::tungstenite::Error;
                    if !matches!(e, Error::AlreadyClosed | Error::ConnectionClosed) {
                        let err = format!("Error while sending an event to {addr:?}: {e:?}");
                        warn!("{}", err);
                        sentry::capture_message(&err, Level::Warning);
                    }

                    break 'out;
                }

                if let EventV1::Logout = event {
                    info!("User {addr:?} received log out event!");
                    break 'out;
                }
            }
        }
    }

    report_internal_error!(subscriber.quit().await).ok();
}

#[allow(clippy::too_many_arguments)]
async fn worker_with_kill_signal(
    addr: SocketAddr,
    subscribed: Arc<RwLock<HashSet<String>>>,
    active_servers: Arc<Mutex<lru_time_cache::LruCache<String, ()>>>,
    user_id: String,
    config: &ProtocolConfiguration,
    topic_signal_s: async_channel::Sender<()>,
    kill_signal_r: async_channel::Receiver<()>,
    read: WsReader,
    write: &Mutex<WsWriter>,
    kill_signal_s: async_channel::Sender<()>,
) {
    worker(
        addr,
        subscribed,
        active_servers,
        user_id,
        config,
        topic_signal_s,
        kill_signal_r,
        read,
        write,
    )
    .await;
    kill_signal_s.send(()).await.ok();
}

#[allow(clippy::too_many_arguments)]
async fn worker(
    addr: SocketAddr,
    subscribed: Arc<RwLock<HashSet<String>>>,
    active_servers: Arc<Mutex<lru_time_cache::LruCache<String, ()>>>,
    user_id: String,
    config: &ProtocolConfiguration,
    topic_signal_s: async_channel::Sender<()>,
    kill_signal_r: async_channel::Receiver<()>,
    mut read: WsReader,
    write: &Mutex<WsWriter>,
) {
    loop {
        let t1 = read.try_next().fuse();
        let t2 = kill_signal_r.recv().fuse();

        pin_mut!(t1, t2);

        select! {
            _ = t2 => {
                return;
            },
            result = t1 => {
                let msg = match result {
                    Ok(Some(msg)) => msg,
                    Ok(None) => {
                        warn!("Received a None message!");
                        sentry::capture_message("Received a None message!", Level::Warning);
                        return;
                    }
                    Err(e) => {
                        use async_tungstenite::tungstenite::Error;
                        if !matches!(e, Error::AlreadyClosed | Error::ConnectionClosed) {
                            let err = format!("Error while reading an event from {addr:?}: {e:?}");
                            warn!("{}", err);
                            sentry::capture_message(&err, Level::Warning);
                        }

                        return;
                    }
                };

                let Ok(payload) = config.decode(&msg) else {
                    continue;
                };

                match payload {
                    ClientMessage::BeginTyping { .. } => {}
                    ClientMessage::EndTyping { .. } => {}
                    ClientMessage::Subscribe { server_id } => {
                        let mut servers = active_servers.lock().await;
                        let has_item = servers.contains_key(&server_id);
                        servers.insert(server_id, ());

                        if !has_item {
                            // Poke the listener to adjust subscriptions
                            topic_signal_s.send(()).await.ok();
                        }
                    }
                    ClientMessage::Ping { data, responded } => {
                        if responded.is_none() {
                            write
                                .lock()
                                .await
                                .send(config.encode(&EventV1::Pong { data }))
                                .await
                                .ok();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
