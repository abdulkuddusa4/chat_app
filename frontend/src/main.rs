use dioxus::prelude::*;
use tonic::transport::Channel;

// ┌─────────────────────────────────────────────────────────────────────────┐
// │  SERVER ADDRESS — edit this one line to point at your backend           │
// └─────────────────────────────────────────────────────────────────────────┘
const SERVER_ADDR: &str = "https://poodle-flexible-carefully.ngrok-free.app";

// ── Proto modules ─────────────────────────────────────────────────────────────
mod users {
    tonic::include_proto!("users");
}
mod chat {
    tonic::include_proto!("chat");
}

use users::user_client::UserClient;
use users::{otp_request, OtpRequest, OtpVerifyRequest, otp_verify_response};

use chat::chat_client::ChatClient;
use chat::{IncomingMessage, ReceiveMessageRequest};

// ── App-level screen state ────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum AppScreen {
    Identifier,
    Otp,
    Chat,
}

// ── Global RPC state (single channel, shared everywhere) ─────────────────────
#[derive(Clone)]
struct GlobalState {
    rpc_channel: Channel,
}

// ── Data model ───────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum Side {
    Me,
    Them,
}

#[derive(Clone, PartialEq)]
struct Message {
    id: usize,
    text: String,
    side: Side,
}

#[derive(Clone, PartialEq)]
struct Chat {
    id: usize,
    contact: String,
    messages: Vec<Message>,
}

impl Chat {
    fn new(id: usize, contact: impl Into<String>) -> Self {
        Self { id, contact: contact.into(), messages: Vec::new() }
    }

    fn display_name(&self) -> &str {
        if let Some(pos) = self.contact.find('@') {
            &self.contact[..pos]
        } else {
            &self.contact
        }
    }

    fn avatar_char(&self) -> char {
        self.display_name()
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .next()
            .unwrap_or('?')
    }

    fn preview(&self) -> String {
        self.messages
            .last()
            .map(|m| {
                let prefix = match m.side {
                    Side::Me   => "You: ",
                    Side::Them => "",
                };
                let body: String = m.text.chars().take(38).collect();
                let tail = if m.text.len() > 38 { "…" } else { "" };
                format!("{prefix}{body}{tail}")
            })
            .unwrap_or_else(|| "No messages yet".to_string())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn validate_identifier(s: &str) -> Result<(), &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Please enter an email or phone number.");
    }
    let looks_like_email = s.contains('@') && s.contains('.');
    let looks_like_phone = s.chars().filter(|c| c.is_ascii_digit()).count() >= 7;
    if looks_like_email || looks_like_phone {
        Ok(())
    } else {
        Err("Enter a valid email (user@example.com) or phone (+1 555 000 0000).")
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    dioxus::launch(App);
}

// ── Root component ────────────────────────────────────────────────────────────
#[component]
fn App() -> Element {
    let mut screen     = use_signal(|| AppScreen::Identifier);
    let mut email      = use_signal(String::new);   // the address the OTP was sent to
    let mut uuid       = use_signal(String::new);   // returned by VerifyOtp

    // Build the shared gRPC channel once on startup.
    let clients = use_resource(|| async {
        let rpc_channel = Channel::from_shared(SERVER_ADDR)
            .expect("invalid SERVER_ADDR")
            .connect()
            .await
            .expect("failed to connect to gRPC server");
        GlobalState { rpc_channel }
    });

    let Some(clients) = clients.read().as_ref().cloned() else {
        return rsx! {
            div { class: "auth-bg",
                div { class: "auth-card",
                    div { class: "auth-logo", "⏳" }
                    p { class: "auth-subtitle", "Connecting to server…" }
                }
            }
            style { {STYLES} }
        };
    };

    use_context_provider(|| GlobalState { rpc_channel: clients.rpc_channel.clone() });

    rsx! {
        style { {STYLES} }
        match *screen.read() {
            AppScreen::Identifier => rsx! {
                IdentifierScreen {
                    on_success: move |ident: String| {
                        email.set(ident);
                        screen.set(AppScreen::Otp);
                    }
                }
            },
            AppScreen::Otp => rsx! {
                OtpScreen {
                    email: email.read().clone(),
                    on_success: move |user_uuid: String| {
                        uuid.set(user_uuid);
                        screen.set(AppScreen::Chat);
                    },
                    on_back: move |_| screen.set(AppScreen::Identifier),
                }
            },
            AppScreen::Chat => rsx! {
                ChatApp { my_id: uuid.read().clone() }
            },
        }
    }
}

// ── Screen 1 – Identifier ─────────────────────────────────────────────────────
#[component]
fn IdentifierScreen(on_success: EventHandler<String>) -> Element {
    let mut input   = use_signal(String::new);
    let mut error   = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let global = use_context::<GlobalState>();

    // use_callback returns a Copy handle — safe to move into multiple event handlers.
    let submit = use_callback(move |_: ()| {
        let ident = input.read().trim().to_string();

        if let Err(e) = validate_identifier(&ident) {
            error.set(e.to_owned());
            return;
        }

        loading.set(true);
        error.set(String::new());

        let channel = global.rpc_channel.clone();
        let ident_clone = ident.clone();

        spawn(async move {
            let mut user_client = UserClient::new(channel);

            let id = if ident_clone.contains('@') {
                otp_request::Id::Email(ident_clone.clone())
            } else {
                otp_request::Id::Phone(ident_clone.clone())
            };

            match user_client
                .request_otp(tonic::Request::new(OtpRequest { id: Some(id) }))
                .await
            {
                Ok(_) => {
                    on_success.call(ident_clone);
                }
                Err(e) => {
                    error.set(format!("Failed to send OTP: {}", e.message()));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "auth-bg",
            div { class: "auth-card",
                div { class: "auth-logo", "💬" }
                h1 { class: "auth-title", "DioxusChat" }
                p  { class: "auth-subtitle",
                    "Enter your email or phone number to get started"
                }

                input {
                    class: "auth-input",
                    r#type: "text",
                    placeholder: "e.g. alice@example.com  or  +1 555 000 0000",
                    value: "{input}",
                    autofocus: true,
                    disabled: *loading.read(),
                    oninput: move |e| {
                        input.set(e.value());
                        error.set(String::new());
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter && !*loading.read() { submit.call(()); }
                    },
                }

                if !error.read().is_empty() {
                    div { class: "auth-error", "⚠  {error}" }
                }

                button {
                    class: "auth-btn",
                    disabled: *loading.read(),
                    onclick: move |_| submit.call(()),
                    if *loading.read() { "Sending…" } else { "Send OTP  →" }
                }
            }
        }
    }
}

// ── Screen 2 – OTP Verification ───────────────────────────────────────────────
// on_success now carries the UUID string returned by the server.
#[component]
fn OtpScreen(
    email: String,
    on_success: EventHandler<String>,
    on_back: EventHandler<()>,
) -> Element {
    let mut otp_val = use_signal(String::new);
    let mut error   = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let global = use_context::<GlobalState>();
    let email_clone = email.clone();

    let verify = use_callback(move |_: ()| {
        let otp = otp_val.read().trim().to_string();
        if otp.len() < 4 {
            error.set("Please enter the complete OTP code.".to_string());
            return;
        }

        loading.set(true);
        error.set(String::new());

        let channel   = global.rpc_channel.clone();
        let addr      = email_clone.clone();

        spawn(async move {
            let mut user_client = UserClient::new(channel);

            match user_client
                .verify_otp(tonic::Request::new(OtpVerifyRequest {
                    email_or_phone: addr,
                    otp,
                }))
                .await
            {
                Ok(resp) => {
                    match resp.into_inner().res {
                        Some(otp_verify_response::Res::Uuid(uuid)) => {
                            on_success.call(uuid);
                        }
                        Some(otp_verify_response::Res::ErrMsg(msg)) => {
                            error.set(msg);
                            loading.set(false);
                        }
                        None => {
                            error.set("Unexpected response from server.".to_string());
                            loading.set(false);
                        }
                    }
                }
                Err(e) => {
                    error.set(format!("Verification failed: {}", e.message()));
                    loading.set(false);
                }
            }
        });
    });

    // Resend OTP
    let global2     = use_context::<GlobalState>();
    let email_resend = email.clone();
    let resend = move |_| {
        otp_val.set(String::new());
        error.set(String::new());

        let channel = global2.rpc_channel.clone();
        let addr    = email_resend.clone();

        spawn(async move {
            let mut user_client = UserClient::new(channel);
            let id = if addr.contains('@') {
                otp_request::Id::Email(addr)
            } else {
                otp_request::Id::Phone(addr)
            };
            let _ = user_client
                .request_otp(tonic::Request::new(OtpRequest { id: Some(id) }))
                .await;
        });
    };

    rsx! {
        div { class: "auth-bg",
            div { class: "auth-card",
                div { class: "auth-logo", "🔑" }
                h1 { class: "auth-title", "Enter OTP" }
                p  { class: "auth-subtitle",
                    "A code was sent to "
                    strong { "{email}" }
                }

                input {
                    class: "auth-input otp-input",
                    r#type: "text",
                    placeholder: "6-digit code",
                    maxlength: "6",
                    value: "{otp_val}",
                    autofocus: true,
                    disabled: *loading.read(),
                    oninput: move |e| {
                        let val: String = e.value().chars().take(6).collect();
                        otp_val.set(val);
                        error.set(String::new());
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter && !*loading.read() { verify.call(()); }
                    },
                }

                if !error.read().is_empty() {
                    div { class: "auth-error", "⚠  {error}" }
                }

                button {
                    class: "auth-btn",
                    disabled: *loading.read(),
                    onclick: move |_| verify.call(()),
                    if *loading.read() { "Verifying…" } else { "Verify & Continue  →" }
                }

                div { class: "auth-links",
                    button {
                        class: "auth-link",
                        disabled: *loading.read(),
                        onclick: move |_| on_back.call(()),
                        "← Change email / phone"
                    }
                    button {
                        class: "auth-link",
                        disabled: *loading.read(),
                        onclick: resend,
                        "Resend OTP"
                    }
                }
            }
        }
    }
}

// ── Screen 3 – Chat app ───────────────────────────────────────────────────────
#[component]
fn ChatApp(my_id: String) -> Element {
    let mut chats:     Signal<Vec<Chat>>        = use_signal(Vec::new);
    let mut active_id: Signal<Option<usize>>    = use_signal(|| None);
    let mut next_id:   Signal<usize>            = use_signal(|| 1usize);
    let mut modal_open:  Signal<bool>           = use_signal(|| false);
    let mut modal_input: Signal<String>         = use_signal(String::new);
    let mut modal_error: Signal<String>         = use_signal(String::new);
    let mut msg_input:   Signal<String>         = use_signal(String::new);

    let global = use_context::<GlobalState>();

    // ── Start the incoming-message stream exactly once on mount ──────────────
    // use_hook runs only on the first render — no reactive re-fires.
    use_hook(|| {
        let channel   = global.rpc_channel.clone();
        let id        = my_id.clone();

        spawn(async move {
            // Retry loop: if the stream drops, wait 2s and reconnect.
            loop {
                let mut chat_client = ChatClient::new(channel.clone());

                let stream_result = chat_client
                    .receive_incoming_messages(tonic::Request::new(
                        ReceiveMessageRequest { id: id.clone() },
                    ))
                    .await;

                let Ok(response) = stream_result else {
                    // Failed to open stream — wait then retry.
                    continue;
                };

                let mut stream = response.into_inner();

                loop {
                    match stream.message().await {
                        Ok(Some(incoming)) => {
                            let from = incoming.from_addr.clone();
                            let text = String::from_utf8_lossy(&incoming.msg).to_string();

                            // ── Step 1: find existing chat id (read lock released before any write) ──
                            let existing_id: Option<usize> = {
                                chats.read().iter().find(|c| c.contact == from).map(|c| c.id)
                            };

                            // ── Step 2: create chat if needed (no read lock held) ──
                            let chat_id = if let Some(cid) = existing_id {
                                cid
                            } else {
                                // Allocate a new id, then push the chat.
                                let cid = { *next_id.read() };
                                *next_id.write() += 1;
                                chats.write().push(Chat::new(cid, from.clone()));
                                cid
                            };

                            // ── Step 3: allocate message id ──
                            let mid = { *next_id.read() };
                            *next_id.write() += 1;

                            // ── Step 4: append message (single write lock, held only for this block) ──
                            {
                                let mut chats_w = chats.write();
                                if let Some(chat) = chats_w.iter_mut().find(|c| c.id == chat_id) {
                                    chat.messages.push(Message {
                                        id:   mid,
                                        text,
                                        side: Side::Them,
                                    });
                                }
                            }
                        }
                        // Stream ended cleanly or errored — break inner loop, then reconnect.
                        Ok(None) | Err(_) => break,
                    }
                }

            }
        });
    });

    // ── Modal helpers ─────────────────────────────────────────────────────────
    let open_modal  = move |_| { modal_input.set(String::new()); modal_error.set(String::new()); modal_open.set(true); };
    let close_modal = move |_| modal_open.set(false);

    let mut confirm_new_chat = move || {
        let contact = modal_input.read().trim().to_string();
        match validate_identifier(&contact) {
            Err(e) => modal_error.set(e.to_string()),
            Ok(()) => {
                let existing = chats.read().iter().find(|c| c.contact == contact).map(|c| c.id);
                if let Some(cid) = existing {
                    active_id.set(Some(cid));
                } else {
                    let id = *next_id.read();
                    *next_id.write() += 1;
                    chats.write().push(Chat::new(id, contact));
                    active_id.set(Some(id));
                }
                modal_open.set(false);
                msg_input.set(String::new());
            }
        }
    };

    let confirm_click = move |_| confirm_new_chat();
    let confirm_key   = move |e: Event<KeyboardData>| { if e.key() == Key::Enter { confirm_new_chat(); } };

    // ── Send message ──────────────────────────────────────────────────────────
    let channel_send = global.rpc_channel.clone();
    let my_id_send   = my_id.clone();

    let send = use_callback(move |_: ()| {
        let text = msg_input.read().trim().to_string();
        if text.is_empty() { return; }
        let Some(aid) = *active_id.read() else { return };

        let contact = chats.read().iter().find(|c| c.id == aid).map(|c| c.contact.clone());
        let Some(to_addr) = contact else { return };

        // Optimistically add the message to the UI.
        let mid = *next_id.read();
        *next_id.write() += 1;
        if let Some(chat) = chats.write().iter_mut().find(|c| c.id == aid) {
            chat.messages.push(Message { id: mid, text: text.clone(), side: Side::Me });
        }
        msg_input.set(String::new());

        // Fire-and-forget over gRPC.
        let channel    = channel_send.clone();
        let from_addr  = my_id_send.clone();

        spawn(async move {
            let mut chat_client = ChatClient::new(channel);
            let _ = chat_client
                .send_message(tonic::Request::new(IncomingMessage {
                    from_addr,
                    to_addr,
                    msg: text.into_bytes(),
                }))
                .await;
        });
    });

    let send_click = move |_| send.call(());
    let send_key   = move |e: Event<KeyboardData>| { if e.key() == Key::Enter { send.call(()); } };

    // ── Derived ───────────────────────────────────────────────────────────────
    let active_chat: Option<Chat> = active_id
        .read()
        .and_then(|aid| chats.read().iter().find(|c| c.id == aid).cloned());

    rsx! {
        div { class: "app",

            // ════ SIDEBAR ════════════════════════════════════════════════════
            div { class: "sidebar",
                div { class: "sidebar-header",
                    div { class: "sidebar-title", span { "💬" } "DioxusChat" }
                    button { class: "add-btn", title: "New chat", onclick: open_modal, "＋" }
                }

                div { class: "chat-list",
                    if chats.read().is_empty() {
                        div { class: "empty-list", "No chats yet." br {} "Tap ＋ to start one." }
                    }
                    for chat in chats.read().iter() {
                        {
                            let cid       = chat.id;
                            let name      = chat.display_name().to_string();
                            let avatar    = chat.avatar_char();
                            let preview   = chat.preview();
                            let is_active = *active_id.read() == Some(cid);
                            let cls = if is_active { "chat-item active" } else { "chat-item" };
                            rsx! {
                                div {
                                    class: "{cls}",
                                    onclick: move |_| { active_id.set(Some(cid)); msg_input.set(String::new()); },
                                    div { class: "chat-avatar", "{avatar}" }
                                    div { class: "chat-item-body",
                                        div { class: "chat-item-name",    "{name}" }
                                        div { class: "chat-item-preview", "{preview}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ════ MAIN PANEL ═════════════════════════════════════════════════
            div { class: "chat-panel",
                match active_chat {
                    None => rsx! {
                        div { class: "empty-state",
                            span { class: "empty-icon", "💬" }
                            div { class: "empty-title", "Welcome to DioxusChat" }
                            div { class: "empty-sub",
                                "Select a conversation or tap " strong { "＋" } " to start a new one."
                            }
                        }
                    },
                    Some(chat) => rsx! {
                        div { class: "chat-header",
                            div { class: "chat-header-avatar", "{chat.avatar_char()}" }
                            div {
                                div { class: "chat-header-name",    "{chat.display_name()}" }
                                div { class: "chat-header-contact", "{chat.contact}" }
                            }
                        }

                        div { class: "messages",
                            if chat.messages.is_empty() {
                                div { class: "no-messages", "Say hello to {chat.display_name()} 👋" }
                            }
                            for msg in chat.messages.iter() {
                                ChatBubble { message: msg.clone() }
                            }
                        }

                        div { class: "input-bar",
                            input {
                                class: "text-input",
                                r#type: "text",
                                placeholder: "Message {chat.display_name()}…",
                                value: "{msg_input}",
                                oninput:   move |e| msg_input.set(e.value()),
                                onkeydown: send_key,
                            }
                            button { class: "send-btn", onclick: send_click, "➤" }
                        }
                    },
                }
            }

            // ════ NEW-CHAT MODAL ══════════════════════════════════════════════
            if *modal_open.read() {
                div { class: "modal-backdrop", onclick: close_modal,
                    div {
                        class: "modal",
                        onclick: move |e| e.stop_propagation(),

                        div { class: "modal-header",
                            span { class: "modal-icon", "✉️" }
                            div {
                                div { class: "modal-title", "New Chat" }
                                div { class: "modal-sub",   "Enter the contact's email or phone number" }
                            }
                            button { class: "modal-close", onclick: close_modal, "✕" }
                        }

                        div { class: "modal-body",
                            input {
                                class: "modal-input",
                                r#type: "text",
                                placeholder: "e.g. alice@example.com or +1 555 000 0000",
                                value: "{modal_input}",
                                autofocus: true,
                                oninput:   move |e| { modal_input.set(e.value()); modal_error.set(String::new()); },
                                onkeydown: confirm_key,
                            }
                            if !modal_error.read().is_empty() {
                                div { class: "modal-error", "⚠ {modal_error}" }
                            }
                        }

                        div { class: "modal-footer",
                            button { class: "modal-btn-cancel",  onclick: close_modal,    "Cancel" }
                            button { class: "modal-btn-confirm", onclick: confirm_click,  "Start Chat" }
                        }
                    }
                }
            }
        }
    }
}

// ── Chat bubble ───────────────────────────────────────────────────────────────
#[component]
fn ChatBubble(message: Message) -> Element {
    let (row_cls, bubble_cls) = match message.side {
        Side::Me   => ("row row-me",   "bubble bubble-me"),
        Side::Them => ("row row-them", "bubble bubble-them"),
    };
    rsx! {
        div { class: "{row_cls}",
            div { class: "{bubble_cls}", "{message.text}" }
        }
    }
}

// ── Styles ────────────────────────────────────────────────────────────────────
const STYLES: &str = r#"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }

.auth-bg {
    display: flex; align-items: center; justify-content: center;
    height: 100vh; background: #0b141a;
}
.auth-card {
    background: #202c33; border-radius: 20px; padding: 48px 40px 40px;
    width: 420px; max-width: calc(100vw - 32px);
    display: flex; flex-direction: column; align-items: center; gap: 16px;
    box-shadow: 0 24px 64px rgba(0,0,0,.55);
}
.auth-logo  { font-size: 56px; line-height: 1; margin-bottom: 4px; }
.auth-title { color: #e9edef; font-size: 26px; font-weight: 700; letter-spacing: -.5px; }
.auth-subtitle { color: #8696a0; font-size: 14px; text-align: center; line-height: 1.5; max-width: 300px; }
.auth-subtitle strong { color: #e9edef; }
.auth-input {
    width: 100%; padding: 14px 18px; border-radius: 12px;
    border: 1.5px solid #2a3942; background: #111b21; color: #e9edef;
    font-size: 15px; outline: none; margin-top: 4px; transition: border-color .18s;
}
.auth-input::placeholder { color: #8696a0; }
.auth-input:focus { border-color: #00a884; }
.otp-input { font-size: 28px; font-weight: 700; letter-spacing: 10px; text-align: center; padding: 14px 10px; }
.auth-error {
    width: 100%; background: rgba(255,107,107,.10); border: 1px solid rgba(255,107,107,.35);
    color: #ff6b6b; font-size: 13px; padding: 10px 14px; border-radius: 8px;
}
.auth-btn {
    width: 100%; padding: 14px; border-radius: 12px; border: none;
    background: #00a884; color: #fff; font-size: 16px; font-weight: 700;
    cursor: pointer; margin-top: 4px; transition: background .18s, opacity .18s;
}
.auth-btn:hover    { background: #06cf9c; }
.auth-btn:active   { background: #008c70; }
.auth-btn:disabled { opacity: .5; cursor: not-allowed; }
.auth-links { display: flex; justify-content: space-between; width: 100%; margin-top: 4px; }
.auth-link { background: none; border: none; color: #00a884; font-size: 13px; cursor: pointer; padding: 4px 2px; transition: color .15s; }
.auth-link:hover    { color: #06cf9c; }
.auth-link:disabled { opacity: .45; cursor: not-allowed; }

.app { display: flex; height: 100vh; overflow: hidden; background: #0b141a; }

.sidebar { width: 300px; min-width: 300px; display: flex; flex-direction: column; background: #111b21; border-right: 1px solid #1e2b33; }
.sidebar-header { display: flex; align-items: center; justify-content: space-between; padding: 16px; background: #202c33; border-bottom: 1px solid #1e2b33; }
.sidebar-title { display: flex; align-items: center; gap: 8px; color: #e9edef; font-size: 17px; font-weight: 700; }
.add-btn { width: 34px; height: 34px; border-radius: 50%; border: none; background: #00a884; color: #fff; font-size: 20px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: background .15s, transform .1s; flex-shrink: 0; }
.add-btn:hover  { background: #06cf9c; }
.add-btn:active { transform: scale(.9); }
.chat-list { flex: 1; overflow-y: auto; }
.chat-list::-webkit-scrollbar { width: 4px; }
.chat-list::-webkit-scrollbar-thumb { background: #2a3942; border-radius: 4px; }
.empty-list { padding: 32px 16px; text-align: center; color: #8696a0; font-size: 13px; line-height: 1.6; }
.chat-item { display: flex; align-items: center; gap: 12px; padding: 12px 16px; cursor: pointer; border-bottom: 1px solid #1e2b33; transition: background .12s; }
.chat-item:hover  { background: #1e2b33; }
.chat-item.active { background: #2a3942; }
.chat-avatar { width: 46px; height: 46px; border-radius: 50%; background: #00a884; color: #fff; font-size: 20px; font-weight: 700; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.chat-item-body { flex: 1; min-width: 0; }
.chat-item-name    { color: #e9edef; font-size: 15px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.chat-item-preview { color: #8696a0; font-size: 12px; margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.chat-panel { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.empty-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; background: #0b141a; color: #8696a0; text-align: center; padding: 40px; }
.empty-icon  { font-size: 56px; opacity: .4; }
.empty-title { font-size: 22px; font-weight: 600; color: #e9edef; }
.empty-sub   { font-size: 14px; line-height: 1.6; max-width: 300px; }
.chat-header { display: flex; align-items: center; gap: 12px; padding: 12px 20px; background: #202c33; border-bottom: 1px solid #1e2b33; }
.chat-header-avatar { width: 40px; height: 40px; border-radius: 50%; background: #00a884; color: #fff; font-size: 16px; font-weight: 700; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.chat-header-name    { color: #e9edef; font-size: 16px; font-weight: 600; }
.chat-header-contact { color: #8696a0; font-size: 12px; margin-top: 1px; }
.messages { flex: 1; overflow-y: auto; padding: 20px 16px; display: flex; flex-direction: column; gap: 6px; background: #0b141a; }
.messages::-webkit-scrollbar { width: 4px; }
.messages::-webkit-scrollbar-thumb { background: #2a3942; border-radius: 4px; }
.no-messages { margin: auto; color: #8696a0; font-size: 14px; }
.row       { display: flex; }
.row-me    { justify-content: flex-end; }
.row-them  { justify-content: flex-start; }
.bubble { max-width: 68%; padding: 8px 12px; border-radius: 8px; font-size: 14px; line-height: 1.5; box-shadow: 0 1px 2px rgba(0,0,0,.3); word-break: break-word; }
.bubble-me   { background: #005c4b; color: #e9edef; border-bottom-right-radius: 2px; }
.bubble-them { background: #202c33; color: #e9edef; border-bottom-left-radius: 2px; }
.input-bar { display: flex; align-items: center; gap: 10px; padding: 10px 16px; background: #202c33; border-top: 1px solid #1e2b33; }
.text-input { flex: 1; padding: 10px 16px; border-radius: 24px; border: none; background: #2a3942; color: #e9edef; font-size: 14px; outline: none; }
.text-input::placeholder { color: #8696a0; }
.text-input:focus { background: #32444e; }
.send-btn { width: 44px; height: 44px; border-radius: 50%; border: none; background: #00a884; color: #fff; font-size: 18px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: background .15s; flex-shrink: 0; }
.send-btn:hover  { background: #06cf9c; }
.send-btn:active { background: #008c70; }

.modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,.65); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: #202c33; border-radius: 14px; width: 420px; max-width: calc(100vw - 32px); box-shadow: 0 20px 60px rgba(0,0,0,.6); overflow: hidden; }
.modal-header { display: flex; align-items: center; gap: 12px; padding: 20px 20px 16px; border-bottom: 1px solid #2a3942; }
.modal-icon  { font-size: 28px; }
.modal-title { color: #e9edef; font-size: 17px; font-weight: 700; }
.modal-sub   { color: #8696a0; font-size: 12px; margin-top: 2px; }
.modal-close { margin-left: auto; background: none; border: none; color: #8696a0; font-size: 18px; cursor: pointer; padding: 4px 6px; border-radius: 6px; transition: background .12s, color .12s; }
.modal-close:hover { background: #2a3942; color: #e9edef; }
.modal-body { padding: 20px; }
.modal-input { width: 100%; padding: 12px 16px; border-radius: 10px; border: 1.5px solid #2a3942; background: #111b21; color: #e9edef; font-size: 15px; outline: none; transition: border-color .15s; }
.modal-input::placeholder { color: #8696a0; }
.modal-input:focus { border-color: #00a884; }
.modal-error { margin-top: 10px; color: #ff6b6b; font-size: 13px; }
.modal-footer { display: flex; justify-content: flex-end; gap: 10px; padding: 12px 20px 20px; }
.modal-btn-cancel  { padding: 9px 20px; border-radius: 8px; border: 1px solid #2a3942; background: none; color: #8696a0; font-size: 14px; cursor: pointer; transition: background .12s; }
.modal-btn-cancel:hover { background: #2a3942; color: #e9edef; }
.modal-btn-confirm { padding: 9px 22px; border-radius: 8px; border: none; background: #00a884; color: #fff; font-size: 14px; font-weight: 600; cursor: pointer; transition: background .15s; }
.modal-btn-confirm:hover  { background: #06cf9c; }
.modal-btn-confirm:active { background: #008c70; }
"#;