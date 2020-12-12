mod domain;

pub use crate::broadcast_public::*;
pub use crate::executors_public::*;
pub use crate::stable_storage_public::*;
pub use crate::system_setup_public::*;
pub use domain::*;

pub mod broadcast_public {
    use crate::executors_public::ModuleRef;
    use crate::{PlainSenderMessage, StableStorage, StubbornBroadcastModule, SystemAcknowledgmentMessage, SystemBroadcastMessage, SystemMessageContent, SystemMessageHeader, SystemMessage};
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

    pub fn build_reliable_broadcast(
        _sbeb: ModuleRef<StubbornBroadcastModule>,
        _storage: Box<dyn StableStorage>,
        _id: Uuid,
        _processes_number: usize,
        _delivered_callback: Box<dyn Fn(SystemMessage) + Send>,
    ) -> Box<dyn ReliableBroadcast> {
        unimplemented!()
    }

    pub trait StubbornBroadcast: Send {
        fn broadcast(&mut self, msg: SystemBroadcastMessage);

        fn receive_acknowledgment(&mut self, proc: Uuid, msg: SystemMessageHeader);

        fn send_acknowledgment(&mut self, proc: Uuid, msg: SystemAcknowledgmentMessage);

        fn tick(&mut self);
    }

    pub struct BasicStubbornBroadcast {
        link: Box<dyn PlainSender>,
        processes: HashSet<Uuid>,
    }

    impl StubbornBroadcast for BasicStubbornBroadcast {
    
        fn broadcast(&mut self, _: SystemBroadcastMessage) {
            todo!()
        }
        fn receive_acknowledgment(&mut self, _: Uuid, _: SystemMessageHeader) {
            todo!()
        }
        fn send_acknowledgment(&mut self, _: Uuid, _: SystemAcknowledgmentMessage) {
            todo!()
        }
        fn tick(&mut self) {
            todo!()
        }
    }

    pub fn build_stubborn_broadcast(
        link: Box<dyn PlainSender>,
        processes: HashSet<Uuid>,
    ) -> Box<dyn StubbornBroadcast> {
        Box::new(BasicStubbornBroadcast{link, processes})
    }
}

pub mod stable_storage_public {
    use std::path::PathBuf;
    use std::collections::HashMap;

    pub trait StableStorage: Send {
        fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        fn get(&self, key: &str) -> Option<Vec<u8>>;
    }


pub struct RamStorage {
    map: HashMap<String, Vec<u8>>,
}

impl RamStorage {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl Default for RamStorage {
    fn default() -> Self {
        RamStorage::new()
    }
}

impl StableStorage for RamStorage {
    fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        self.map.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.map.get(key).cloned()
    }
}

    pub fn build_stable_storage(_root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
        Box::new(RamStorage::new())
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
        RequestTick(ModId, Duration, Box<dyn FnOnce() + Send + Sync>)
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
        ticked_modules : Arc<HashMap<usize, (ModId, Receiver<std::time::Instant>, Box<dyn FnOnce() + Send + Sync>)>>,
    }

    impl System {
        pub fn request_tick<T: Handler<Tick>>(
                &mut self,
                requester: &ModuleRef<T>,
                delay: Duration) {
            let requester_cloned = requester.clone();

            self.ticker_meta_tx.send(RequestTick{
                0: requester.id, 
                1: delay, 
                2: Box::new(move || {
                        requester_cloned.send(Tick{});
                    })
            });
        }

        pub fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
            let id = self.next_mod_id;
            self.next_mod_id += 1;
            let (tx, rx) : (Sender<MessageLambda<T>>, Receiver<MessageLambda<T>>) = unbounded();
            let meta = self.executor_meta_tx.clone();
            let worker_buffer = tx;
            self.workers.lock().unwrap().insert(id, Box::new(Worker{receiver: rx, module}));
            ModuleRef{meta, worker_buffer, id}
        }
        
        pub fn new() -> Self {
            let workers_raw : HashMap<ModId, Box<dyn WorkerType + Send>> = HashMap::new();
            let workers = Arc::new(Mutex::new(workers_raw));
            
            let workers_cloned_executor = workers.clone();
            let workers_cloned_ticker = workers.clone();
            
            let ticked_modules = Arc::new(HashMap::new());
            let mut ticked_modules_cloned = ticked_modules.clone();
            
            let (executor_meta_tx, executor_meta_rx) = unbounded();
            let executor_meta_tx_cloned = executor_meta_tx.clone();

            let executor = std::thread::spawn(move || {
                loop {
                    match executor_meta_rx.recv().unwrap() {
                        NewModule(_) => {
                            unimplemented!()
                        },
                        RemoveModule(id) => {
                            // TODO it's never called for now
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

            let (ticker_meta_tx, ticker_meta_rx) = unbounded();
            let ticker = std::thread::spawn(move || {
                loop {
                    let mut sel = Select::new();
                    let oper_meta_num = sel.recv(&ticker_meta_rx);
                    for (_, (_, tick_rx, _)) in ticked_modules_cloned.iter() {
                        sel.recv(&tick_rx);
                    }
                    let oper = sel.select();
                    let idx = oper.index();
                    match idx {
                        i if i == oper_meta_num => {
                            match oper.recv(&ticker_meta_rx).unwrap() {
                                RequestTick(id, dur, lambda) => {
                                    let ticker = tick(dur);
                                    let oper_mod_num = sel.recv(&ticker);
                                    ticked_modules_cloned.insert(oper_mod_num, (id, ticker, lambda));
                                }
                            }
                        },
                        i => {
                            let (mod_id, ticker, lambda) = ticked_modules_cloned.get(&i).unwrap();
                            oper.recv(&ticker).unwrap();
                            (*lambda)(); // that simply sends Tick to ModuleRef and runs all the machinery
                        }
                    }
                }
            });

            let next_mod_id = 0;

            System{next_mod_id, workers, ticker, 
                ticker_meta_tx, executor, executor_meta_tx, ticked_modules}
        }
    }

    pub struct ModuleRef<T: 'static> {
        meta: Sender<WorkerMsg>, // informs executor about pending messages
        worker_buffer: Sender<MessageLambda<T>>,
        id: u32,
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
            self.worker_buffer.send(Box::new(message)).unwrap();
            self.meta.send(ExecuteModule{0: self.id}).unwrap();
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
                id: self.id,
                meta: self.meta.clone(),
                worker_buffer: self.worker_buffer.clone()
            }
        }
    }
}

pub mod system_setup_public {
    use crate::{Configuration, ModuleRef, ReliableBroadcast, StubbornBroadcast, System};

    pub fn setup_system(
        _system: &mut System,
        _config: Configuration,
    ) -> ModuleRef<ReliableBroadcastModule> {
        unimplemented!()
    }

    pub struct ReliableBroadcastModule {
        pub(crate) reliable_broadcast: Box<dyn ReliableBroadcast>,
    }

    pub struct StubbornBroadcastModule {
        pub(crate) stubborn_broadcast: Box<dyn StubbornBroadcast>,
    }
}
