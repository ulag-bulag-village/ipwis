#[ipwis_modules_task_entrypoint::entrypoint]
pub async fn main(inputs: ObjectData) -> ::ipis::core::anyhow::Result<ObjectData> {
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

    Ok(inputs)
}
