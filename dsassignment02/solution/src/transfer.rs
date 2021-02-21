
/// Your internal representation of RegisterCommand for ser/de can be anything you want,
/// we just would like some hooks into your solution to asses where the problem is, should
/// there be a problem.
pub mod transfer_public {
    use crate::domain::SectorVec;
    use crate::RegisterCommand;
    use crate::ClientRegisterCommand;
    use crate::ClientCommandHeader;
    use crate::ClientRegisterCommandContent;
    use std::convert::TryInto;
    use std::io::{Error, Read, Write, BufWriter, Cursor};
    pub use crate::transfer_system;

    const MAGIC: &[u8; 4] = &[0x61, 0x74, 0x64, 0x64];
    const MSG_OFFSET : usize = 7;
    
    const REQ_NUM_OFFSET : usize = 8;
    const SECTOR_IDX_OFFSET : usize = 16;
    const HDR_SIZE : usize = 24;
    const BLOCK_SIZE : usize = 4096;


    #[derive(Debug, PartialEq)]
    pub enum Direction {
        Request,
        ReadResponse(Option<SectorVec>, u8), // u8 stands for Status Code
        WriteResponse(u8), // u8 stands for Status Code
    }


    pub fn deserialize_register_command(data: &mut dyn Read) -> Result<RegisterCommand, Error> {
        deserialize_register_command_generic(data, Direction::Request{})
    }
    
    // to not breach the API..
    pub fn deserialize_register_command_send(data: &mut (dyn Read + Send)) -> Result<RegisterCommand, Error> {
        deserialize_register_command_generic(data, Direction::Request{})
    }

    fn deserialize_register_command_generic(
        data: &mut dyn Read, 
        direction : Direction) -> Result<RegisterCommand, Error> {
        let mut read_buf = vec![];
        let num = data.read_to_end(&mut read_buf).unwrap();

        if &read_buf[0..MAGIC.len()] != MAGIC {
            return safe_err_return!(format!("wrong Magic number: expected {:?}, got {:?}", MAGIC, &read_buf[0..MAGIC.len()]));
        }

        // split by message type
        if &read_buf[7] >= &0x3 {
            return transfer_system::deserialize_system_command(&mut read_buf.as_slice());
        }

        // response not supported for now
        assert_eq!(direction, Direction::Request{});

        if direction == Direction::Request {
            let content = if (read_buf[MSG_OFFSET] & 0x1) != 0 {
                if num != HDR_SIZE {
                    return safe_err_return!(format!("mismatched message size, expected {}, got {}", HDR_SIZE, num))
                }
                ClientRegisterCommandContent::Read{}
            } else {
                if num != HDR_SIZE + BLOCK_SIZE {
                    return safe_err_return!(format!("mismatched message size, expected {}, got {}", HDR_SIZE + BLOCK_SIZE, num))
                }
                ClientRegisterCommandContent::Write{data: SectorVec((
                    &read_buf[HDR_SIZE..]).to_vec()
                )}
            };

            let header = ClientCommandHeader {
                request_identifier: u64::from_be_bytes(read_buf[REQ_NUM_OFFSET..REQ_NUM_OFFSET+8].try_into().expect("internal error")),
                sector_idx: u64::from_be_bytes(read_buf[SECTOR_IDX_OFFSET..SECTOR_IDX_OFFSET+8].try_into().expect("internal error")) as crate::domain::SectorIdx,
            };

            Ok(RegisterCommand::Client(ClientRegisterCommand{
                header,
                content,
            }))
        } else {
            unimplemented!()
        }
    }


    pub fn serialize_register_command_generic(
        cmd: &RegisterCommand,
        writer: &mut dyn Write,
        direction: Direction,
    ) -> Result<(), Error> {
        let stream = writer;  // replace it with File or Cursor::new(Vec::<u8>::new()) for testing purposes
        let mut buf_writer = BufWriter::new(stream);
        match cmd {
            RegisterCommand::Client(ClientRegisterCommand{header, content}) => {
                let ClientCommandHeader{request_identifier, sector_idx} = header;
                let mut separable_part = BufWriter::new(Cursor::new(Vec::<u8>::new()));

                let msg_type : u8;

                // if it's request, then its simply third byte of padding
                let status_code = match direction {
                    Direction::Request{} => {0x0},
                    Direction::ReadResponse(_, code) => {code},
                    Direction::WriteResponse(code) => {code},
                };

                match content {
                    ClientRegisterCommandContent::Read => {
                        msg_type = if let Direction::Request = direction {0x1} else {0x41};

                        if let Direction::Request = direction {
                            safe_unwrap!(separable_part.write_all(&sector_idx.to_be_bytes()));
                        }
                        if let Direction::ReadResponse(data, _) = direction {
                            if let None = data {
                                ()
                            }
                            let SectorVec(data) = data.unwrap();
                            if data.len() != 4096 {
                                log::error!("malformed SectorVec! data length should be 4096, not {}", data.len());
                            }
                            safe_unwrap!(separable_part.write_all(&data));
                        }
                    },
                    ClientRegisterCommandContent::Write{data} => {
                        msg_type = if let Direction::Request = direction {0x2} else {0x42};

                        if let Direction::Request = direction {
                            safe_unwrap!(separable_part.write_all(&sector_idx.to_be_bytes()));
                            let SectorVec(data) = data;
                            if data.len() != 4096 {
                                log::error!("malformed SectorVec! data length should be 4096, not {}", data.len());
                            }
                            safe_unwrap!(separable_part.write_all(&data));
                        }
                    }
                }
                // common part
                vec![
                    buf_writer.write_all(MAGIC),
                    buf_writer.write_all(&[0x0, 0x0]), // padding
                    buf_writer.write_all(&[status_code]),
                    buf_writer.write_all(&msg_type.to_be_bytes()),
                    buf_writer.write_all(&request_identifier.to_be_bytes()),
                ].into_iter().for_each(|x| {safe_unwrap!(x)});

                // part that depends on msg. type
                safe_unwrap!(buf_writer.write_all(separable_part.buffer()));
            },
            RegisterCommand::System(cmd) => {
                transfer_system::serialize_system_command(&cmd, &mut buf_writer)
            },
        }
        Ok(())
    }

    pub fn serialize_register_command(
        cmd: &RegisterCommand,
        writer: &mut dyn Write,
    ) -> Result<(), Error> {
        serialize_register_command_generic(cmd, writer, Direction::Request{})
    }
}


pub mod transfer_system {
    use crate:: SystemRegisterCommand;
    use crate::SystemCommandHeader;
    use crate::RegisterCommand;
    use crate::SectorVec;
    use crate::SystemRegisterCommandContent;
    use std::io::{Read, Write, Cursor, Error, BufWriter};
    use uuid::Uuid;
    use std::convert::TryInto;

    const MAGIC: &[u8; 4] = &[0x61, 0x74, 0x64, 0x64];

    #[derive(Debug, PartialEq)]
    pub enum Direction {
        Request,
        Response(u8), // u8 stands for Status Code
    }

    fn serialize_msg_type(system_msg_type: &SystemRegisterCommandContent) -> u8 {
        match system_msg_type {
            SystemRegisterCommandContent::ReadProc{} => {0x3},
            SystemRegisterCommandContent::Value{..} => {0x4},
            SystemRegisterCommandContent::WriteProc{..} => {0x5},
            SystemRegisterCommandContent::Ack{} => {0x6},
        }
    }

    pub fn serialize_system_command(cmd: &SystemRegisterCommand, writer: &mut dyn Write) {
        serialize_system_command_generic(cmd, writer, &Direction::Request);
    }

    pub fn serialize_system_response(cmd: &SystemRegisterCommand, writer: &mut dyn Write, status_code: u8) {
        serialize_system_command_generic(cmd, writer, &Direction::Response(status_code));
    }

    pub fn serialize_system_command_generic(cmd: &SystemRegisterCommand, writer: &mut dyn Write, dir: &Direction) {

        if let Direction::Response(_) = dir {
            unimplemented!()
        }
        
        let mut common = BufWriter::new(writer);
        let mut content = BufWriter::new(Cursor::new(Vec::<u8>::new()));
        
        match &cmd.content {
            SystemRegisterCommandContent::ReadProc{} => {
                // no content
            },
            SystemRegisterCommandContent::WriteProc{timestamp, write_rank, data_to_write} => {
                let SectorVec(sector) = data_to_write;
                vec![
                    content.write_all(&timestamp.to_be_bytes()),
                    content.write_all(&[0x0; 7]), // padding
                    content.write_all(&write_rank.to_be_bytes()),
                    content.write_all(&sector),
                ].into_iter().for_each(|x| {safe_unwrap!(x)});
            },
            SystemRegisterCommandContent::Value{timestamp, write_rank, sector_data} => {
                let SectorVec(sector) = sector_data;
                vec![
                    content.write_all(&timestamp.to_be_bytes()),
                    content.write_all(&[0x0; 7]), // padding
                    content.write_all(&write_rank.to_be_bytes()),
                    content.write_all(&sector),
                ].into_iter().for_each(|x| {safe_unwrap!(x)});
            },
            SystemRegisterCommandContent::Ack{} => {
                // no content
            },
        }

        // common part
        vec![
            common.write_all(MAGIC),
            common.write_all(&[0x0, 0x0]), // padding
            common.write_all(&cmd.header.process_identifier.to_be_bytes()),
            common.write_all(&serialize_msg_type(&cmd.content).to_be_bytes()),
            common.write_all(cmd.header.msg_ident.as_bytes()),
            common.write_all(&cmd.header.read_ident.to_be_bytes()),
            common.write_all(&cmd.header.sector_idx.to_be_bytes()),
            common.write_all(&content.buffer()),
        ].into_iter().for_each(|x| {safe_unwrap!(x)});
    }

    pub fn deserialize_system_command(reader: &mut dyn Read) -> Result<RegisterCommand, Error> {
        deserialize_system_command_generic(reader, Direction::Request{})
    }

    fn deserialize_system_command_generic(reader: &mut dyn Read, direction : Direction) -> Result<RegisterCommand, Error> {
        let mut read_buf = vec![0_u8; 40];
        let mut content_buf : Vec<u8> = vec![];

        if let Direction::Response(_) = direction {
            unimplemented!()
        }

        reader.read_exact(&mut read_buf).unwrap();

        let num = reader.read_to_end(&mut content_buf).unwrap();

        let uuid : [u8; 16] = read_buf[8..24].try_into().expect("[msg_ident] try_into error");
        
        let header = SystemCommandHeader{
            process_identifier: read_buf[6],
            msg_ident: Uuid::from_bytes(uuid),
            read_ident: u64::from_be_bytes(read_buf[24..32].try_into().expect("[read_ident] try_into error")),
            sector_idx: u64::from_be_bytes(read_buf[32..40].try_into().expect("[sector_idx] try_into error")),
        };

        let content = match read_buf[7] {
            0x3 => {SystemRegisterCommandContent::ReadProc{}},
            0x4 => {SystemRegisterCommandContent::Value{
                timestamp: u64::from_be_bytes(content_buf[0..8].try_into().expect("[timestamp] try_into error")),
                write_rank: content_buf[15],
                sector_data: SectorVec((
                    &content_buf[16..]).to_vec()
                )}
            },
            0x5 => {SystemRegisterCommandContent::WriteProc{
                timestamp: u64::from_be_bytes(content_buf[0..8].try_into().expect("[timestamp] try_into error")),
                write_rank: content_buf[15],
                data_to_write: SectorVec((
                    &content_buf[16..]).to_vec()
                )}
            },
            0x6 => {SystemRegisterCommandContent::Ack{}},
            
            _ => {panic!("MALFORMED MSG TYPE - HANDLE ME")},
        };

        Ok(RegisterCommand::System(SystemRegisterCommand{header, content}))
    }
}
