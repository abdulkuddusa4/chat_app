use dioxus::prelude::*;

// ── App-level screen state ────────────────────────────────────────────────────
// use tonic::transport::Channel;

#[derive(Clone, PartialEq)]
enum AppScreen {
    Identifier,
    Otp,
    Chat,
}

mod users{
    tonic::include_proto!("users");
}

mod chat{
    tonic::include_proto!("chat");
}

use users::user_client::UserClient;
use users::{otp_request, otp_request_error};
use users::{OtpRequest, OtpRequestError};

use users::{otp_verify_response};
use users::{OtpVerifyRequest, OtpVerifyResponse};


// ── Data model ───────────────────────────────────────────────────────────────

/// Which side of the conversation a message belongs to.
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
        Self {
            id,
            contact: contact.into(),
            messages: Vec::new(),
        }
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

    /// Last-message preview for the sidebar.
    fn preview(&self) -> String {
        self.messages
            .last()
            .map(|m| {
                let prefix = match m.side {
                    Side::Me => "You: ",
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

/// Validate that the string looks like an email or a phone number.
fn validate_identifier(s: &str) -> Result<(), &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Please enter an email or phone number.");
    }
    // Very loose checks — just enough to give useful feedback.
    let looks_like_email = s.contains('@') && s.contains('.');
    let looks_like_phone = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .count() >= 7;
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


#[derive(Clone)]
struct GlobalState{
    rpc_channel: Channel
}

use tonic::transport::Channel;
#[component]
fn App() -> Element {
    let mut email = use_signal(String::new);
    let mut screen     = use_signal(|| AppScreen::Identifier);
    let mut identifier = use_signal(String::new);

    let clients = use_resource(|| async {
        let rpc_channel = Channel::from_static("http://[::1]:10000")
            .connect()
            .await
            .unwrap();
        GlobalState {
            rpc_channel
        }
    });

    let Some(clients) = clients.read().as_ref().cloned() else {
        return rsx! { div { "Connecting..." } };
    };
    use_context_provider(||{
        GlobalState {
            rpc_channel: clients.rpc_channel.clone()
        }
    });

    rsx! {
        style { {STYLES} }
        match *screen.read() {
            AppScreen::Identifier => rsx! {
                IdentifierScreen {
                    on_success: move |ident: String|  async move{

                        let global_context = use_context::<GlobalState>();
                        let mut user_client = UserClient::new(global_context.rpc_channel.clone());
                        user_client.request_otp(
                            tonic::Request::new(
                                OtpRequest {
                                    id: Some(otp_request::Id::Email(ident.clone()))
                                }
                            )
                        ).await.unwrap();
                        email.set(ident.clone());
                        identifier.set(format!("an email has been sent to {}", &ident));
                        screen.set(AppScreen::Otp);
                    }
                }
            },
            AppScreen::Otp => rsx! {
                OtpScreen {
                    email: email.read().clone(),
                    identifier: identifier.read().clone(),
                    on_success: move |_| {
                        screen.set(AppScreen::Chat);
                    },
                    on_back: move |_| screen.set(AppScreen::Identifier),
                }
            },
            AppScreen::Chat => rsx! {
                ChatApp { my_id: identifier.read().clone() }
            },
        }
    }
}


#[component]
fn IdentifierScreen(on_success: EventHandler<String>) -> Element {
    let mut input = use_signal(String::new);
    let mut error = use_signal(String::new);

    let mut submit = move || {
        let ident = input.read().trim().to_string();

        if let Err(e) = validate_identifier(&ident) {
            error.set("Helo brother!".to_owned());
            return;
        }
        on_success.call(ident);
    };

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
                    oninput: move |e| {
                        input.set(e.value());
                        error.set(String::new());
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter { submit(); }
                    },
                }

                if !error.read().is_empty() {
                    div { class: "auth-error", "⚠  {error}" }
                }

                button {
                    class: "auth-btn",
                    onclick: move |_| submit(),
                    "Send OTP  →"
                }
            }
        }
    }
}

// ── Screen 2 – OTP Verification ───────────────────────────────────────────────

/// Show the OTP entry form. Allows going back and resending the code.
#[component]
fn OtpScreen(
    email: String,
    identifier: String,
    on_success: EventHandler<()>,
    on_back:    EventHandler<()>,
) -> Element {
    let mut otp_val = use_signal(String::new);
    let mut error   = use_signal(String::new);

    let mut verify = move || async move{
        let otp = otp_val.read().trim().to_string();
        if otp.len() < 4 {
            error.set("Please enter the complete OTP code.".to_string());
            return;
        }
        let channel = use_context::<GlobalState>().rpc_channel.clone();
        let mut user_client = UserClient::new(channel);
        let verify_response = user_client.verify_otp(tonic::Request::new(OtpVerifyRequest{
            email_or_phone: "".to_owned(),
            otp
        })).await.unwrap().into_inner();

        match verify_response.res{
            Some(otp_verify_response::Res::Uuid(uuid)) => {
                on_success.call(());
            },
            Some(otp_verify_response::Res::ErrMsg(msg)) => todo!(),
            _ => {
                error.set("invalid otp".to_owned());
            }
        }
        
    };

    let resend = move |_| {
        otp_val.set(String::new());
        error.set(String::new());
        let eml = email.clone();
        spawn(async {
            let global_context = use_context::<GlobalState>();
            let mut user_client = UserClient::new(global_context.rpc_channel.clone());

            user_client.request_otp(
                tonic::Request::new(
                    OtpRequest {
                        id: Some(otp_request::Id::Email(eml))
                    }
                )
            ).await.unwrap();
            // identifier.set("otp ")
        });

    };

    rsx! {
        div { class: "auth-bg",
            div { class: "auth-card",
                div { class: "auth-logo", "🔑" }
                h1 { class: "auth-title", "Enter OTP" }
                p  { class: "auth-subtitle",
                    // "A verification code was sent to "
                    strong { "{identifier}" }
                }

                // OTP digit input
                input {
                    class: "auth-input otp-input",
                    r#type: "text",
                    placeholder: "6-digit code",
                    maxlength: "6",
                    value: "{otp_val}",
                    autofocus: true,
                    oninput: move |e| {
                        // digits only, max 6
                        let digits: String = e.value()
                            .chars()
                            .filter(|c| c.is_ascii_digit())
                            .take(6)
                            .collect();
                        otp_val.set(digits);
                        error.set(String::new());
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter { verify(); }
                    },
                }

                if !error.read().is_empty() {
                    div { class: "auth-error", "⚠  {error}" }
                }

                button {
                    class: "auth-btn",
                    onclick: move |_| verify(),
                    "Verify & Continue  →"
                }

                div { class: "auth-links",
                    button {
                        class: "auth-link",
                        onclick: move |_| on_back.call(()),
                        "← Change email / phone"
                    }
                    button {
                        class: "auth-link",
                        onclick: resend,
                        "Resend OTP"
                    }
                }
            }
        }
    }
}

// ── Screen 3 – Chat app ───────────────────────────────────────────────────────

/// Full chat UI shown after successful authentication.
#[component]
fn ChatApp(my_id: String) -> Element {
    // ── Global state ─────────────────────────────────────────────────────────
    let mut chats: Signal<Vec<Chat>>  = use_signal(Vec::new);
    let mut active_id: Signal<Option<usize>> = use_signal(|| None);
    let mut next_id: Signal<usize>    = use_signal(|| 1usize);

    // ── Modal state ──────────────────────────────────────────────────────────
    let mut modal_open:  Signal<bool>   = use_signal(|| false);
    let mut modal_input: Signal<String> = use_signal(String::new);
    let mut modal_error: Signal<String> = use_signal(String::new);

    // ── Message input state ──────────────────────────────────────────────────
    let mut msg_input: Signal<String> = use_signal(String::new);

    // ── Open / close modal ───────────────────────────────────────────────────
    let open_modal = move |_| {
        modal_input.set(String::new());
        modal_error.set(String::new());
        modal_open.set(true);
    };
    let close_modal = move |_| {
        modal_open.set(false);
    };

    // ── Confirm new chat ─────────────────────────────────────────────────────
    let mut confirm_new_chat = move || {
        let contact = modal_input.read().trim().to_string();
        match validate_identifier(&contact) {
            Err(e) => {
                modal_error.set(e.to_string());
            }
            Ok(()) => {
                // Check for duplicate
                let exists = chats.read().iter().any(|c| c.contact == contact);
                if exists {
                    // Just switch to the existing chat
                    if let Some(existing) = chats.read().iter().find(|c| c.contact == contact) {
                        active_id.set(Some(existing.id));
                    }
                } else {
                    let id = *next_id.read();
                    chats.write().push(Chat::new(id, contact));
                    active_id.set(Some(id));
                    *next_id.write() += 1;
                }
                modal_open.set(false);
                msg_input.set(String::new());
            }
        }
    };

    let confirm_click = move |_| confirm_new_chat();
    let confirm_key = move |e: Event<KeyboardData>| {
        if e.key() == Key::Enter { confirm_new_chat(); }
    };

    // ── Send message ─────────────────────────────────────────────────────────
    let mut send = move || {
        let text = msg_input.read().trim().to_string();
        if text.is_empty() { return; }
        let Some(aid) = *active_id.read() else { return };
        let mid = *next_id.read();
        *next_id.write() += 1;
        if let Some(chat) = chats.write().iter_mut().find(|c| c.id == aid) {
            chat.messages.push(Message { id: mid, text, side: Side::Me });
        }
        msg_input.set(String::new());
    };

    let send_click = move |_| send();
    let send_key   = move |e: Event<KeyboardData>| {
        if e.key() == Key::Enter { send(); }
    };

    // ── Derived data ─────────────────────────────────────────────────────────
    let active_chat: Option<Chat> = active_id.read().and_then(|aid| {
        chats.read().iter().find(|c| c.id == aid).cloned()
    });

    rsx! {
        div { class: "app",

            // ════════════════════════════════════════════════
            //  SIDEBAR
            // ════════════════════════════════════════════════
            div { class: "sidebar",

                div { class: "sidebar-header",
                    div { class: "sidebar-title",
                        span { "💬" }
                        "DioxusChat"
                    }
                    button {
                        class: "add-btn",
                        title: "New chat",
                        onclick: open_modal,
                        "＋"
                    }
                }

                div { class: "chat-list",
                    if chats.read().is_empty() {
                        div { class: "empty-list",
                            "No chats yet." br {}
                            "Tap ＋ to start one."
                        }
                    }
                    for chat in chats.read().iter() {
                        {
                            let cid      = chat.id;
                            let name     = chat.display_name().to_string();
                            let avatar   = chat.avatar_char();
                            let preview  = chat.preview();
                            let is_active = *active_id.read() == Some(cid);
                            let cls = if is_active { "chat-item active" } else { "chat-item" };
                            rsx! {
                                div {
                                    class: "{cls}",
                                    onclick: move |_| {
                                        active_id.set(Some(cid));
                                        msg_input.set(String::new());
                                    },
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

            // ════════════════════════════════════════════════
            //  MAIN PANEL
            // ════════════════════════════════════════════════
            div { class: "chat-panel",
                match active_chat {
                    // ── No chat selected ────────────────────
                    None => rsx! {
                        div { class: "empty-state",
                            span { class: "empty-icon", "💬" }
                            div { class: "empty-title", "Welcome to DioxusChat" }
                            div { class: "empty-sub",
                                "Select a conversation or tap " strong { "＋" } " to start a new one."
                            }
                        }
                    },
                    // ── Active chat ─────────────────────────
                    Some(chat) => rsx! {
                        // Header
                        div { class: "chat-header",
                            div { class: "chat-header-avatar", "{chat.avatar_char()}" }
                            div {
                                div { class: "chat-header-name",    "{chat.display_name()}" }
                                div { class: "chat-header-contact", "{chat.contact}" }
                            }
                        }

                        // Messages
                        div { class: "messages",
                            if chat.messages.is_empty() {
                                div { class: "no-messages",
                                    "Say hello to {chat.display_name()} 👋"
                                }
                            }
                            for msg in chat.messages.iter() {
                                ChatBubble { message: msg.clone() }
                            }
                        }

                        // Input bar
                        div { class: "input-bar",
                            input {
                                class: "text-input",
                                r#type: "text",
                                placeholder: "Message {chat.display_name()}…",
                                value: "{msg_input}",
                                oninput:  move |e| msg_input.set(e.value()),
                                onkeydown: send_key,
                            }
                            button {
                                class: "send-btn",
                                onclick: send_click,
                                "➤"
                            }
                        }
                    },
                }
            }


            if *modal_open.read() {
                div { class: "modal-backdrop", onclick: close_modal,
                    div {
                        class: "modal",
                        // stop clicks inside the modal from closing it
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
                                oninput:   move |e| {
                                    modal_input.set(e.value());
                                    modal_error.set(String::new());
                                },
                                onkeydown: confirm_key,
                            }
                            if !modal_error.read().is_empty() {
                                div { class: "modal-error", "⚠ {modal_error}" }
                            }
                        }

                        div { class: "modal-footer",
                            button { class: "modal-btn-cancel", onclick: close_modal, "Cancel" }
                            button { class: "modal-btn-confirm", onclick: confirm_click, "Start Chat" }
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


const STYLES: &str = r#"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }

/* ══════════════════════════════════════════════════════════════════════════
   AUTH SCREENS  (Identifier + OTP)
══════════════════════════════════════════════════════════════════════════ */

.auth-bg {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: #0b141a;
}

.auth-card {
    background: #202c33;
    border-radius: 20px;
    padding: 48px 40px 40px;
    width: 420px;
    max-width: calc(100vw - 32px);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    box-shadow: 0 24px 64px rgba(0,0,0,.55);
}

.auth-logo  { font-size: 56px; line-height: 1; margin-bottom: 4px; }

.auth-title {
    color: #e9edef;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -.5px;
}

.auth-subtitle {
    color: #8696a0;
    font-size: 14px;
    text-align: center;
    line-height: 1.5;
    max-width: 300px;
}
.auth-subtitle strong { color: #e9edef; }

.auth-input {
    width: 100%;
    padding: 14px 18px;
    border-radius: 12px;
    border: 1.5px solid #2a3942;
    background: #111b21;
    color: #e9edef;
    font-size: 15px;
    outline: none;
    margin-top: 4px;
    transition: border-color .18s;
}
.auth-input::placeholder { color: #8696a0; }
.auth-input:focus { border-color: #00a884; }

/* OTP: large, centred, monospace digits */
.otp-input {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 10px;
    text-align: center;
    padding: 14px 10px;
}

.auth-error {
    width: 100%;
    background: rgba(255,107,107,.10);
    border: 1px solid rgba(255,107,107,.35);
    color: #ff6b6b;
    font-size: 13px;
    padding: 10px 14px;
    border-radius: 8px;
}

.auth-btn {
    width: 100%;
    padding: 14px;
    border-radius: 12px;
    border: none;
    background: #00a884;
    color: #fff;
    font-size: 16px;
    font-weight: 700;
    cursor: pointer;
    margin-top: 4px;
    transition: background .18s, opacity .18s;
}
.auth-btn:hover    { background: #06cf9c; }
.auth-btn:active   { background: #008c70; }
.auth-btn:disabled { opacity: .5; cursor: not-allowed; }

.auth-links {
    display: flex;
    justify-content: space-between;
    width: 100%;
    margin-top: 4px;
}

.auth-link {
    background: none;
    border: none;
    color: #00a884;
    font-size: 13px;
    cursor: pointer;
    padding: 4px 2px;
    transition: color .15s, opacity .15s;
}
.auth-link:hover    { color: #06cf9c; }
.auth-link:disabled { opacity: .45; cursor: not-allowed; }

/* ══════════════════════════════════════════════════════════════════════════
   CHAT APP
══════════════════════════════════════════════════════════════════════════ */

/* ── Layout ────────────────────────────────────────────────── */
.app {
    display: flex;
    height: 100vh;
    overflow: hidden;
    background: #0b141a;
}

/* ── Sidebar ───────────────────────────────────────────────── */
.sidebar {
    width: 300px;
    min-width: 300px;
    display: flex;
    flex-direction: column;
    background: #111b21;
    border-right: 1px solid #1e2b33;
}

.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 16px;
    background: #202c33;
    border-bottom: 1px solid #1e2b33;
}

.sidebar-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #e9edef;
    font-size: 17px;
    font-weight: 700;
}

.add-btn {
    width: 34px; height: 34px;
    border-radius: 50%;
    border: none;
    background: #00a884;
    color: #fff;
    font-size: 20px;
    cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background .15s, transform .1s;
    flex-shrink: 0;
}
.add-btn:hover  { background: #06cf9c; }
.add-btn:active { transform: scale(.9); }

/* Chat list */
.chat-list {
    flex: 1;
    overflow-y: auto;
}
.chat-list::-webkit-scrollbar { width: 4px; }
.chat-list::-webkit-scrollbar-thumb { background: #2a3942; border-radius: 4px; }

.empty-list {
    padding: 32px 16px;
    text-align: center;
    color: #8696a0;
    font-size: 13px;
    line-height: 1.6;
}

.chat-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    cursor: pointer;
    border-bottom: 1px solid #1e2b33;
    transition: background .12s;
}
.chat-item:hover  { background: #1e2b33; }
.chat-item.active { background: #2a3942; }

.chat-avatar {
    width: 46px; height: 46px;
    border-radius: 50%;
    background: #00a884;
    color: #fff;
    font-size: 20px; font-weight: 700;
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
}

.chat-item-body { flex: 1; min-width: 0; }

.chat-item-name {
    color: #e9edef;
    font-size: 15px; font-weight: 600;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.chat-item-preview {
    color: #8696a0;
    font-size: 12px; margin-top: 2px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}

/* ── Main panel ────────────────────────────────────────────── */
.chat-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

/* Empty state */
.empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: #0b141a;
    color: #8696a0;
    text-align: center;
    padding: 40px;
}
.empty-icon  { font-size: 56px; opacity: .4; }
.empty-title { font-size: 22px; font-weight: 600; color: #e9edef; }
.empty-sub   { font-size: 14px; line-height: 1.6; max-width: 300px; }

/* Chat header */
.chat-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    background: #202c33;
    border-bottom: 1px solid #1e2b33;
}
.chat-header-avatar {
    width: 40px; height: 40px;
    border-radius: 50%;
    background: #00a884;
    color: #fff;
    font-size: 16px; font-weight: 700;
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
}
.chat-header-name    { color: #e9edef; font-size: 16px; font-weight: 600; }
.chat-header-contact { color: #8696a0; font-size: 12px; margin-top: 1px; }

/* Messages */
.messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: #0b141a;
}
.messages::-webkit-scrollbar { width: 4px; }
.messages::-webkit-scrollbar-thumb { background: #2a3942; border-radius: 4px; }

.no-messages {
    margin: auto;
    color: #8696a0;
    font-size: 14px;
}

/* Rows & bubbles */
.row       { display: flex; }
.row-me    { justify-content: flex-end; }
.row-them  { justify-content: flex-start; }

.bubble {
    max-width: 68%;
    padding: 8px 12px 8px 12px;
    border-radius: 8px;
    font-size: 14px;
    line-height: 1.5;
    box-shadow: 0 1px 2px rgba(0,0,0,.3);
    word-break: break-word;
}
.bubble-me {
    background: #005c4b;
    color: #e9edef;
    border-bottom-right-radius: 2px;
}
.bubble-them {
    background: #202c33;
    color: #e9edef;
    border-bottom-left-radius: 2px;
}

/* Input bar */
.input-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    background: #202c33;
    border-top: 1px solid #1e2b33;
}
.text-input {
    flex: 1;
    padding: 10px 16px;
    border-radius: 24px;
    border: none;
    background: #2a3942;
    color: #e9edef;
    font-size: 14px;
    outline: none;
}
.text-input::placeholder { color: #8696a0; }
.text-input:focus { background: #32444e; }

.send-btn {
    width: 44px; height: 44px;
    border-radius: 50%;
    border: none;
    background: #00a884;
    color: #fff;
    font-size: 18px;
    cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background .15s;
    flex-shrink: 0;
}
.send-btn:hover  { background: #06cf9c; }
.send-btn:active { background: #008c70; }

/* ── Modal ─────────────────────────────────────────────────── */
.modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
}

.modal {
    background: #202c33;
    border-radius: 14px;
    width: 420px;
    max-width: calc(100vw - 32px);
    box-shadow: 0 20px 60px rgba(0,0,0,.6);
    overflow: hidden;
}

.modal-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px 20px 16px;
    border-bottom: 1px solid #2a3942;
}
.modal-icon  { font-size: 28px; }
.modal-title { color: #e9edef; font-size: 17px; font-weight: 700; }
.modal-sub   { color: #8696a0; font-size: 12px; margin-top: 2px; }

.modal-close {
    margin-left: auto;
    background: none;
    border: none;
    color: #8696a0;
    font-size: 18px;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    transition: background .12s, color .12s;
}
.modal-close:hover { background: #2a3942; color: #e9edef; }

.modal-body { padding: 20px; }

.modal-input {
    width: 100%;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1.5px solid #2a3942;
    background: #111b21;
    color: #e9edef;
    font-size: 15px;
    outline: none;
    transition: border-color .15s;
}
.modal-input::placeholder { color: #8696a0; }
.modal-input:focus { border-color: #00a884; }

.modal-error {
    margin-top: 10px;
    color: #ff6b6b;
    font-size: 13px;
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px 20px;
}

.modal-btn-cancel {
    padding: 9px 20px;
    border-radius: 8px;
    border: 1px solid #2a3942;
    background: none;
    color: #8696a0;
    font-size: 14px;
    cursor: pointer;
    transition: background .12s;
}
.modal-btn-cancel:hover { background: #2a3942; color: #e9edef; }

.modal-btn-confirm {
    padding: 9px 22px;
    border-radius: 8px;
    border: none;
    background: #00a884;
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background .15s;
}
.modal-btn-confirm:hover  { background: #06cf9c; }
.modal-btn-confirm:active { background: #008c70; }
"#;