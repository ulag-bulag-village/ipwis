use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use ipis::{
    core::{
        anyhow::{bail, Result},
        value::text::Text,
    },
    tokio::{self, sync::Mutex},
};

use crate::{task_manager::TaskManager, task_state::TaskState};

pub struct TaskInstance<R, T>
where
    T: TaskManager,
{
    pub state: Arc<Mutex<TaskState<T>>>,
    pub handler: tokio::task::JoinHandle<Result<R, Text>>,
}

impl<R, T> Future for TaskInstance<R, T>
where
    T: TaskManager,
{
    type Output = Result<R>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handler)
            .poll(cx)
            .map(|result| match result {
                Ok(Ok(outputs)) => Ok(outputs),
                Ok(Err(errors)) => bail!("{}", errors.msg),
                Err(error) => Err(error.into()),
            })
    }
}
