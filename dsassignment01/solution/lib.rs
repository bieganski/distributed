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

    pub fn build_stubborn_broadcast(
        _link: Box<dyn PlainSender>,
        _processes: HashSet<Uuid>,
    ) -> Box<dyn StubbornBroadcast> {
        unimplemented!()
    }
}

pub mod stable_storage_public {
    use std::path::PathBuf;

    pub trait StableStorage: Send {
        fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        fn get(&self, key: &str) -> Option<Vec<u8>>;
    }

    pub fn build_stable_storage(_root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
        unimplemented!()
    }
}

pub mod executors_public {
    use crate::executors_public::WorkerMsg::{ExecuteModule, NewModule, RemoveModule};
    use std::sync::Arc;
    use std::fmt;
    use std::time::Duration;
    use crossbeam_channel::{unbounded, Receiver, Sender};
    use std::collections::HashMap;
    use std::sync::Mutex;


    type MessageLambda<T> = Box<dyn FnOnce (&mut T) + Send>;
    type ModId = u32;

    // use crate system_setup_public::ReliableBroadcastModule;

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

    #[derive(Debug, Clone)]
    pub struct Tick {}

    pub struct System {
        executor : std::thread::JoinHandle<()>,
        meta_tx : Sender<WorkerMsg>,
        next_mod_id : ModId,
        workers : Arc<Mutex<HashMap<ModId, Box<dyn WorkerType + Send>>>>,
    }

    impl System {
        pub fn request_tick<T: Handler<Tick>>(&mut self, _requester: &ModuleRef<T>, _delay: Duration) {
            unimplemented!()
        }

        pub fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
            let id = self.next_mod_id;
            self.next_mod_id += 1;
            let (tx, rx) : (Sender<MessageLambda<T>>, Receiver<MessageLambda<T>>) = unbounded();
            let meta = self.meta_tx.clone();
            let worker_buffer = tx;
            self.workers.lock().unwrap().insert(id, Box::new(Worker{receiver: rx, module}));
            ModuleRef{meta, worker_buffer, id}
        }
        

        pub fn new() -> Self {
            let (meta_tx, meta_rx) = unbounded();
            let workers_raw : HashMap<ModId, Box<dyn WorkerType + Send>> = HashMap::new();
            let workers = Arc::new(Mutex::new(workers_raw));
            let workers_cloned = workers.clone();
            
            let executor = std::thread::spawn(move || {
                loop {
                    match meta_rx.recv().unwrap() {
                        NewModule(_) => {
                            unimplemented!()
                        },
                        RemoveModule(id) => {
                            workers_cloned.lock().unwrap().remove(&id).unwrap();
                        },
                        ExecuteModule(id) => {
                            let mut map = workers_cloned.lock().unwrap();
                            let mut w = map.get_mut(&id).unwrap();
                            let worker : &mut Box<dyn WorkerType + Send> = &mut w;
                            worker.execute();
                        },
                    }
                }
            });
            let next_mod_id = 0;
            System{next_mod_id, workers, meta_tx, executor}
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
