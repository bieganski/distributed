mod domain;

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
    use std::convert::TryInto;

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
        storage: StorageConnector,
        id: Uuid,
        processes_number: usize,
        delivered_callback: Box<dyn Fn(SystemMessage) + Send>,

        pending: HashMap<SystemMessageHeader, SystemMessageContent>,
        delivered: HashSet<SystemMessageHeader>,
        ack: HashMap<SystemMessageHeader, HashSet<Uuid>>,
    }

    impl BasicReliableBroadcast {
        pub fn new(
            sbeb: ModuleRef<StubbornBroadcastModule>,
            storage: Box<dyn StableStorage>,
            id: Uuid,
            processes_number: usize,
            delivered_callback: Box<dyn Fn(SystemMessage) + Send>,
        ) -> Self {
    
            let storage = StorageConnector{storage, free_pending_id: MSG_FIRST_ID, free_delivered_id: MSG_FIRST_ID};
            let ack = HashMap::new();
            let delivered = storage.restore_delivered(id);
            let delivered = HashSet::from_iter(delivered);
            let pending = storage.restore_pending(id);
            for (hdr, content) in pending.iter() {
                sbeb.send(SystemBroadcastMessage{message: SystemMessage{data: content.clone(), header: hdr.clone()}, forwarder_id: id.clone()});
            }
            Self{sbeb, storage, id, processes_number, delivered_callback, delivered, pending, ack}
        }
    }

    pub struct StorageConnector {
        storage: Box<dyn StableStorage>,
        free_pending_id: usize,
        free_delivered_id: usize,
    }

    pub enum StorageType {
        DELIVERED,
        PENDING,
    }

    static MSG_FIRST_ID  : usize = 0;
    static MSG_HDR_BYTES : usize = 32;

    impl StorageConnector {
        
        pub fn get_key_name(storage_type: StorageType, proc_id: &Uuid, num: usize) -> String {
            let prefix = match storage_type {
                StorageType::DELIVERED => "delivered",
                StorageType::PENDING => "pending",
            };
            let id_str = proc_id.to_string();
            [prefix, &id_str, &num.to_string()].concat()
        }

        pub fn restore_pending(&self, id : Uuid) -> HashMap<SystemMessageHeader, SystemMessageContent> {
            let mut res = HashMap::new();    
            let mut num = MSG_FIRST_ID;
            let mut key = Self::get_key_name(StorageType::PENDING, &id, num);
            while let Some(hdr_content_bytes) = self.storage.get(&key) {
                let header = Self::bytes_to_hdr(&hdr_content_bytes);
                let data = SystemMessageContent{msg: hdr_content_bytes[MSG_HDR_BYTES..].to_vec()};
                res.insert(header, data);
                num += 1;
                key = Self::get_key_name(StorageType::PENDING, &id, num);
            }
            res
        }
        
        // TODO unpub
        pub fn restore_delivered(&self, id: Uuid) -> Vec<SystemMessageHeader> {
            let mut res = Vec::new();
            let mut num = MSG_FIRST_ID;
            let mut key = Self::get_key_name(StorageType::DELIVERED, &id, num);
            while let Some(hdr_content_bytes) = self.storage.get(&key) {
                let msg = Self::bytes_to_hdr(&hdr_content_bytes);
                res.push(msg);
                num += 1;
                key = Self::get_key_name(StorageType::DELIVERED, &id, num);
            }
            res
        }

        // stores message content as value
        pub fn store_pending(&mut self, hdr_msg: &SystemMessageHeader, content_msg: &SystemMessageContent) {
            let key = Self::get_key_name(StorageType::PENDING, &hdr_msg.message_source_id, self.free_pending_id);
            self.free_pending_id += 1;
            let mut bytes = Self::hdr_to_bytes(&hdr_msg);
            bytes.extend(&content_msg.msg);
            let res = self.storage.put(&key, &bytes);

            match res {
                Ok(_) => (),
                Err(s) => println!("[StorageConnector][store_pending]: {}", s),
            }
        }

        // stores SystemHeader as value
        pub fn store_delivered(&mut self, hdr_msg: &SystemMessageHeader) {
            let key = Self::get_key_name(StorageType::DELIVERED, &hdr_msg.message_source_id, self.free_delivered_id);
            let res = self.storage.put(&key, &Self::hdr_to_bytes(hdr_msg));
            
            match res {
                Ok(_) => (),
                Err(s) => log::error!("[StorageConnector][store_delivered]: {}", s),
            }
        }

        fn hdr_to_bytes(hdr_msg: &SystemMessageHeader) -> Vec<u8> {
            let m : [u8; 16] = hdr_msg.message_id.as_bytes().clone();
            let s : [u8; 16] = hdr_msg.message_source_id.as_bytes().clone();
            [m, s].concat()
        }

        fn bytes_to_hdr(v: &Vec<u8>) -> SystemMessageHeader {
            let m : [u8; 16] = v.as_slice()[..16].try_into().expect("could not retrieve message id");
            let m = Uuid::from_bytes(m);
            let s : [u8; 16] = v.as_slice()[16..32].try_into().expect("could not retrieve message source id");
            let s = Uuid::from_bytes(s);
            SystemMessageHeader{message_id: m, message_source_id: s}
        }
    }
    impl ReliableBroadcast for BasicReliableBroadcast {
        
        fn broadcast(&mut self, content_msg: SystemMessageContent) {
            let header = SystemMessageHeader{message_id: Uuid::new_v4(), message_source_id: self.id};
            // log::info!("[broadcast]: proc {:?} generated message with id {}", self.id, &header.message_id.to_string()[0..5]);
            self.storage.store_pending(&header, &content_msg.clone());
            self.pending.insert(header.clone(), content_msg.clone());
            let message = SystemMessage{header, data: SystemMessageContent{msg: content_msg.msg}};
            self.sbeb.send(SystemBroadcastMessage{forwarder_id: self.id, message});
        }

        fn deliver_message(&mut self, msg: SystemBroadcastMessage) {
            // log::info!("[deliver_message]: sending ack to {:?} from {:?}", msg.forwarder_id, self.id);
            self.sbeb.send((msg.forwarder_id, SystemAcknowledgmentMessage{proc: self.id, hdr: msg.message.header.clone()}));

            if !self.pending.contains_key(&msg.message.header) {
                self.pending.insert(msg.message.header.clone(), msg.message.data.clone());
                self.storage.store_pending(&msg.message.header, &msg.message.data);
                // log::info!("[deliver_message]: next broadcasting msg from {:?}", msg.forwarder_id);
                self.sbeb.send(SystemBroadcastMessage{forwarder_id: self.id, message: msg.message.clone()});
            }
            if !self.ack.contains_key(&msg.message.header) {
                self.ack.insert(msg.message.header.clone(), HashSet::new());
            }
            let ack_m : &mut HashSet<_> = self.ack.get_mut(&msg.message.header).unwrap(); // that 'unwrap' won't panic
            if !ack_m.contains(&msg.forwarder_id) {
                ack_m.insert(msg.forwarder_id.clone());
                if (self.ack.len() > self.processes_number / 2) && (!self.delivered.contains(&msg.message.header)) {
                    (self.delivered_callback)(msg.message.clone()); // 'at least once' semantics
                    
                    self.storage.store_delivered(&msg.message.header);
                    self.delivered.insert(msg.message.header.clone());
                } 
            }
        }
        fn receive_acknowledgment(&mut self, ack: SystemAcknowledgmentMessage) {
            self.sbeb.send((ack.proc, ack.hdr));
        }
    }

    pub fn build_reliable_broadcast(
        sbeb: ModuleRef<StubbornBroadcastModule>,
        storage: Box<dyn StableStorage>,
        id: Uuid,
        processes_number: usize,
        delivered_callback: Box<dyn Fn(SystemMessage) + Send>,
    ) -> Box<dyn ReliableBroadcast> {
        Box::new(
            BasicReliableBroadcast::new(
                sbeb, storage, id, processes_number, delivered_callback
            )
        )
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
                self.link.send_to(id, Broadcast(b_msg.clone()));
            }

            self.contents.insert(b_msg.message.header, b_msg);
        }

        fn receive_acknowledgment(&mut self, id: Uuid, hdr_msg: SystemMessageHeader) {
            let msg_acks = self.ack.get_mut(&hdr_msg);
            match msg_acks {
                None => {
                    () // sent to me by mistake
                },
                Some(msg_acks) => {
                    msg_acks.remove(&id);
                    if msg_acks.is_empty() {
                        self.contents.remove(&hdr_msg);
                        self.ack.remove(&hdr_msg);
                    }
                }
            }
            
        }

        fn send_acknowledgment(&mut self, id: Uuid, ack_msg: SystemAcknowledgmentMessage) {
            self.link.send_to(&id, Acknowledge(ack_msg))
        }

        fn tick(&mut self) {
            for (hdr_msg, procs) in self.ack.iter() {
                for id in procs.iter() {
                    let content = self.contents.get(&hdr_msg);
                    match content {
                        None => log::error!("[tick][internal] 'contents' array inconsistency!"),
                        Some(content) => self.link.send_to(id, Broadcast(content.clone())),
                    }
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
        match std::fs::create_dir_all(&root_storage_dir) {
            Ok(_) => (),
            Err(err) => log::error!("[build_stable_storage] cannot create directory! '{}'", err),
        }
        Box::new(BasicStableStorage{root: root_storage_dir})
    }

    struct BasicStableStorage {
        pub root: PathBuf,
    }

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
            log::debug!("[BasicStableStorage] put: trying to open file {:?}", &fname);
            match File::create(fname.clone()) {
                Ok(mut f) => {
                    f.write_all(value).unwrap_or_else(|_| {log::error!("[fs] cannot write to {:?}", &fname);});
                    Ok(())
                },
                Err(msg) => Err(msg.to_string())
            }
        }
    
        fn get(&self, key: &str) -> Option<Vec<u8>> {
            let fname = self.key_to_fname(key);
            log::debug!("[BasicStableStorage] get: trying to open file {:?}", &fname);
            match File::open(fname.clone()) {
                Ok(mut f) => {
                    let mut res = Vec::new();
                    f.read_to_end(&mut res).unwrap_or_else(|_| {
                        log::error!("[fs] cannot read from {:?}", &fname);
                        return 0;
                    });
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


    #[allow(dead_code)]
    pub struct System {
        // use those handles if need graceful joining, otherwise unused 
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
            }).unwrap_or_else(|_| log::error!("[request_tick] cannot send value via crossbeam channel!"));
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
                    let msg = executor_meta_rx.recv();
                    if msg.is_err() {
                        log::error!("[System::new()][executor] - error receiving message via crossbeam channel!");
                        continue;
                    }
                    // that unwrap won't panic
                    match msg.unwrap() {
                        NewModule(_) => {
                            unimplemented!()
                        },
                        RemoveModule(id) => {
                            match workers_cloned_executor.lock() {
                                Ok(mut res) => match res.remove(&id) {
                                                    Some(_) => (),
                                                    None => log::error!("[System::new()][executor][internal] double worker remove detected!"),
                                                }
                                Err(_)      => ()
                            }
                        },
                        ExecuteModule(id) => {
                            match workers_cloned_executor.lock() {
                                Ok(mut workers) => {
                                    match workers.get_mut(&id) {
                                        Some(mut w) => {
                                            let worker : &mut Box<dyn WorkerType + Send> = &mut w;
                                            worker.execute();
                                        },
                                        None => log::error!("[System::new()][executor][internal] malformed 'workers' map, probably bad adding new module"),
                                    }
                                }
                                Err(_) => log::error!("[System::new()][executor][internal] mutex lock error!"),
                            }
                            
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
                            let msg = oper.recv(&ticker_meta_rx);
                            if msg.is_err() {
                                log::error!("[System][ticker] error reading from crossbeam channel!");
                                continue;
                            }
                            // that unwrap won't panic
                            match msg.unwrap() {
                                RequestTick(id, dur, lambda) => {
                                    log::debug!("[System][ticker] {:?} wants me to tick each {:?}", id, dur);
                                    let ticker = tick(dur);
                                    ticker_rxs_funs.insert(id, (ticker, lambda));
                                }
                            }
                        },
                        i => {
                            let chain = mods.get(&i)
                                .and_then(|id| {ticker_rxs_funs.get(&id)})
                                .and_then(|(ticker, lambda)| {lambda(); oper.recv(&ticker).ok()});

                            if chain.is_none() {
                                log::error!("[System][ticker] problems reveiving tick message..");
                            }
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
            if self.meta.send(RemoveModule{0: self.id}).is_err() {
                log::error!("[ModuleRefData][Drop] problem sending to 'meta' via crossbeam channel!");
            }
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
            match self.receiver.recv() {
                Ok(f) => f(&mut self.module),
                Err(_) => log::error!("[WorkerType][execute] problem receiving message from 'receiver' via crossbeam channel!"),
            }
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
            if self.data.worker_buffer.send(Box::new(message)).is_err() {
                log::error!("[ModuleRef][send] problem sending to 'worker_buffer' via crossbeam channel!");
                return;
            }
            if self.data.meta.send(ExecuteModule{0: self.data.id}).is_err() {
                log::error!("[ModuleRef][send] problem sending to 'meta' via crossbeam channel!");
                return;
            }
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
