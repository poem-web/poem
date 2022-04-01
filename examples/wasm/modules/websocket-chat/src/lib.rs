use futures_util::{SinkExt, StreamExt};
use poem::runtime::tokio::sync::broadcast;
use poem::{
    get, handler,
    web::{
        websocket::{Message, WebSocket},
        Data, Html, Path,
    },
    EndpointExt, IntoResponse, Route,
};

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

#[handler]
fn ws(
    Path(name): Path<String>,
    ws: WebSocket,
    sender: Data<&broadcast::Sender<String>>,
) -> impl IntoResponse {
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        poem::runtime::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    if sender.send(format!("{}: {}", name, text)).is_err() {
                        break;
                    }
                }
            }
        });

        poem::runtime::spawn(async move {
            while let Ok(msg) = receiver.recv().await {
                if sink.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });
    })
}

#[no_mangle]
fn start() {
    poem::runtime::wasi::run(Route::new().at("/", get(index)).at(
        "/ws/:name",
        get(ws.data(broadcast::channel::<String>(32).0)),
    ));
}
