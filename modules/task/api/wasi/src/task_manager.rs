use std::sync::Arc;

use ipis::{
    async_trait::async_trait,
    core::{
        account::GuarantorSigned,
        anyhow::{bail, Result},
        value::{chrono::DateTime, text::Text},
    },
    object::data::ObjectData,
    pin::PinnedInner,
    resource::Resource,
    tokio::{self, sync::Mutex},
};
use ipwis_modules_task_api::{
    task_instance::TaskInstance, task_manager::TaskManager, task_state::TaskState,
};
use ipwis_modules_task_common::task::Task;
use ipwis_modules_task_common_wasi::extern_data::{ExternData, ExternDataRef};
use wasmtime::{Config, Engine, Linker, Module, Store, Trap};

use crate::{
    interrupt_manager::InterruptManager,
    intrinsics::syscall,
    memory::{IpwisMemoryInner, Memory},
    task_ctx::IpwisTaskCtx,
};

pub struct IpwisTaskManager {
    linker: Linker<IpwisTaskCtx>,
    pub interrupt_manager: Arc<InterruptManager>,
}

#[async_trait]
impl TaskManager for IpwisTaskManager {
    type ExternData = ExternData;
    type Program = [u8];

    async fn spawn_raw(
        self: &Arc<Self>,
        task: GuarantorSigned<Task>,
        program: &<Self as TaskManager>::Program,
    ) -> Result<TaskInstance<Result<Box<ObjectData>, Text>, Self>> {
        // create a new state
        let state = Arc::new(Mutex::new(TaskState {
            manager: self.clone(),
            task,
            created_date: DateTime::now(),
            is_working: true,
        }));

        // create a new store
        let mut store = Store::new(
            self.linker.engine(),
            IpwisTaskCtx::try_new(self.clone(), state.clone())?,
        );

        // create an instance with given module and store
        let module = Module::from_binary(self.linker.engine(), program)?;
        let instance = self.linker.instantiate_async(&mut store, &module).await?;

        // find main function
        let func = syscall::instance::__syscall(&instance, &mut store)?;

        // external call
        // note: the inner schedule is controlled by `wasmtime` engine, not by this scheduler
        let handler = {
            let state = state.clone();
            let (inputs, outputs, errors) = {
                let mut memory = IpwisMemoryInner::with_instance(&instance, &mut store)?;
                let state = state.lock().await;

                let inputs = memory
                    .dump_doubled_object(&state.task.constraints.inputs)
                    .await?;
                let outputs = memory.dump_doubled_null().await?;
                let errors = memory.dump_doubled_null().await?;
                (inputs, outputs, errors)
            };

            tokio::spawn(async move {
                let result = func
                    .call_async(
                        &mut store,
                        (0 /* nullptr */, inputs.ptr, outputs.ptr, errors.ptr),
                    )
                    .await;

                let memory = IpwisMemoryInner::with_instance(&instance, &mut store);
                let mut state = state.lock().await;
                state.is_working = false;

                fn parse_status_code<T>(
                    memory: Result<IpwisMemoryInner<&'_ mut Store<T>>>,
                    outputs: ExternData,
                    errors: ExternData,
                    result: Result<ExternDataRef, Trap>,
                ) -> Result<Box<ObjectData>>
                where
                    T: Resource + Send + Sync,
                {
                    let memory = memory?;

                    match result {
                        Ok(syscall::SYSCALL_OK) => {
                            // parse outputs as ObjectData
                            let outputs = memory.load_doubled(outputs.ptr)?;
                            PinnedInner::deserialize_owned(outputs).map(Box::new)
                        }
                        Ok(syscall::SYSCALL_ERR_NORMAL) => {
                            // parse errors as String
                            let errors = memory.load_doubled(errors.ptr)?;
                            bail!("{}", ::core::str::from_utf8(errors)?)
                        }
                        Ok(syscall::SYSCALL_ERR_FATAL) => bail!("fatal error"),
                        Ok(_) => bail!("unknown status code"),
                        Err(e) => bail!("trap: {e}"),
                    }
                }

                parse_status_code(memory, outputs, errors, result).map_err(Text::with_en_us)
            })
        };

        Ok(TaskInstance { state, handler })
    }
}

impl IpwisTaskManager {
    pub async fn try_new() -> Result<Self> {
        // define the WASI functions globally on the `Config`.
        let engine = Engine::new(Config::new().async_support(true))?;

        // create a linker
        let mut linker = Linker::new(&engine);
        ::wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut IpwisTaskCtx| &mut ctx.wasi)?;

        // register intrinsics
        {
            crate::intrinsics::syscall::linker::__syscall(&mut linker)?;
        }

        // create an interrupt maanger
        let interrupt_manager = Default::default();

        Ok(Self {
            linker,
            interrupt_manager,
        })
    }
}
