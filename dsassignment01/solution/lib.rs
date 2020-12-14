mod domain;

use std::path::PathBuf;
pub use crate::broadcast_public::*;
pub use crate::executors_public::*;
pub use crate::stable_storage_public::*;
pub use crate::system_setup_public::*;
pub use domain::*;

pub mod broadcast_public {
    use crate::domain::PlainSenderMessage::Acknowledge;
    use crate::domain::PlainSenderMessage::Broadcast;
    use crate::executors_public::ModuleRef;
    use crate::{PlainSenderMessage, StableStorage, StubbornBroadcastModule, SystemAcknowledgmentMessage, SystemBroadcastMessage, SystemMessageContent, SystemMessageHeader, SystemMessage};
    
    use std::collections::HashMap;
    use std::collections::HashSet;
    use uuid::Uuid;

    pub trait PlainSender: Send + Sync {
        fn send_to(&self, uuid: &Uuid, msg: PlainSenderMessage);
    }

    pub trait ReliableBroadcast: Send {
        fn broadcast(&mut self, msg: SystemMessageContent);

        fn deliver_message(&mut self, msg: SystemBroadcastMessage);

        fn receive_acknowledgment(&mut self, msg: SystemAcknowledgmentMessage);
    }

    pub struct BasicReliableBroadcast {
        sbeb: ModuleRef<StubbornBroadcastModule>,
        storage: Box<dyn StableStorage>,
        id: Uuid,
        processes_number: usize,
        delivered_callback: Box<dyn Fn(SystemMessage) + Send>,

        pending: HashMap<SystemMessageHeader, SystemMessageContent>,
        delivered: HashSet<SystemMessageHeader>,
        ack: HashMap<SystemMessageHeader, HashSet<Uuid>>,
    }

    impl ReliableBroadcast for BasicReliableBroadcast {
        
        fn broadcast(&mut self, content_msg: SystemMessageContent) {
            let header = SystemMessageHeader{message_id: self.id, message_source_id: Uuid::new_v4()};
            println!("[broadcast]: proc {:?} generated message with id {}", self.id, &header.message_source_id.to_string()[0..5]);
            self.pending.insert(header.clone(), content_msg.clone());
            // TODO store pending
            let message = SystemMessage{header, data: SystemMessageContent{msg: content_msg.msg}};
            self.sbeb.send(SystemBroadcastMessage{forwarder_id: self.id, message});
        }

        fn deliver_message(&mut self, msg: SystemBroadcastMessage) {
            if !self.pending.contains_key(&msg.message.header) {
                self.pending.insert(msg.message.header.clone(), msg.message.data.clone()); // TODO na pewno clone?
                // TODO store pending
                println!("[deliver_message]: sending ack to {:?} from {:?}", msg.forwarder_id, self.id);
                self.sbeb.send((msg.forwarder_id, SystemAcknowledgmentMessage{proc: msg.forwarder_id, hdr: msg.message.header.clone()}));
                println!("[deliver_message]: next broadcasting msg from {:?}", msg.forwarder_id);
                self.sbeb.send(SystemBroadcastMessage{forwarder_id: self.id, message: msg.message.clone()});
            }
            if !self.ack.contains_key(&msg.message.header) {
                self.ack.insert(msg.message.header.clone(), HashSet::new());
                if (self.ack.len() > self.processes_number / 2) && (!self.delivered.contains(&msg.message.header)) {
                    (self.delivered_callback)(msg.message.clone()); // TODO at least once semantics
                    self.delivered.insert(msg.message.header.clone());
                    // TODO store delivered
                    println!("[deliver_message]: sending ack to {:?} from {:?}", msg.forwarder_id, self.id);
                    println!("possible bug - sent twice?");
                    self.sbeb.send((msg.forwarder_id, SystemAcknowledgmentMessage{proc: msg.forwarder_id, hdr: msg.message.header}));
                } 
            }
        }
        fn receive_acknowledgment(&mut self, _: SystemAcknowledgmentMessage) { todo!() }
    }

    pub fn build_reliable_broadcast(
        sbeb: ModuleRef<StubbornBroadcastModule>,
        storage: Box<dyn StableStorage>,
        id: Uuid,
        processes_number: usize,
        delivered_callback: Box<dyn Fn(SystemMessage) + Send>,
    ) -> Box<dyn ReliableBroadcast> {

        Box::new(BasicReliableBroadcast{sbeb, storage, id, processes_number, 
            delivered_callback, delivered: HashSet::new(), pending: HashMap::new(), ack: HashMap::new()})
    }

    pub trait StubbornBroadcast: Send {
        fn broadcast(&mut self, msg: SystemBroadcastMessage);

        fn receive_acknowledgment(&mut self, proc: Uuid, msg: SystemMessageHeader);

        fn send_acknowledgment(&mut self, proc: Uuid, msg: SystemAcknowledgmentMessage);

        fn tick(&mut self);
    }

    struct BasicStubbornBroadcast {
        link: Box<dyn PlainSender>,
        processes: HashSet<Uuid>,
        ack: HashMap<SystemMessageHeader, HashSet<Uuid>>,
        contents: HashMap<SystemMessageHeader, SystemBroadcastMessage>,
    }
    
    use std::iter::FromIterator;

    impl StubbornBroadcast for BasicStubbornBroadcast {
    
        fn broadcast(&mut self, b_msg: SystemBroadcastMessage) {
            self.ack.insert(b_msg.message.header, HashSet::from_iter(self.processes.clone()));
            
            for id in self.processes.iter() {
                println!("[sbeb]: sending to {:?} from forwarder {:?}", id, b_msg.forwarder_id);
                self.link.send_to(id, Broadcast(b_msg.clone()));
            }

            self.contents.insert(b_msg.message.header, b_msg);
        }

        fn receive_acknowledgment(&mut self, id: Uuid, hdr_msg: SystemMessageHeader) {
            let msg_acks = self.ack.get_mut(&hdr_msg).unwrap(); 
            msg_acks.remove(&id);
            if msg_acks.is_empty() {
                self.contents.remove(&hdr_msg).unwrap();
                self.ack.remove(&hdr_msg);
            }
        }

        fn send_acknowledgment(&mut self, id: Uuid, ack_msg: SystemAcknowledgmentMessage) {
            self.link.send_to(&id, Acknowledge(ack_msg))
        }

        fn tick(&mut self) {
            for (hdr_msg, procs) in self.ack.iter() {
                for id in procs.iter() {
                    self.link.send_to(id, Broadcast(self.contents.get(&hdr_msg).unwrap().clone()));
                }
            }
        }
    }

    pub fn build_stubborn_broadcast(
        link: Box<dyn PlainSender>,
        processes: HashSet<Uuid>,
    ) -> Box<dyn StubbornBroadcast> {
        Box::new(BasicStubbornBroadcast{link, processes, ack: HashMap::new(), contents: HashMap::new()})
    }
}

pub mod stable_storage_public {
    use std::path::PathBuf;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::fs::File;
    use std::io::Write;
    use std::io::Read;

    pub trait StableStorage: Send {
        fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        fn get(&self, key: &str) -> Option<Vec<u8>>;
    }

    pub fn build_stable_storage(root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
        Box::new(BasicStableStorage{root: root_storage_dir})
    }

    struct BasicStableStorage {
        root: PathBuf,
    }

    // impl BasicReliableBroadcast {
    //     fn store_pending(&mut self, hdr_msg: &SystemMessageHeader, content_msg: &SystemMessageContent) {
    //         let key = BasicReliableBroadcast::hdr_to_key(hdr_msg);
    //         let key = std::str::from_utf8(key.as_slice()).unwrap(); // TODO FOW NOW ONLY SPECIFIC THINGS hashing etc.
    //         let value = &content_msg.msg;
    //         let res = self.storage.put(key, value.as_slice());
    //         match res {
    //             Ok(_) => (),
    //             Err(s) => println!("store_pending: {}", s),
    //         }
    //     }

    //     fn store_delivered(hdr_msg: &SystemMessageHeader) {
    //         unimplemented!()
    //     }

    //     fn hdr_to_key(hdr_msg: &SystemMessageHeader) -> Vec<u8> {
    //         let m : [u8; 16] = hdr_msg.message_id.as_bytes().clone();
    //         let p : [u8; 16] = hdr_msg.message_source_id.as_bytes().clone();
    //         [m, p].concat()
    //     }

    //     fn key_to_hdr(v: &Vec<u8>) -> SystemMessageHeader {
    //         // let m : &str = "a";
    //         // let p : &str = "a";
    //         // SystemMessageHeader{message_id: Uuid::from_bytes(m).unwrap(), message_source_id: Uuid::from_str(p).unwrap()}
    //         unimplemented!()
    //     }
    // }
    
    impl BasicStableStorage {
        fn calculate_hash<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        fn key_to_fname(&self, key: &str) -> PathBuf {
            let basename = BasicStableStorage::calculate_hash(&key).to_string();
            let mut path = self.root.clone();
            path.push(PathBuf::from(basename));
            path
        }
    }
    
    impl StableStorage for BasicStableStorage {
        fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
            let fname = self.key_to_fname(key);
            match File::create(fname) {
                Ok(mut f) => {
                    f.write_all(value).unwrap();
                    Ok(())
                },
                Err(msg) => Err(msg.to_string())
            }
        }
    
        fn get(&self, key: &str) -> Option<Vec<u8>> {
            let fname = self.key_to_fname(key);
            match File::open(fname) {
                Ok(mut f) => {
                    let mut res = Vec::new();
                    f.read_to_end(&mut res).unwrap();
                    Some(res)
                },
                Err(_) => {
                    None
                },
            }
        }
    }
}


pub mod executors_public {
    use crate::executors_public::TickerMsg::RequestTick;
    use crate::executors_public::WorkerMsg::{ExecuteModule, NewModule, RemoveModule};
    use std::sync::Arc;
    use std::fmt;
    use std::time::Duration;
    use crossbeam_channel::{unbounded, Receiver, Sender, Select, tick};
    use std::collections::HashMap;
    use std::sync::Mutex;


    type MessageLambda<T> = Box<dyn FnOnce (&mut T) + Send>;
    type ModId = u32;

    pub trait Message: fmt::Debug + Clone + Send + 'static {}
    impl<T: fmt::Debug + Clone + Send + 'static> Message for T {}

    pub trait Handler<M: Message>
    where
        M: Message,
    {
        fn handle(&mut self, msg: M);
    }

    pub enum WorkerMsg {
        NewModule(ModId),
        RemoveModule(ModId),
        ExecuteModule(ModId),
    }

    pub enum TickerMsg {
        RequestTick(ModId, Duration, Box<dyn Fn() + Send + Sync>)
    }

    #[derive(Debug, Clone)]
    pub struct Tick {}

    pub struct System {
        executor : std::thread::JoinHandle<()>,
        ticker : std::thread::JoinHandle<()>,
        executor_meta_tx : Sender<WorkerMsg>,
        ticker_meta_tx : Sender<TickerMsg>,
        next_mod_id : ModId,
        workers : Arc<Mutex<HashMap<ModId, Box<dyn WorkerType + Send>>>>,
    }

    impl System {
        pub fn request_tick<T: Handler<Tick>>(
                &mut self,
                requester: &ModuleRef<T>,
                delay: Duration) {
            let requester_cloned = requester.clone();

            self.ticker_meta_tx.send(RequestTick{
                0: requester_cloned.data.id,
                1: delay, 
                2: Box::new(move || {
                        requester_cloned.send(Tick{});
                    })
            }).unwrap();
        }

        pub fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
            let id = self.next_mod_id;
            self.next_mod_id += 1;
            let (tx, rx) : (Sender<MessageLambda<T>>, Receiver<MessageLambda<T>>) = unbounded();
            let meta = self.executor_meta_tx.clone();
            let worker_buffer = tx;
            self.workers.lock().unwrap().insert(id, Box::new(Worker{receiver: rx, module}));
            ModuleRef{data: Arc::new(ModuleRefData{meta, worker_buffer, id})}
        }
        
        pub fn new() -> Self {
            let workers_raw : HashMap<ModId, Box<dyn WorkerType + Send>> = HashMap::new();
            let workers = Arc::new(Mutex::new(workers_raw));
            
            let workers_cloned_executor = workers.clone();
            
            let (executor_meta_tx, executor_meta_rx) = unbounded();

            let executor = std::thread::spawn(move || {
                loop {
                    match executor_meta_rx.recv().unwrap() {
                        NewModule(_) => {
                            unimplemented!()
                        },
                        RemoveModule(id) => {
                            workers_cloned_executor.lock().unwrap().remove(&id).unwrap();
                        },
                        ExecuteModule(id) => {
                            let mut map = workers_cloned_executor.lock().unwrap();
                            let mut w = map.get_mut(&id).unwrap();
                            let worker : &mut Box<dyn WorkerType + Send> = &mut w;
                            worker.execute();
                        },
                    }
                }
            });

            // tx is used in request_tick function, rx is owned by ticker thread
            let (ticker_meta_tx, ticker_meta_rx) = unbounded();
            let ticker = std::thread::spawn(move || {
                let mut ticker_rxs_funs : HashMap<ModId, (Receiver<_>, Box<dyn Fn() + Send + Sync>)> = HashMap::new();
                let mut mods : HashMap<usize, ModId> = HashMap::new();

                loop {
                    let mut sel = Select::new();
                    let oper_meta_num = sel.recv(&ticker_meta_rx);
                    {
                    for (mod_id, (tick_rx, _)) in ticker_rxs_funs.iter() {                        
                        let oper_id : usize = sel.recv(&tick_rx);
                        mods.insert(oper_id, mod_id.clone());
                    }
                    }
                    let oper = sel.select();
                    let idx = oper.index();
                    match idx {
                        i if i == oper_meta_num => {
                            match oper.recv(&ticker_meta_rx).unwrap() {
                                RequestTick(id, dur, lambda) => {
                                    println!("[ticker] {:?} wants me to tick each {:?}", id, dur);
                                    let ticker = tick(dur);
                                    ticker_rxs_funs.insert(id, (ticker, lambda));
                                }
                            }
                        },
                        i => {
                            let mod_id = mods.get(&i).unwrap().clone();
                            let (ticker, lambda) = ticker_rxs_funs.get(&mod_id).unwrap();
                            oper.recv(&ticker).unwrap();
                            lambda(); // that simply sends Tick to ModuleRef and runs all the machinery
                        }
                    }
                }
            });

            let next_mod_id = 0;

            System{next_mod_id, workers, ticker, 
                ticker_meta_tx, executor, executor_meta_tx}
        }
    }

    // thanks to usage of Arc, ModuleRefData's drop() will be called exactly once
    pub struct ModuleRef<T: 'static> {
        data: Arc<ModuleRefData<T>>,
    }

    struct ModuleRefData<T: 'static> {
        meta: Sender<WorkerMsg>, // informs executor about pending messages
        worker_buffer: Sender<MessageLambda<T>>,
        id: ModId,
    }

    impl<T> Drop for ModuleRefData<T> {
        fn drop(&mut self) {
            self.meta.send(RemoveModule{0: self.id}).unwrap();
        }
    }

    pub struct Worker<T: 'static> {
        receiver: Receiver<MessageLambda<T>>,
        module: T,
    }

    pub trait WorkerType {
        fn execute(&mut self);
    }

    impl<T> WorkerType for Worker<T> {
        fn execute(&mut self) {
            let f = self.receiver.recv().unwrap();
            f(&mut self.module);
        }
    }

    impl<T> ModuleRef<T> {
        pub fn send<M: Message>(&self, msg: M)
        where
            T: Handler<M>,
        {
            let message = move |module: &mut T| {
                module.handle(msg);
            };
            self.data.worker_buffer.send(Box::new(message)).unwrap();
            self.data.meta.send(ExecuteModule{0: self.data.id}).unwrap();
        }
    }

    impl<T> fmt::Debug for ModuleRef<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            f.write_str("<ModuleRef>")
        }
    }

    impl<T> Clone for ModuleRef<T> {
        fn clone(&self) -> Self {
            ModuleRef{
                data: self.data.clone(),
            }
        }
    }
}

pub mod system_setup_public {
use crate::broadcast_public::build_reliable_broadcast;
use crate::broadcast_public::build_stubborn_broadcast;
use crate::{Configuration, ModuleRef, ReliableBroadcast, StubbornBroadcast, System};

    pub fn setup_system(
        system: &mut System,
        config: Configuration,
    ) -> ModuleRef<ReliableBroadcastModule> {

        let sbeb = build_stubborn_broadcast(config.sender, config.processes.clone());
        let sbeb = StubbornBroadcastModule{stubborn_broadcast: sbeb};
        let sbeb = system.register_module(sbeb);
        system.request_tick(&sbeb, config.retransmission_delay);

        let lurb = build_reliable_broadcast(
            sbeb, 
            config.stable_storage,
            config.self_process_identifier, 
            config.processes.len(), 
            config.delivered_callback,
        );
    
        let lurb_mod = ReliableBroadcastModule{reliable_broadcast: lurb};
        system.register_module(lurb_mod)
    }

    pub struct ReliableBroadcastModule {
        pub(crate) reliable_broadcast: Box<dyn ReliableBroadcast>,
    }

    pub struct StubbornBroadcastModule {
        pub(crate) stubborn_broadcast: Box<dyn StubbornBroadcast>,
    }
}
