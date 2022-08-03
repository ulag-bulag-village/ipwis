#![cfg(target_os = "wasi")]

use bytecheck::CheckBytes;
use ipis::{
    class::Class,
    core::{anyhow::bail, signed::IsSigned},
    env::Infer,
};
use ipsis_common::Ipsis;
use ipwis_modules_ipiis_common::IpiisClient;
use rkyv::{Archive, Deserialize, Serialize};

#[ipwis_modules_task_entrypoint::entrypoint]
pub async fn main(inputs: ObjectData) -> Result<ObjectData> {
    println!("{:?}", inputs);

    // no-stream module (small)
    let instant = ::std::time::Instant::now();
    {
        let mut src = "hello world!".as_bytes();

        let mut dst = Vec::new();
        ::ipis::tokio::io::copy(&mut src, &mut dst).await?;

        println!("{}", String::from_utf8(dst)?);
    }
    println!("stream module (small) = {:?}", instant.elapsed());

    // no-stream module (large)
    let instant = ::std::time::Instant::now();
    {
        let src = vec![42u8; 1_000_000_000];

        let mut dst = Vec::new();
        ::ipis::tokio::io::copy(&mut src.as_slice(), &mut dst).await?;

        assert_eq!(src.len(), dst.len());
    }
    println!("stream module (large) = {:?}", instant.elapsed());

    // stream module (small)
    let instant = ::std::time::Instant::now();
    {
        let src = "hello world!".as_bytes();
        let mut reader = ExternReader::try_from(src)?;

        let mut dst = Vec::new();
        ::ipis::tokio::io::copy(&mut reader, &mut dst).await?;

        println!("{}", String::from_utf8(dst)?);
    }
    println!("stream module (small) = {:?}", instant.elapsed());

    // stream module (large)
    let instant = ::std::time::Instant::now();
    {
        let src = vec![42u8; 1_000_000_000];
        let mut reader = ExternReader::try_from(src.as_slice())?;

        let mut dst = Vec::new();
        ::ipis::tokio::io::copy(&mut reader, &mut dst).await?;

        assert_eq!(src.len(), dst.len());
    }
    println!("stream module (large) = {:?}", instant.elapsed());

    // IPIIS module (ipsis: large)
    let instant = ::std::time::Instant::now();
    {
        #[derive(Class, Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(CheckBytes, Debug, PartialEq))]
        pub struct MyData {
            name: String,
            age: u32,
        }

        impl IsSigned for MyData {}
        // create a client
        let client = IpiisClient::try_infer().await?;

        // let's make a data we want to store
        let mut data = MyData {
            name: "Alice".to_string(),
            age: 42,
        };

        // CREATE
        let path_create = client.put(&data).await?;
        assert!(client.contains(&path_create).await?);

        // UPDATE (identity)
        let path_update_identity = client.put(&data).await?;
        assert_eq!(&path_create, &path_update_identity); // SAME Path

        // let's modify the data so that it has a different path
        data.name = "Bob".to_string();

        // UPDATE (changed)
        let path_update_changed = client.put(&data).await?;
        assert_ne!(&path_create, &path_update_changed); // CHANGED Path

        // READ
        let data_from_storage: MyData = client.get(&path_update_changed).await?;
        assert_eq!(&data, &data_from_storage);

        // DELETE
        client.delete(&path_update_identity).await?;
        client.delete(&path_update_changed).await?;

        // data is not exist after DELETE
        match client.get::<MyData>(&path_update_changed).await {
            Ok(_) => bail!("data not deleted!"),
            Err(_) => {
                assert!(!client.contains(&path_create).await?);
                assert!(!client.contains(&path_update_changed).await?);
            }
        }
    }
    println!("IPIIS module (ipsis: large) = {:?}", instant.elapsed());

    Ok(inputs)
}
