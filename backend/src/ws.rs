use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use tokio::sync::broadcast;

use crate::fhir::FhirObservation;

#[derive(Clone)]
pub struct WsHub {
    pub tx: broadcast::Sender<FhirObservation>,
}

pub struct WsSession {
    rx: broadcast::Receiver<FhirObservation>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // clone/resubscribe so we can move it into the closure
        let mut rx = self.rx.resubscribe();

        ctx.run_interval(std::time::Duration::from_millis(250), move |_, ctx| {
            // Drain all queued messages quickly each tick
            while let Ok(obs) = rx.try_recv() {
                if let Ok(txt) = serde_json::to_string(&obs) {
                    ctx.text(txt);
                }
            }
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(m)) => ctx.pong(&m),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Close(r)) => {
                ctx.close(r);
                ctx.stop();
            }
            // Optional: respond to text/binary if you want; otherwise ignore
            _ => {}
        }
    }
}

pub async fn ws_live(
    req: HttpRequest,
    stream: web::Payload,
    hub: web::Data<WsHub>,
) -> Result<HttpResponse, Error> {
    let rx = hub.tx.subscribe();
    ws::start(WsSession { rx }, &req, stream)
}