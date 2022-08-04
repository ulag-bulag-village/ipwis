use std::sync::Arc;

use ipis::{
    async_trait::async_trait,
    core::{account::GuarantorSigned, anyhow::Result, data::Data},
    object::data::ObjectData,
};
use ipwis_modules_task_common::task::Task;

use crate::task_instance::TaskInstance;

#[async_trait]
pub trait TaskManager {
    type ExternData: Clone + ::core::fmt::Debug + Send + Sync;
    type Program: ?Sized;

    async fn spawn_raw(
        self: &Arc<Self>,
        task: Data<GuarantorSigned, Task>,
        program: &<Self as TaskManager>::Program,
    ) -> Result<TaskInstance<Box<ObjectData>, Self>>
    where
        Self: Sized;
}
