use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Duration;

use actix::io::SinkWrite;
use actix::{Actor, Addr, AsyncContext, Context, Message, StreamHandler};
use bytes::{Bytes, BytesMut};
use futures::stream::{SplitSink, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use uuid::Uuid;

type SinkItem = (Bytes, SocketAddr);
type UdpSink = SplitSink<UdpFramed<BytesCodec>, SinkItem>;

// TODO add whatever fields necessary.
pub struct FailureDetectorActor {
    sink: SinkWrite<SinkItem, UdpSink>,
    addresses: HashMap<Uuid, SocketAddr>,
    id: Uuid,
}

impl Actor for FailureDetectorActor {
    type Context = Context<Self>;
}

/// New operation arrived on socket.
impl StreamHandler<DetectorOperationUdp> for FailureDetectorActor {
    fn handle(&mut self, item: DetectorOperationUdp, _ctx: &mut Self::Context) {
        let detector_operation = item.0;

        let _ = match detector_operation {
            DetectorOperation::HeartbeatRequest(uuid) => None,
            DetectorOperation::HeartbeatResponse(uuid) => None,
            DetectorOperation::UpRequest => None,
            DetectorOperation::UpInfo(vec) => None,
        };

        panic!("message type not supported");
    }
}

impl actix::io::WriteHandler<std::io::Error> for FailureDetectorActor {}

impl FailureDetectorActor {
    pub async fn new(
        delay: Duration,
        addresses: &HashMap<Uuid, SocketAddr>,
        ident: Uuid,
        all_idents: HashSet<Uuid>,
    ) -> Addr<Self> {
        let addr = addresses.get(&ident).unwrap();
        let sock = UdpSocket::bind(addr).await.unwrap();

        let (sink, stream) = UdpFramed::new(sock, BytesCodec::new()).split();

        FailureDetectorActor::create(|ctx| {
            // This sets up Udp stream.
            ctx.add_stream(stream.filter_map(
                |item: std::io::Result<(BytesMut, SocketAddr)>| async {
                    item.map(|(data, sender)| {
                        DetectorOperationUdp(
                            bincode::deserialize(data.as_ref())
                                .expect("Invalid format of detector operation!"),
                            sender,
                        )
                    })
                    .ok()
                },
            ));

            // Tick takes care of broadcast.
            ctx.run_interval(delay, move |a, _| {
                a.tick();
            });

            // When you need to send UDP message, use this sink.write((Bytes, SocketAddr)).
            let sink = SinkWrite::new(sink, ctx);

            // TODO finish FailureDetectorActor type and its construction here.
            // loop {
            //    ; 
            // }
            FailureDetectorActor {
                sink: sink,
                addresses: addresses.clone(),
                id: ident,
            }
            // unimplemented!()
        })
    }

    /// Called periodically to check send broadcast and update alive processes.
    fn tick(&mut self) {
        // TODO finish tick - it is registered in `new`.
        for addr in self.addresses.values() {
            println!("tick {:?}", addr);
            let bytes = bincode::serialize(&DetectorOperation::HeartbeatRequest(self.id)).unwrap();
            self.sink.write((bytes.into(), *addr));
        }
        
        // unimplemented!()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DetectorOperationUdp(DetectorOperation, SocketAddr);

#[derive(Serialize, Deserialize)]
pub enum DetectorOperation {
    /// Process with uuid sends heartbeat.
    HeartbeatRequest(Uuid),
    /// Response to heartbeat, contains uuid of the receiver of HeartbeatRequest.
    HeartbeatResponse(Uuid),
    /// Request to receive information about working processes.
    UpRequest,
    /// Vector of processes which are up according to UpRequest receiver.
    UpInfo(Vec<Uuid>),
}
