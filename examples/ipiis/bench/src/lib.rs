use std::{net::SocketAddr, sync::Arc, time::Instant};

use byte_unit::Byte;
use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipiis_modules_bench_common::{IpiisBench, KIND};
use ipis::{
    core::account::Account,
    env::{self, Infer},
    futures,
    stream::DynStream,
};
use rand::{distributions::Uniform, Rng};

#[ipwis_modules_task_entrypoint::entrypoint]
pub async fn main(inputs: ObjectData) -> Result<ObjectData> {
    // Account of the target server
    let account: Account = env::infer("IPIIS_BENCH_ACCOUNT").unwrap_or_else(|_| {
        "3KvPsVRhN49CHj5ReVvmq1pozQMeTqsm9KmQrR9EsM2wBEn67aiXqA8Rr6YkTGJWHdvKxtDzMb4jkc7gD42CSM2p"
            .parse()
            .unwrap()
    });
    // Address of the target server
    let address: SocketAddr = env::infer::<_, SocketAddr>("IPIIS_BENCH_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:9999".parse().unwrap());
    // Size of benchmarking stream
    let size: u128 = env::infer("IPIIS_BENCH_SIZE").unwrap_or(1_000_000_000);
    // Number of threads
    let num_threads: u32 = env::infer("IPIIS_BENCH_NUM_THREADS").unwrap_or(4);

    // create a client
    let client = IpiisClient::genesis(None).await?;

    // registre the server account as primary
    client
        .set_account_primary(KIND.as_ref(), &account.account_ref())
        .await?;
    client
        .set_address(KIND.as_ref(), &account.account_ref(), &address)
        .await?;

    let size = Byte::from_bytes(size).get_appropriate_unit(false);

    // print the configuration
    println!("- Account: {}", account.to_string());
    println!("- Address: {address}");
    println!("- Data Size: {size}");
    println!("- Number of Threads: {num_threads}");

    // init data
    println!("- Initializing...");
    let range = Uniform::from(0..=255);
    let data: Arc<Vec<u8>> = Arc::new(
        ::rand::thread_rng()
            .sample_iter(&range)
            .take(size.get_byte().get_bytes().try_into()?)
            .collect(),
    );

    // begin benchmaring
    println!("- Benchmarking...");
    let instant = Instant::now();
    {
        futures::future::try_join_all(
            (0..num_threads).map(|_| client.ping(DynStream::ArcVec(data.clone()))),
        )
        .await?;
    }
    let duration = instant.elapsed();

    // print the output
    println!("- Finished!");
    println!("- Elapsed Time: {duration:?}");
    println!("- Elapsed Speed: {}bps", {
        let mut speed = Byte::from_bytes(
            ((8 * num_threads as u128 * size.get_byte().get_bytes()) as f64
                / duration.as_secs_f64()) as u128,
        )
        .get_appropriate_unit(false)
        .to_string();
        speed.pop();
        speed
    });

    let outputs = inputs;
    Ok(outputs)
}
