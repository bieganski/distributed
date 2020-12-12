/* Copied from actix examples repository github.com/actix/examples/udp-echo
It starts a UDP server, which echos messages send to it. You can use
socat - UDP4:HOST:PORT
or
nc -u HOST PORT
to communicate with it. */

/// The API is rather involved...
/// When using streams, it best to look for working examples.
use actix::Handler;
use actix::io::SinkWrite;
use actix::{Actor, AsyncContext, Context, Message, StreamHandler};
use bytes::Bytes;
use bytes::BytesMut;
use futures::stream::{SplitSink, StreamExt};
use std::io::Result;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

/// The response type for UDP - after writing a SinkItem to a stream,
/// the underlying UDP socket will send Bytes to SocketAddr.
type SinkItem = (Bytes, SocketAddr);
/// Write end of Sink which will receive replies (echo). UdpFramed is best
/// described by its docs (cargo downloads source code and documentation, and you
/// should use a tool which makes it a single click/keystroke to view doc for a type).
/// Having said that, it is a wrapper around a UDP socket, which provides
/// a streaming interface to it. BytesCodes is a type to which transforming
/// datagrams into meaningful messages is delegated - BytesCodes simply returns
/// each datagram as it was received to the stream.
type UdpSink = SplitSink<UdpFramed<BytesCodec>, SinkItem>;

/// Actor has other handle to stream, the one for writing SinkItems. Remember,
/// this is nothing more than a wrapping around a UdpSocket.
struct UdpActor {
    sink: SinkWrite<SinkItem, UdpSink>,
}
impl Actor for UdpActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message can be derived.
struct UdpPacket(BytesMut, SocketAddr);

/// This is how you can handle individual items from the stream.
impl StreamHandler<UdpPacket> for UdpActor {
    fn handle(&mut self, msg: UdpPacket, _: &mut Context<Self>) {
        println!("Received: ({:?}, {:?})", msg.0, msg.1);
        // Write to sink transfers performs UdpSocket.send_to under the hood.
        self.sink.write((msg.0.into(), msg.1));
    }
}

impl Handler<UdpPacket> for UdpActor {
    type Result = ();
    fn handle(&mut self, msg: UdpPacket, _: &mut Context<Self>) {
        println!("Received: ({:?}, {:?})", msg.0, msg.1);
        // Write to sink transfers performs UdpSocket.send_to under the hood.
        self.sink.write((msg.0.into(), msg.1));
    }
}

/// Handling of errors when writing (sending) messages.
/// The default is to stop actor.
impl actix::io::WriteHandler<std::io::Error> for UdpActor {}

#[actix_rt::main]
/// This might look like a lot of hassle and it is. But after configuring a stream,
/// you only need to worry about message handler and actix takes care of the rest.
async fn main() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock = UdpSocket::bind(&addr).await.unwrap();
    println!(
        "Started udp server on: 127.0.0.1:{:?}",
        sock.local_addr().unwrap().port()
    );
    let (sink, stream) = UdpFramed::new(sock, BytesCodec::new()).split();
    // Actor::create is similar to actor.start, but gives access to Context when
    // creating an actor.
    let actor = UdpActor::create(|ctx| {
        // This is how you register a stream. Notice how a filter is registered - it is
        // asynchronous function, as the reason for streams is having asynchronous pipelines.
        ctx.add_stream(
            stream.filter_map(|item: Result<(BytesMut, SocketAddr)>| async {
                item.map(|(data, sender)| UdpPacket(data, sender)).ok()
            }),
        );
        let sink = SinkWrite::new(sink, ctx);

        UdpActor {
            sink
        }
    });

    let bytes = BytesMut::from("aaa");
    let packet = UdpPacket(bytes, addr);

    actor.send(packet).await.unwrap();


    // Arbiter is the event loop itself which runs futures in actix.
    // You can get arbiter for this thread using `actix::Arbiter::current` (if
    // a system was started). Think of Arbiter as a technical detail of actix,
    // it provides a `local_join` which waits for all futures assigned to Arbiter
    // running on this thread - in our case, socket never closes so this blocks forever.
    actix_rt::Arbiter::local_join().await;
}
