use std::sync::Arc;

use ipis::core::{account::GuarantorSigned, data::Data, value::chrono::DateTime};
use ipwis_modules_task_common::task::Task;

use crate::task_manager::TaskManager;

#[derive(Clone, Debug)]
pub struct TaskState<T>
where
    T: TaskManager,
{
    pub manager: Arc<T>,
    pub task: Data<GuarantorSigned, Task>,
    pub created_date: DateTime,
    pub is_working: bool,
}
