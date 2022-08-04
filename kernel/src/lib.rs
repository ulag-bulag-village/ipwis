use std::sync::Arc;

use ipis::{
    core::{account::GuarantorSigned, anyhow::Result, data::Data, value::text::Text},
    object::data::ObjectData,
    tokio::sync::Mutex,
};
use ipwis_modules_core_common::resource_store::{ResourceId, ResourceStore};
use ipwis_modules_task_api::{task_instance::TaskInstance, task_manager::TaskManager};
use ipwis_modules_task_api_wasi::task_manager::IpwisTaskManager;
use ipwis_modules_task_common::{task::Task, task_poll::TaskPoll};

type IpwisTaskInstance = TaskInstance<Box<ObjectData>, IpwisTaskManager>;

pub struct Kernel {
    manager: Arc<IpwisTaskManager>,
    instances: Arc<Mutex<ResourceStore<IpwisTaskInstance>>>,
}

impl Kernel {
    pub async fn try_new() -> Result<Self> {
        // prepare a task manager
        let manager = Arc::new(IpwisTaskManager::try_new().await?);

        // register some interrupt modules
        macro_rules! load_builtin_modules {
            ( $manager:expr => { $( $ty:ty, )* }, ) => {{$(
                $manager.interrupt_manager.put(<$ty>::default()).await?;
            )*}};
        }
        load_builtin_modules!(
            manager => {
                ::ipwis_modules_ipiis_api::IpiisModule,
                ::ipwis_modules_stream_api::StreamModule,
            },
        );

        Ok(Self {
            manager,
            instances: Default::default(),
        })
    }

    pub async fn spawn_raw(
        &self,
        task: Data<GuarantorSigned, Task>,
        program: &<IpwisTaskManager as TaskManager>::Program,
    ) -> Result<ResourceId> {
        // spawn a task
        let instance = self.manager.spawn_raw(task, program).await?;

        // register as a resource
        Ok(self.instances.lock().await.put(instance))
    }

    pub async fn poll(&self, id: &ResourceId) -> Result<TaskPoll> {
        let instances = self.instances.lock().await;
        if instances.get(id)?.handler.is_finished() {
            drop(instances);

            match self.wait(id).await {
                Ok(outputs) => Ok(TaskPoll::Ready(outputs)),
                Err(errors) => Ok(TaskPoll::Trap(Text::with_en_us(errors))),
            }
        } else {
            Ok(TaskPoll::Pending)
        }
    }

    pub async fn wait(&self, id: &ResourceId) -> Result<Box<ObjectData>> {
        self.instances.lock().await.remove(id)?.await
    }
}
