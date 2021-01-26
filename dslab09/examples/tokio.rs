use async_channel::unbounded;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::time::Duration;

fn tokio_simple() {
    let (tx, rx) = unbounded();
    let single_thread_runtime = Builder::new_current_thread().build().unwrap();

    // Moving values into async task.
    single_thread_runtime.block_on(async move {
        tx.send(7).await.unwrap();
        println!("Received: {}", rx.recv().await.unwrap());
    });

    let multi_threaded_runtime = Runtime::new().unwrap();
    multi_threaded_runtime.block_on(async {
        println!(
            "Multi threaded for free. Just spawn hundreds of tasks and watch it \
            utilize CPU with small overhead."
        )
    });
}

/// tokio::main is a macro which takes care of creating a runtime, multi-threaded one by default.
/// Spawning tasks is a very important feature.
#[tokio::main]
async fn tokio_spawning_tasks() {
    let task = tokio::spawn(async {
        // Delay
        tokio::time::sleep(Duration::from_millis(200)).await;
    });

    assert!(matches!(task.await, Ok(_)));
}

#[tokio::main]
async fn tokio_filesystem() {
    let data = [0, 1, 1, 2, 3, 5, 8, 13];
    // Standard fs API blocks. We use it in async task context only for portable demonstration.
    // In general it is a bad idea to use blocking calls in async code, as this will block
    // executor thread for a long time and concurrency will suffer.
    // We use it to obtain a temporary file in async task context only for portable demonstration.
    let mut tmp_file = tokio::fs::File::from_std(tempfile::tempfile().unwrap());
    // Import AsyncWriteExt trait to use write.
    tmp_file.write_all(&data).await.unwrap();
    tmp_file.sync_data().await.unwrap();
    let mut buf = vec![];
    // Writing moved file offset, so we bring that to beginning.
    tmp_file.seek(SeekFrom::Start(0)).await.unwrap();
    // Import AsyncReadExt trait to use read.
    tmp_file.read_to_end(&mut buf).await.unwrap();
    assert_eq!(buf.as_slice(), &data);
}

#[tokio::main]
/// Basic synchronization.
async fn tokio_sync() {
    // Std Mutex is fine if it is expected to block briefly or not at all.
    let shared_data = std::sync::Mutex::new(7);
    {
        *shared_data.lock().unwrap().deref_mut() = 10;
    }
    // tokio Mutex is more expensive, but can be hold over await blocks.
    let tokio_mutex = tokio::sync::Mutex::new(7);
    {
        *tokio_mutex.lock().await = 10;
    }
    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    {
        // After permit is acquired, exclusive access is granted.
        let _permit = semaphore.acquire_owned().await;
        // Send cmd to register...
        // drop returns permit
    }
}

/// TCP example. UDP is similar.
#[tokio::main]
async fn tokio_network() {
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let task = tokio::spawn(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(b"Hello world network!").await.unwrap();
    });

    let (mut s, _) = listener.accept().await.unwrap();
    let mut data = vec![];
    s.read_to_end(&mut data).await.unwrap();
    println!("{}", String::from_utf8(data).unwrap());
    assert!(matches!(task.await, Ok(_)));
}



#[tokio::main]
async fn main() {
    //tokio_simple();
    //tokio_spawning_tasks();
    //tokio_filesystem();
    //tokio_sync();
    //tokio_network();
    //
    tokio::spawn(async{
        loop {}
    });
/*
    tokio::spawn(async{
        loop {}
    });
    tokio::spawn(async{
        loop {}
    });
    tokio::spawn(async{
        loop {}
    });
    tokio::spawn(async{
        loop {}
    });
    tokio::spawn(async{
        loop {}
    });
    tokio::spawn(async{
        loop {}
    });

    */
    }
