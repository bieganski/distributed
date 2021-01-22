use assignment_2_solution::{
    deserialize_register_command, serialize_register_command, SystemCommandHeader,
    SystemRegisterCommand, SystemRegisterCommandContent, SectorVec, RegisterCommand, MAGIC_NUMBER,
};
use ntest::timeout;
use uuid::Uuid;
use std::io::{BufReader, Cursor};


// TODO tu jestem
// name clash with deserialize_register_command I suppose...



// #[test]
// #[timeout(200)]
fn serialize_deserialize_is_identity() {
    
    let sector_data = SectorVec(vec![0x61; 4096]);
    
    let value = SystemRegisterCommandContent::Value{timestamp: 0xdeadbeef, write_rank: 0xf, sector_data: sector_data.clone()};
    let write = SystemRegisterCommandContent::WriteProc{timestamp: 0xdeadbeef, write_rank: 0xf, data_to_write: sector_data.clone()};
    let read  = SystemRegisterCommandContent::ReadProc{};
    let ack   = SystemRegisterCommandContent::Ack{};

    let system_cmd_value = SystemRegisterCommand {
        header: SystemCommandHeader {
            process_identifier: 7,
            sector_idx: 8,
            msg_ident: Uuid::new_v4(),
            read_ident: 0xdeadbeef + 2,
        },
        content: value,
    };

    let system_cmd_write = SystemRegisterCommand{content: write, ..system_cmd_value};
    let system_cmd_read  = SystemRegisterCommand{content: read,  ..system_cmd_value};
    let system_cmd_ack   = SystemRegisterCommand{content: ack,   ..system_cmd_value};

    let mut sink_ser: Vec<u8> = Vec::new();
    let mut sink_deser: Vec<u8> = Vec::new();

    serialize_register_command(&RegisterCommand::System(system_cmd_value), &mut sink_ser).expect("Could not serialize");
    let deserialized_cmd = deserialize_register_command(&mut BufReader::new(Cursor::new(sink_deser))).expect("Could not deserialize");


    // let mut slice: &[u8] = &sink[..];
    // let data_read: &mut dyn std::io::Read = &mut slice;
    // let deserialized_cmd = deserialize_register_command(data_read).expect("Could not deserialize");

    // // then
    // match deserialized_cmd {
    //     RegisterCommand::Client(ClientRegisterCommand {
    //         header,
    //         content: ClientRegisterCommandContent::Read,
    //     }) => {
    //         assert_eq!(header.sector_idx, sector_idx);
    //         assert_eq!(header.request_identifier, request_identifier);
    //     }
    //     _ => panic!("Expected Read command"),
    // }
}



#[test]
#[timeout(200)]
fn deserialize_ack_cmd_has_correct_format() {
    // given
    let process_identifier_orig = 17_u8;
    let msg_ident_orig = Uuid::new_v4();
    let sector_idx_orig = 6224645_u64;
    let read_ident_orig = 170_170_170_170_u64;

    let mut serialized = Vec::new();
    serialized.extend_from_slice(MAGIC_NUMBER.as_ref());
    serialized.extend_from_slice(&[0, 0, process_identifier_orig, 6]);
    serialized.extend_from_slice(msg_ident_orig.as_bytes());
    serialized.extend_from_slice(&read_ident_orig.to_be_bytes());
    serialized.extend_from_slice(&sector_idx_orig.to_be_bytes());

    // when
    let mut slice: &[u8] = &serialized[..];
    let data_read: &mut dyn std::io::Read = &mut slice;
    let ack_cmd = deserialize_register_command(data_read);

    // then

    match ack_cmd {
        Ok(RegisterCommand::System(SystemRegisterCommand {
            header:
                SystemCommandHeader {
                    process_identifier,
                    msg_ident,
                    sector_idx,
                    read_ident,
                },
            content: SystemRegisterCommandContent::Ack,
        })) => {
            assert_eq!(process_identifier_orig, process_identifier);
            assert_eq!(msg_ident_orig, msg_ident);
            assert_eq!(sector_idx_orig, sector_idx);
            assert_eq!(read_ident_orig, read_ident);
        }
        _ => panic!("Expected Ack command"),
    }
}