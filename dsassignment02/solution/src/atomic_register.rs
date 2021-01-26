pub mod atomic_register_public {
    use crate::domain::SectorVec;
    use std::collections::HashSet;
    use std::collections::HashMap;

    use crate::{
        ClientRegisterCommand, OperationComplete, RegisterClient, SectorsManager, StableStorage,
        SystemRegisterCommand,
    };
    use std::sync::Arc;

    #[async_trait::async_trait]
    pub trait AtomicRegister: Send + 'static {
        /// Send client command to the register. After it is completed, we expect
        /// callback to be called. Note that completion of client command happens after
        /// delivery of multiple system commands to the register, as the algorithm specifies.
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<dyn FnOnce(OperationComplete) + Send + Sync>,
        );

        /// Send system command to the register.
        async fn system_command(&mut self, cmd: SystemRegisterCommand);
    }

    /// Idents are numbered starting at 1 (up to the number of processes in the system).
    /// Storage for atomic register algorithm data is separated into StableStorage.
    /// Communication with other processes of the system is to be done by register_client.
    /// And sectors must be stored in the sectors_manager instance.
    pub async fn build_atomic_register(
        self_ident: u8,
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
    ) -> (Box<dyn AtomicRegister>, Option<ClientRegisterCommand>) {
        build_atomic_register_generic(self_ident, metadata, register_client, sectors_manager, processes_count, 
            Arc::new(tokio::sync::Mutex::new(HashMap::new())), 0xdeadbeef).await
    }
    
    
    pub async fn build_atomic_register_generic(
        self_ident: u8,
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
        msg_owners: Arc<tokio::sync::Mutex<HashMap<uuid::Uuid, usize>>>,
        my_idx: usize,
    ) -> (Box<dyn AtomicRegister>, Option<ClientRegisterCommand>) {
        // TODO that None below handling
        let state = SystemNodeState{
            ts: Ts(0),
            wr: Rank(self_ident),
            writeval: SectorVec(Vec::new()),
            readval: SectorVec(Vec::new()),
            rid: RequestId(0),
            readlist: HashMap::<Rank, (Ts, Rank, SectorVec)>::new(),
            acklist: HashSet::<Rank>::new(),
            reading: false,
            writing: false,
            operation_complete: None,
            msg_owners,
            my_idx,
        };
        (Box::new(BasicAtomicRegister{self_id: Rank(self_ident), state, metadata, register_client, sectors_manager, processes_count}), None)
        
        // enable this for testing 'client_response', however it should work with BasicAtomicRegister, providing that it's mature enough 
        // (
        //     Box::new(TestClientOkAtomicRegister{}), 
        //     None
        // )
    }

    // timestamp
    #[derive(Debug, Clone, Copy)]
    struct Ts(u64);

    // id of read operation
    #[derive(Copy, Clone, Debug)]
    struct RequestId(u64);

    // process (and 'wr', at most 'rank')
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Rank(u8);

    // impl std::ops::Add<u64> for RequestId {
    //     type Output = RequestId;
    
    //     fn add(self, rhs: u64) -> RequestId {
    //         RequestId(self.0 + rhs)
    //     }
    // }

    impl std::ops::AddAssign<u64> for RequestId {
        fn add_assign(&mut self, rhs: u64) {
            self.0 += rhs;
        }
    }

    struct SystemNodeState {
        ts: Ts,
        wr: Rank,
        writeval: SectorVec,
        readval: SectorVec,
        rid: RequestId,
        readlist: HashMap<Rank, (Ts, Rank, SectorVec)>,
        acklist: HashSet<Rank>,
        reading: bool,
        writing: bool,
        operation_complete: Option<Box<dyn FnOnce(OperationComplete) + Send + Sync>>,
        msg_owners: Arc<tokio::sync::Mutex<HashMap<uuid::Uuid, usize>>>, // used only when using multiple registers
        my_idx: usize, // used only when using multiple registers
    }

    struct BasicAtomicRegister {
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
        state: SystemNodeState,
        self_id: Rank,
    }

    use crate::ClientRegisterCommandContent;
    use crate::SystemCommandHeader;
    use crate::SystemRegisterCommandContent;
    use crate::OperationReturn;
    use crate::StatusCode;
    use crate::ReadReturn;
    use log;

    pub struct TestClientOkAtomicRegister {}
    pub struct TestClientAuthErrAtomicRegister {}
    pub struct TestClientInvalidSectorErrAtomicRegister {}
    
    #[async_trait::async_trait]
    impl AtomicRegister for TestClientOkAtomicRegister {
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<dyn FnOnce(OperationComplete) + Send + Sync>) {
                let ret = if let ClientRegisterCommandContent::Write{data: _} = cmd.content {
                    log::info!("detected Write...");
                    OperationReturn::Write{}
                } else {
                    log::info!("detected Read...");
                    let dummy_data = SectorVec(vec![0x61; 4096]);
                    OperationReturn::Read(ReadReturn{read_data: Some(dummy_data)})
                };

                log::info!("client_command from TestClientOkAtomicRegister: calling lambda...");
                let op_complete = OperationComplete{
                    op_return: ret,
                    request_identifier: cmd.header.request_identifier,
                    status_code: StatusCode::Ok,
                };
                operation_complete(op_complete);
        }
        
        async fn system_command(&mut self, _: SystemRegisterCommand) {}
    }

    impl BasicAtomicRegister {
        fn highest(&self) -> Rank {

            let mut max = (Ts(0), Rank(0));
            let mut max_id = Rank(0);
            for (key, (ts, wr, _)) in &self.state.readlist {
                if (ts.0, wr.0) > ((max.0).0, (max.1).0) {
                    max_id = *key;
                    max = (*ts, *wr)
                }
            }
            log::info!("[highest]: found max for {}", max_id.0);
            assert_ne!(0, max_id.0);
            max_id
        }
    }

    #[async_trait::async_trait]
    impl AtomicRegister for BasicAtomicRegister {
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<dyn FnOnce(OperationComplete) + Send + Sync>) {
                self.state.operation_complete = Some(operation_complete);
                // TODOO sprawdź czy reading || writing
                assert_eq!(self.state.writing, false);
                // TODOO po co te UUID?
                match cmd.content {
                    ClientRegisterCommandContent::Read => {
                        log::info!("[client_command] captured Client Read (from {})", self.self_id.0);
                        // rid := rid + 1;
                        // store(rid);
                        // readlist := [ _ ] `of length` N;
                        // acklist := [ _ ] `of length` N;
                        // reading := TRUE;
                        // trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                        self.state.rid += 1; // TOOD maybe += 'processes_count'?
                        safe_unwrap!(self.metadata.put("rid", &self.state.rid.0.to_ne_bytes()).await);
                        self.state.readlist.drain();
                        self.state.acklist.drain();
                        self.state.reading = true;
                        let msg_ident = uuid::Uuid::new_v4();
                        self.state.msg_owners.lock().await.insert(msg_ident, self.state.my_idx);
                        log::info!("[client_command][client-system] sending broadcast ReadProc (from {})", self.self_id.0);
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                 header: SystemCommandHeader{
                                     process_identifier: self.self_id.0,
                                     msg_ident,
                                     read_ident: self.state.rid.0,
                                     sector_idx: cmd.header.sector_idx, 
                                 },
                                 content: SystemRegisterCommandContent::ReadProc{}
                            })
                        }).await;
                    },
                    ClientRegisterCommandContent::Write{data} => {
                        log::info!("[{}][client_command] captured Client Write", self.self_id.0);
                        // rid := rid + 1;
                        // writeval := v;
                        // acklist := [ _ ] `of length` N;
                        // readlist := [ _ ] `of length` N;
                        // writing := TRUE;
                        // store(wr, ts, rid, writeval, writing);
                        // trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                        self.state.rid += 1;
                        self.state.writeval = data;
                        self.state.readlist.drain();
                        self.state.acklist.drain();
                        self.state.writing = true;
                        assert_eq!(self.state.reading, false);
                        vec![
                            self.metadata.put("wr",  &self.state.wr.0.to_ne_bytes()).await,
                            self.metadata.put("ts", &self.state.ts.0.to_ne_bytes()).await,
                            self.metadata.put("rid", &self.state.rid.0.to_ne_bytes()).await,
                            self.metadata.put("writeval", &self.state.writeval.0).await,
                            self.metadata.put("writing", &bincode::serialize(&self.state.writing).unwrap()).await,
                        ].into_iter().for_each(|x| {safe_unwrap!(x)});

                        log::info!("[{}][client_command][client-system] sending broadcast ReadProc", self.self_id.0);
                        let msg_ident = uuid::Uuid::new_v4();
                        self.state.msg_owners.lock().await.insert(msg_ident, self.state.my_idx);
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                 header: SystemCommandHeader{
                                    process_identifier: self.self_id.0,
                                    msg_ident,
                                    read_ident: self.state.rid.0,
                                    sector_idx: cmd.header.sector_idx, 
                                 },
                                 content: SystemRegisterCommandContent::ReadProc{}
                            })
                        }).await;
                    },
                }
            }

        async fn system_command(&mut self, cmd: SystemRegisterCommand) {
            let response_header =  SystemCommandHeader {
                process_identifier: self.self_id.0,
                ..cmd.header
            };

            // TODO tu jestem
            // * brak obsługi wielu sektorów - co ze zmienną val?
            // * store val, writeval
            match cmd.content {
                SystemRegisterCommandContent::ReadProc => {
                    log::info!("[{}][system_command] captured ReadProc from {}", self.self_id.0, cmd.header.process_identifier);
                    // trigger < pl, Send | p, [VALUE, r, ts, wr, val] >;
                    let val = self.sectors_manager.read_data(cmd.header.sector_idx).await;
                    self.register_client.send(crate::Send{
                        target: cmd.header.process_identifier as usize, // TODO u8 cast
                        cmd: Arc::new(SystemRegisterCommand {
                            header: response_header.clone(),
                            content: SystemRegisterCommandContent::Value {
                                timestamp: self.state.ts.0,
                                write_rank: self.state.wr.0,
                                sector_data: val,
                            },
                        })
                    }).await;
                },
                SystemRegisterCommandContent::Value{timestamp, write_rank, sector_data} => {
                    log::info!("[{}][system_command] captured Value from {}", self.self_id.0, cmd.header.process_identifier);
                    if cmd.header.read_ident != self.state.rid.0 {
                        log::info!("WAZNE: RID MISMATCH: {} against {}", cmd.header.read_ident, self.state.rid.0);
                        return ();
                    }
                    if !self.state.reading && !self.state.writing {
                        log::error!("TODO REMOVE ME: test error!: got Value when not performed any action");
                        return ();
                    }

                    let ts = Ts(timestamp);
                    let wr = Rank(write_rank);
                    self.state.readlist.insert(Rank(cmd.header.process_identifier), (ts, wr, sector_data));

                    if (self.state.readlist.len() <= self.processes_count / 2) {
                        return ();
                    }

                    let (maxts, rr, readval) = self.state.readlist.remove(&(self.highest())).unwrap();
                    self.state.readval = readval;
                    self.state.readlist.drain();
                    self.state.acklist.drain();

                    // upon event <sl, Deliver | q, [VALUE, r, ts', wr', v'] > such that r == rid do
                    // readlist[q] := (ts', wr', v');
                    // if #(readlist) > N / 2 and (reading or writing) then
                        // (maxts, rr, readval) := highest(readlist);
                        // readlist := [ _ ] `of length` N;
                        // acklist := [ _ ] `of length` N;
                        // if reading = TRUE then
                        //     trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts, rr, readval] >;
                        // else
                        //     trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts + 1, rank(self), writeval] >;


                    if self.state.reading {
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                    header: response_header.clone(),
                                    content: SystemRegisterCommandContent::WriteProc{
                                        timestamp: maxts.0,
                                        write_rank: rr.0,
                                        data_to_write: self.state.readval.clone(),
                                    }
                            })
                        }).await;
                    } else {
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                    header: response_header.clone(),
                                    content: SystemRegisterCommandContent::WriteProc{
                                        timestamp: maxts.0 + 1,
                                        write_rank: self.self_id.0,
                                        data_to_write: self.state.writeval.clone(),
                                    }
                            })
                        }).await;
                    }
                },
                SystemRegisterCommandContent::WriteProc{timestamp, write_rank, data_to_write} => {
                    log::info!("[{}][system_command] captured WriteProc from {}", self.self_id.0, cmd.header.process_identifier);
                    // upon event < sbeb, Deliver | p, [WRITE_PROC, r, ts', wr', v'] > do
                    // if (ts', wr') > (ts, wr) then
                    //     (ts, wr, val) := (ts', wr', v');
                    //     store(ts, wr, val);
                    // trigger < pl, Send | p, [ACK, r] >;

                    // TODOO handle val

                    if (timestamp, write_rank) > (self.state.ts.0, self.state.wr.0) {
                        self.state.ts = Ts(timestamp);
                        self.state.wr = Rank(write_rank);
                        self.sectors_manager.write(cmd.header.sector_idx, &(data_to_write, timestamp, write_rank)).await;

                        vec![
                            self.metadata.put("wr",  &self.state.wr.0.to_ne_bytes()).await,
                            self.metadata.put("ts", &self.state.ts.0.to_ne_bytes()).await,
                        ].into_iter().for_each(|x| {safe_unwrap!(x)});
                    }
                    self.register_client.send(crate::Send{
                        target: cmd.header.process_identifier as usize, // TODO u8 cast
                        cmd: Arc::new(SystemRegisterCommand {
                            header: response_header.clone(),
                            content: SystemRegisterCommandContent::Ack{},
                        })
                    }).await;
                },
                SystemRegisterCommandContent::Ack => {
                    // upon event < pl, Deliver | q, [ACK, r] > such that r == rid do
                    // acklist[q] := Ack;
                    // if #(acklist) > N / 2 and (reading or writing) then
                    //     acklist := [ _ ] `of length` N;
                    //     if reading = TRUE then
                    //         reading := FALSE;
                    //         trigger < nnar, ReadReturn | readval >;
                    //     else
                    //         writing := FALSE;
                    //         store(writing);
                    //         trigger < nnar, WriteReturn >;
                    log::info!("[{}][system_command] captured Ack from {}", self.self_id.0, cmd.header.process_identifier);
                    if cmd.header.read_ident != self.state.rid.0 {
                        return ();
                    }
                    self.state.acklist.insert(Rank(cmd.header.process_identifier));
                    if self.state.reading && self.state.writing {
                        log::error!("internal error: reading and writing simulataneously!");
                    }
                    if (self.state.acklist.len() <= self.processes_count / 2) || !(self.state.reading || self.state.writing) {
                        return ();
                    }
                    self.state.acklist.drain();

                    let op_return : crate::domain::OperationReturn;
                    if self.state.reading {
                        self.state.reading = false;
                        op_return = OperationReturn::Read(ReadReturn{read_data: Some(self.state.readval.clone())});
                    } else {
                        // writing
                        self.state.writing = false;
                        op_return = OperationReturn::Write;
                    }

                    // that's bad I know
                    let op_complete = None;
                    let op_complete = std::mem::replace(&mut self.state.operation_complete, op_complete);

                    match op_complete {
                        None => {}, // it seems that I died and recovered 
                        Some(op_complete) => {
                            op_complete(OperationComplete{
                                status_code: StatusCode::Ok,
                                request_identifier: self.state.rid.0,
                                op_return,
                            });
                        },
                    }
                },
            }
        }
    }
}

