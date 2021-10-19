use futures_util::{SinkExt, StreamExt};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{
        websocket::{Message, WebSocket},
        Data, Html, Path,
    },
    EndpointExt, IntoResponse, Route, Server,
};

/// The sample page for websocket usage
///
/// Returns a html page include simple input & submit button for communicating
/// with backend service through websocket
#[handler]
fn index() -> Html<&'static str> {
    Html(
        r###"
    <body>
        <form id="loginForm">
            Name: <input id="nameInput" type="text" />
            <button type="submit">Login</button>
        </form>
        
        <form id="sendForm" hidden>
            Text: <input id="msgInput" type="text" />
            <button type="submit">Send</button>
        </form>
        
        <textarea id="msgsArea" cols="50" rows="30" hidden></textarea>
    </body>
    <script>
        let ws;
        const loginForm = document.querySelector("#loginForm");
        const sendForm = document.querySelector("#sendForm");
        const nameInput = document.querySelector("#nameInput");
        const msgInput = document.querySelector("#msgInput");
        const msgsArea = document.querySelector("#msgsArea");
        
        nameInput.focus();

        loginForm.addEventListener("submit", function(event) {
            event.preventDefault();
            loginForm.hidden = true;
            sendForm.hidden = false;
            msgsArea.hidden = false;
            msgInput.focus();
            ws = new WebSocket("ws://127.0.0.1:3000/ws/" + nameInput.value);
            ws.onmessage = function(event) {
                msgsArea.value += event.data + "\r\n";
            }
        });
        
        sendForm.addEventListener("submit", function(event) {
            event.preventDefault();
            ws.send(msgInput.value);
            msgInput.value = "";
        });

    </script>
    "###,
    )
}

/// The websocket handler method
///
/// [`Path`] receive the path variable in the first attempt of connection
/// ws ([`WebSocket`]) contains the websocket connection object
/// sender ([`Data<T>`]) was used for exchanging data through the T (tokio
/// broadcast channel here) ws will register 2 thread, 1 for receive the message
/// sent by other service through websocket, and send into the channel init
/// before; and the other receive the message from internal channel and
/// send back to other service through websocket as the answer.
#[handler]
fn ws(
    Path(name): Path<String>,
    ws: WebSocket,
    sender: Data<&tokio::sync::broadcast::Sender<String>>,
) -> impl IntoResponse {
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    if sender.send(format!("{}: {}", name, text)).is_err() {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(msg) = receiver.recv().await {
                if sink.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });
    })
}

/// Main method in service.
///
/// ```
/// let app = Route::new().at("/", get(index)).at(
///     "/ws/:name",
///     get(ws.data(tokio::sync::broadcast::channel::<String>(32).0)),
/// );
/// ```
/// register the websocket api in router, which would take action on handling
/// the messages exchanged through websocket
/// Other details ref the doc in hello-world
///
/// usage:
/// 1. build & start the main.rs
/// 2. connect service by ws protocol through http://localhost:3000/ws/$name
/// 3. send message
/// 4. service will send message back through websocket with log print in
/// console
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index)).at(
        "/ws/:name",
        get(ws.data(tokio::sync::broadcast::channel::<String>(32).0)),
    );

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
