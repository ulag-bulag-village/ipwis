use ipiis_api::common::Ipiis;
use ipis::{core::anyhow::Result, env::Infer, tokio};
use ipsis_api::{common::Ipsis, server::IpsisServer};
use ipwis_api::{
    client::IpwisClient,
    common::{Ipwis, Task},
};

#[tokio::main]
async fn main() -> Result<()> {
    // create and deploy a sample IPSIS server
    let ipsis = IpsisServer::infer().await;
    tokio::task::spawn(ipsis.run());

    // create an IPWIS account
    let ipwis = IpwisClient::infer().await;
    let ipiis = &ipwis.ipiis;

    ipiis
        .set_account_primary(::ipsis_api::common::KIND.as_ref(), ipiis.account_ref())
        .await?;
    ipiis
        .set_address(
            ::ipsis_api::common::KIND.as_ref(),
            ipiis.account_ref(),
            &"127.0.0.1:5001".parse()?,
        )
        .await?;

    // prepare a program
    // TODO: automatically build and attach
    let my_program =
        include_bytes!("../../target/wasm32-wasi/release/ipwis_modules_task_demo.wasi.wasm");

    // upload the program
    let my_program = ipiis.put(&my_program.to_vec()).await?;
    let my_program = ipiis.sign_owned(*ipiis.account_ref(), my_program)?;
    let my_program = ipiis.sign_as_guarantor(my_program)?;

    // create a task and sign
    let mut task = Task::new_sandbox();
    task.program = Some(my_program);
    let task = ipiis.sign_owned(*ipiis.account_ref(), task)?;

    // spawn a task
    let id = ipwis.task_spawn(task).await?;
    let id = ipiis.sign_as_guarantor(id)?;

    // wait the task
    let outputs = ipwis.task_wait(id).await?;
    println!("{:?}", outputs);

    Ok(())
}
